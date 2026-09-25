//! Offline cohort files, reproducible demonstration and exports.
use crate::workflows::{self, AppError};
use clap::{Subcommand, ValueEnum};
use josh_core::{
    DataClass,
    molecular::{hash, onconpc_taxonomy},
};
use josh_features::{
    evaluation::Record,
    study::{self, Random, Report, Study},
    survival::{Concordance, Observation, Outcomes},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, ValueEnum)]
pub enum ExportKind {
    Svg,
    Markdown,
    Json,
}
#[derive(Subcommand)]
pub enum Command {
    /// Print an invented 22-class cohort with survival outcomes; never calls Jev.
    Demo,
    /// Wrap frozen evaluation records in an explicit study protocol. No inference.
    FromRecords {
        records: PathBuf,
        #[arg(long)]
        taxonomy: Option<PathBuf>,
        #[arg(long)]
        study_id: String,
        #[arg(long)]
        protocol: String,
        #[arg(long)]
        prediction_source: String,
        #[arg(long)]
        model: String,
        #[arg(long, default_value = "test")]
        partition: String,
        #[arg(long)]
        synthetic: bool,
    },
    /// Evaluate a study locally, with patient-cluster confidence intervals.
    Analyze {
        input: PathBuf,
        #[arg(long, default_value_t = 200)]
        bootstrap: usize,
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },
    /// Export a paper-style SVG, Markdown report or reopenable JSON archive.
    Export {
        input: PathBuf,
        #[arg(long, value_enum, default_value = "svg")]
        kind: ExportKind,
        #[arg(long, default_value_t = 200)]
        bootstrap: usize,
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Archive {
    pub kind: String,
    pub study: Study,
    pub report: Report,
}
pub fn load(path: &Path) -> Result<Study, AppError> {
    let value: serde_json::Value = workflows::read_json(path, 64 * 1024 * 1024)?;
    let data = if value.get("kind").and_then(|v| v.as_str()) == Some("josh_cohort_report") {
        let archive: Archive = serde_json::from_value(value)?;
        if hash(&archive.study)? != archive.report.study_sha256 {
            return Err("Cohort archive input hash does not match.".into());
        }
        archive.study
    } else {
        serde_json::from_value(value)?
    };
    study::validate(&data)?;
    Ok(data)
}
pub fn archive(study: &Study, report: &Report) -> Result<String, AppError> {
    workflows::pretty(&Archive {
        kind: "josh_cohort_report".into(),
        study: study.clone(),
        report: report.clone(),
    })
}
pub fn export(study: &Study, report: &Report, kind: ExportKind) -> Result<String, AppError> {
    match kind {
        ExportKind::Svg => Ok(crate::study_charts::svg(study, report)),
        ExportKind::Markdown => Ok(markdown(study, report)),
        ExportKind::Json => archive(study, report),
    }
}
pub fn execute(command: Command) -> Result<crate::molecular::Output, AppError> {
    let (data, options, kind) = match command {
        Command::Demo => (demo(), None, None),
        Command::FromRecords {
            records,
            taxonomy,
            study_id,
            protocol,
            prediction_source,
            model,
            partition,
            synthetic,
        } => {
            let data = Study {
                kind: study::KIND.into(),
                schema_version: 1,
                title: study_id.clone(),
                study_id,
                data_class: if synthetic {
                    DataClass::Synthetic
                } else {
                    DataClass::DeidentifiedResearch
                },
                protocol,
                prediction_source,
                model,
                partition,
                taxonomy: crate::molecular::load_taxonomy(taxonomy.as_deref())?,
                records: workflows::read_json(&records, 64 * 1024 * 1024)?,
                broad_groups: BTreeMap::new(),
                broad_group_version: None,
                outcomes: None,
            };
            (data, None, None)
        }
        Command::Analyze {
            input,
            bootstrap,
            seed,
        } => (load(&input)?, Some((bootstrap, seed)), None),
        Command::Export {
            input,
            kind,
            bootstrap,
            seed,
        } => (load(&input)?, Some((bootstrap, seed)), Some(kind)),
    };
    study::validate(&data)?;
    if let Some((reps, seed)) = options {
        let report = study::analyze(&data, reps, seed)?;
        let text = markdown(&data, &report);
        let raw = kind.map(|k| export(&data, &report, k)).transpose()?;
        Ok(crate::molecular::Output {
            value: serde_json::to_value(Archive {
                kind: "josh_cohort_report".into(),
                study: data,
                report,
            })?,
            text,
            raw,
        })
    } else {
        Ok(crate::molecular::Output {
            value: serde_json::to_value(&data)?,
            text: workflows::pretty(&data)?,
            raw: None,
        })
    }
}

pub fn demo() -> Study {
    let taxonomy = onconpc_taxonomy();
    let keys: Vec<_> = taxonomy.criteria().keys().cloned().collect();
    let mut rng = Random(0x4a4f5348);
    let mut records = Vec::new();
    for i in 0..528 {
        let class = &taxonomy.classes[i % 22].id;
        let mut prediction = class.clone();
        let score = 0.34 + rng.unit() * 0.65;
        if rng.unit() > 0.30 + score * 0.65 {
            prediction = taxonomy.classes[rng.index(22)].id.clone();
        }
        if i % 37 == 0 {
            prediction = taxonomy.unknown_id.clone();
        }
        if i % 83 == 0 {
            prediction = taxonomy.other_id.clone();
        }
        let failed = i % 61 == 0;
        let probabilities = (!failed).then(|| {
            keys.iter()
                .map(|k| {
                    (
                        k.clone(),
                        if k == &prediction {
                            score
                        } else {
                            (1.0 - score) / (keys.len() - 1) as f64
                        },
                    )
                })
                .collect()
        });
        records.push(Record {
            sample_id: format!("DEMO-S{i:04}"),
            patient_group_id: format!("DEMO-P{i:04}"),
            partition: "test".into(),
            truth: class.clone(),
            probabilities,
            accepted: !failed
                && score >= 0.75
                && prediction != taxonomy.unknown_id
                && prediction != taxonomy.other_id,
            failure: failed.then(|| "invented unavailable prediction".into()),
        });
    }
    let mut observations = Vec::new();
    let classes = ["BRCA", "NSCLC", "COADREAD", "PAAD", "OVT"];
    for i in 0..180 {
        let age = 35. + rng.unit() * 45.;
        let status = rng.index(3) as f64;
        let propensity = 1. / (1. + (-0.4 + 0.015 * (age - 60.) + 0.25 * status).exp());
        let concordant = rng.unit() < propensity;
        let group = i % classes.len();
        let rate = [0.018, 0.048, 0.028, 0.065, 0.034][group]
            * (0.014 * (age - 60.) + 0.22 * status).exp()
            * if concordant { 0.65 } else { 1.0 };
        let entry = if i % 7 == 0 { rng.unit() * 3.0 } else { 0.0 };
        let event_time = entry - rng.unit().ln() / rate;
        let censor_time = entry + 12. + rng.unit() * 48.;
        observations.push(Observation {
            patient_id: format!("DEMO-OUTCOME-{i:04}"),
            predicted_class: classes[group].into(),
            entry,
            time: event_time.min(censor_time),
            event: event_time <= censor_time,
            concordance: if i % 29 == 0 {
                Concordance::Empiric
            } else if i % 31 == 0 {
                Concordance::Unreviewed
            } else if concordant {
                Concordance::Concordant
            } else {
                Concordance::Discordant
            },
            propensity: Some(propensity),
            covariates: BTreeMap::from([
                ("age_years".into(), age),
                ("performance_status".into(), status),
            ]),
        });
    }
    Study{kind:study::KIND.into(),schema_version:1,study_id:"synthetic-cohort-demo-v1".into(),title:"Cohort observatory".into(),data_class:DataClass::Synthetic,protocol:"Invented classification and outcome cohorts for graphics and numerical verification; no clinical claims.".into(),prediction_source:"Deterministic synthetic generator; no provider requests or paper patient data.".into(),model:"analytical-demo (not Jev)".into(),partition:"test".into(),taxonomy,records,broad_groups:BTreeMap::new(),broad_group_version:None,outcomes:Some(Outcomes{endpoint:"Overall survival (invented)".into(),time_origin:"First treatment (invented)".into(),time_unit:"months".into(),source:"Invented independent patients, events and censoring".into(),adjustment_covariates:vec!["age_years".into(),"performance_status".into()],propensity_model:Some("Known synthetic assignment probability; not a fitted patient model".into()),observations})}
}
fn md(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('|', "\\|")
        .replace('[', "\\[")
        .replace(']', "\\]")
}
pub fn markdown(data: &Study, report: &Report) -> String {
    let mut text = format!(
        "# {}\n\n{}\n\nStudy: {} · Partition: {} · Model: {}\n\nProtocol: {}\n\nPrediction source: {}\n\nInput SHA-256: `{}`\n\n",
        md(&data.title),
        if data.data_class == DataClass::Synthetic {
            "**SYNTHETIC DEMONSTRATION — invented data; not cancer performance.**"
        } else {
            "Offline analysis of imported frozen predictions; no new provider requests."
        },
        md(&data.study_id),
        md(&data.partition),
        md(&data.model),
        md(&data.protocol),
        md(&data.prediction_source),
        report.study_sha256
    );
    if let Some(m) = &report.evaluation {
        text.push_str(&format!("## Classification\n\nEligible: {} · Returned: {} · Failed: {}\n\nTop-1: {:.4} · Top-3: {:.4} · Weighted F1: {:.4} · Macro F1: {:.4}\n\n",m.eligible,m.predicted,m.failed,m.top1_all_eligible,m.top3_all_eligible,m.weighted_f1,m.macro_f1));
        if let Some(b) = &report.bootstrap {
            text.push_str(&format!("95% patient-bootstrap intervals ({} replicates, seed {}): top-1 [{:.4}, {:.4}], weighted F1 [{:.4}, {:.4}].\n\n",b.replicates,b.seed,b.top1.lower,b.top1.upper,b.weighted_f1.lower,b.weighted_f1.upper));
        } else if let Some(reason) = &report.bootstrap_unavailable {
            text.push_str(&format!("Bootstrap: {}.\n\n", md(reason)));
        }
        text.push_str(
            "| Class | Count | Precision | Recall | F1 |\n| --- | ---: | ---: | ---: | ---: |\n",
        );
        for (class, c) in &m.per_class {
            text.push_str(&format!(
                "| {} | {} | {:.4} | {:.4} | {:.4} |\n",
                md(class),
                c.count,
                c.precision,
                c.recall,
                c.f1
            ));
        }
        text.push_str("\n### Confidence and coverage\n\n| Threshold | Retained | Coverage | Accuracy | Weighted F1 |\n| --- | ---: | ---: | ---: | ---: |\n");
        let number = |v: Option<f64>| v.map(|v| format!("{v:.4}")).unwrap_or_else(|| "N/A".into());
        for t in &report.thresholds {
            text.push_str(&format!(
                "| {:.2} | {} | {:.4} | {} | {} |\n",
                t.threshold,
                t.retained,
                t.coverage,
                number(t.accuracy),
                number(t.weighted_f1)
            ));
        }
        text.push_str(&format!("\nBrier: {} · Log loss: {} · ECE (10 bins): {}\n\nThese scoring diagnostics do not fit or establish calibration.\n\n",number(m.brier),number(m.log_loss),number(m.ece)));
    }
    if let (Some(s), Some(input)) = (&report.survival, &data.outcomes) {
        text.push_str(&format!("## Outcomes\n\nEndpoint: {} · Origin: {} · Units: {}\n\nPatients: {} · Events: {} · Delayed entries: {} · Excluded from concordance: {}\n\n| Predicted class | Patients | Events | Median survival |\n| --- | ---: | ---: | ---: |\n",md(&input.endpoint),md(&input.time_origin),md(&input.time_unit),s.patients,s.events,s.delayed_entries,s.excluded_from_concordance));
        for c in &s.by_class {
            text.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                md(&c.label),
                c.patients,
                c.events,
                c.median
                    .map(|m| format!("{m:.2}"))
                    .unwrap_or_else(|| "Not reached".into())
            ));
        }
        if let Some(lr) = &s.subtype_log_rank {
            text.push_str(&format!(
                "\nGlobal predicted-type log-rank: chi-square {:.6}, df {}, p {:.6e}.\n",
                lr.chi_squared, lr.degrees_of_freedom, lr.p_value
            ));
        } else if let Some(reason) = &s.subtype_log_rank_unavailable {
            text.push_str(&format!("\n{}\n", md(reason)));
        }
        if let Some(lr) = &s.log_rank {
            text.push_str(&format!(
                "\nUnweighted concordance log-rank: chi-square {:.6}, df 1, p {:.6e}.\n",
                lr.chi_squared, lr.p_value
            ));
        }
        if let Some(reason) = &s.log_rank_unavailable {
            text.push_str(&format!("\n{}\n", md(reason)));
        }
        if let Some(cox) = &s.cox {
            text.push_str(&format!("\n### Adjusted treatment association\n\n{}\n\n| Term | HR | 95% interval | p |\n| --- | ---: | --- | ---: |\n",md(&cox.method)));
            for c in &cox.coefficients {
                text.push_str(&format!(
                    "| {} | {:.4} | {:.4}–{:.4} | {:.6e} |\n",
                    md(&c.name),
                    c.hazard_ratio,
                    c.lower,
                    c.upper,
                    c.p_value
                ));
            }
        } else if let Some(reason) = &s.cox_unavailable {
            text.push_str(&format!("\n{}\n", md(reason)));
        }
        if let Some(w) = &s.weights {
            text.push_str(&format!("\nIPTW propensity range {:.3}–{:.3}; maximum weight {:.3}; effective N {:.1} concordant / {:.1} discordant. Weighted confidence intervals and weighted log-rank are unavailable.\n\n| Covariate | Unweighted SMD | Weighted SMD |\n| --- | ---: | ---: |\n",w.min_propensity,w.max_propensity,w.max_weight,w.concordant_effective_n,w.discordant_effective_n));
            for b in &w.balance {
                text.push_str(&format!(
                    "| {} | {:?} | {:?} |\n",
                    md(&b.covariate),
                    b.unweighted_smd,
                    b.weighted_smd
                ));
            }
        } else if let Some(reason) = &s.weighted_unavailable {
            text.push_str(&format!("\n{}\n", md(reason)));
        }
        for limit in &s.limitations {
            text.push_str(&format!("\n- {}\n", md(limit)));
        }
    }
    text.push_str("\n## Interpretation and references\n\n");
    for limit in &report.limitations {
        text.push_str(&format!("- {}\n", md(limit)));
    }
    text.push_str("\nResearch reference: Moon et al., OncoNPC. [Preprint](https://doi.org/10.1101/2022.12.22.22283696), [published study](https://doi.org/10.1038/s41591-023-02482-6), [correction](https://doi.org/10.1038/s41591-023-02693-x). Published results do not validate this cohort or Jev.\n");
    text
}
