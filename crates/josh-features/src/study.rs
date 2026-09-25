//! Frozen cohort studies: classification and outcomes stay separate; no provider calls.
use crate::{
    evaluation::{self, Record},
    survival,
};
use josh_core::{
    DataClass, ValidationError,
    molecular::{TaxonomyDefinition, hash},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const KIND: &str = "josh_cohort_study";
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Study {
    pub kind: String,
    pub schema_version: u32,
    pub study_id: String,
    pub title: String,
    pub data_class: DataClass,
    pub protocol: String,
    pub prediction_source: String,
    pub model: String,
    pub partition: String,
    pub taxonomy: TaxonomyDefinition,
    pub records: Vec<Record>,
    /// A complete, explicit leaf -> group map; empty means no broad-group analysis.
    pub broad_groups: BTreeMap<String, String>,
    pub broad_group_version: Option<String>,
    pub outcomes: Option<survival::Outcomes>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threshold {
    pub threshold: f64,
    pub retained: usize,
    pub coverage: f64,
    pub accuracy: Option<f64>,
    pub weighted_f1: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interval {
    pub estimate: f64,
    pub lower: f64,
    pub upper: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bootstrap {
    pub replicates: usize,
    pub seed: u64,
    pub patients: usize,
    pub top1: Interval,
    pub weighted_f1: Interval,
    pub method: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub study_sha256: String,
    pub evaluation: Option<evaluation::Report>,
    pub broad_evaluation: Option<evaluation::Report>,
    pub thresholds: Vec<Threshold>,
    pub bootstrap: Option<Bootstrap>,
    pub bootstrap_unavailable: Option<String>,
    pub survival: Option<survival::Report>,
    pub limitations: Vec<String>,
}
pub fn winner(record: &Record) -> Option<(&str, f64)> {
    record
        .probabilities
        .as_ref()?
        .iter()
        .min_by(|(a, p), (b, q)| q.total_cmp(p).then(a.cmp(b)))
        .map(|(k, p)| (k.as_str(), *p))
}
fn text(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 512 && !s.chars().any(char::is_control)
}
pub fn validate(study: &Study) -> Result<(), ValidationError> {
    if study.kind != KIND
        || study.schema_version != 1
        || ![
            &study.study_id,
            &study.title,
            &study.protocol,
            &study.prediction_source,
            &study.model,
        ]
        .iter()
        .all(|s| text(s))
        || !matches!(
            study.partition.as_str(),
            "development" | "calibration" | "test"
        )
        || (study.records.is_empty() && study.outcomes.is_none())
    {
        return Err(ValidationError(
            "invalid cohort study metadata, partition or empty population",
        ));
    }
    study.taxonomy.validate()?;
    let classes: BTreeSet<_> = study
        .taxonomy
        .classes
        .iter()
        .map(|c| c.id.clone())
        .collect();
    if classes.len() > 32 || study.taxonomy.criteria().contains_key("__failed__") {
        return Err(ValidationError(
            "cohort dashboard supports at most 32 cancer classes; __failed__ is reserved",
        ));
    }
    if !study.broad_groups.is_empty() {
        if study.broad_groups.keys().cloned().collect::<BTreeSet<_>>() != classes
            || study.broad_groups.values().any(|g| {
                !text(g)
                    || g == &study.taxonomy.unknown_id
                    || g == &study.taxonomy.other_id
                    || g == "__failed__"
            })
            || !study.broad_group_version.as_ref().is_some_and(|s| text(s))
            || study.broad_groups.values().collect::<BTreeSet<_>>().len() < 2
        {
            return Err(ValidationError(
                "broad groups require a named version and complete, nonreserved leaf mapping to at least two groups",
            ));
        }
    } else if study.broad_group_version.is_some() {
        return Err(ValidationError(
            "broad group version supplied without mapping",
        ));
    }
    if !study.records.is_empty() {
        evaluation::evaluate(
            &study.records,
            &classes,
            &study.taxonomy.criteria().keys().cloned().collect(),
            &study.partition,
        )?;
    }
    if let Some(outcomes) = &study.outcomes {
        survival::validate(outcomes)?;
        if outcomes
            .observations
            .iter()
            .any(|r| !study.taxonomy.criteria().contains_key(&r.predicted_class))
        {
            return Err(ValidationError(
                "survival predicted class is outside the frozen taxonomy",
            ));
        }
    }
    Ok(())
}
pub fn mapped_records(study: &Study) -> Vec<Record> {
    study
        .records
        .iter()
        .map(|r| {
            let mut mapped = r.clone();
            mapped.truth = study.broad_groups[&r.truth].clone();
            mapped.probabilities = r.probabilities.as_ref().map(|p| {
                let mut groups = BTreeMap::new();
                for (class, value) in p {
                    *groups
                        .entry(study.broad_groups.get(class).unwrap_or(class).clone())
                        .or_default() += value;
                }
                groups
            });
            mapped
        })
        .collect()
}
pub fn analyze(
    study: &Study,
    bootstrap_replicates: usize,
    seed: u64,
) -> Result<Report, ValidationError> {
    validate(study)?;
    let classes: BTreeSet<_> = study
        .taxonomy
        .classes
        .iter()
        .map(|c| c.id.clone())
        .collect();
    let outcomes = study.taxonomy.criteria().keys().cloned().collect();
    let selected: Vec<_> = study
        .records
        .iter()
        .filter(|r| r.partition == study.partition)
        .collect();
    let evaluation = if selected.is_empty() {
        None
    } else {
        Some(evaluation::evaluate(
            &study.records,
            &classes,
            &outcomes,
            &study.partition,
        )?)
    };
    let broad_evaluation = if study.broad_groups.is_empty() || selected.is_empty() {
        None
    } else {
        let broad = study
            .broad_groups
            .values()
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut options = broad.clone();
        options.insert(study.taxonomy.unknown_id.clone());
        options.insert(study.taxonomy.other_id.clone());
        Some(evaluation::evaluate(
            &mapped_records(study),
            &broad,
            &options,
            &study.partition,
        )?)
    };
    let mut thresholds = Vec::new();
    for threshold in [0.0, 0.5, 0.7, 0.8, 0.9, 0.95]
        .into_iter()
        .filter(|_| !selected.is_empty())
    {
        let retained: Vec<_> = selected
            .iter()
            .filter(|r| winner(r).is_some_and(|(c, p)| classes.contains(c) && p >= threshold))
            .map(|r| (*r).clone())
            .collect();
        let metrics = if retained.is_empty() {
            None
        } else {
            Some(evaluation::evaluate(
                &retained,
                &classes,
                &outcomes,
                &study.partition,
            )?)
        };
        thresholds.push(Threshold {
            threshold,
            retained: retained.len(),
            coverage: if selected.is_empty() {
                0.0
            } else {
                retained.len() as f64 / selected.len() as f64
            },
            accuracy: metrics.as_ref().map(|m| m.top1_all_eligible),
            weighted_f1: metrics.map(|m| m.weighted_f1),
        });
    }
    let boot = bootstrap(&selected, &classes, bootstrap_replicates, seed);
    let mut limitations=vec![
        "Imported frozen predictions are not authenticated provider responses. The study records their declared source, model and protocol; a study hash identifies these exact inputs.".into(),
        "Failures remain in all-eligible accuracy and coverage. Unknown/other scores are retained without renormalization; thresholds select only biological winners.".into(),
        "Cohort association, confidence intervals and software tests do not establish cancer calibration or treatment benefit.".into(),
    ];
    if study.data_class == DataClass::Synthetic {
        limitations.insert(0,"SYNTHETIC DEMONSTRATION: all measurements and outcomes are invented; these are not Jev or OncoNPC performance results.".into());
    }
    if study.partition != "test" {
        limitations.push(
            "This is not the held-out test partition; do not report it as final test performance."
                .into(),
        );
    }
    Ok(Report {
        study_sha256: hash(study)?,
        evaluation,
        broad_evaluation,
        thresholds,
        bootstrap_unavailable: boot.as_ref().err().map(ToString::to_string),
        bootstrap: boot.ok(),
        survival: study.outcomes.as_ref().map(survival::analyze).transpose()?,
        limitations,
    })
}

/// Patient-cluster percentile bootstrap. Repeated samples from a patient travel together.
pub fn bootstrap(
    rows: &[&Record],
    classes: &BTreeSet<String>,
    replicates: usize,
    seed: u64,
) -> Result<Bootstrap, ValidationError> {
    if !(100..=1000).contains(&replicates) || rows.len().saturating_mul(replicates) > 10_000_000 {
        return Err(ValidationError(
            "bootstrap needs 100–1,000 replicates and at most 10 million resampled records; zero disables it",
        ));
    }
    let ids: Vec<_> = classes.iter().collect();
    let mut grouped: BTreeMap<&str, Vec<(usize, Option<usize>)>> = BTreeMap::new();
    for r in rows {
        let truth = ids
            .iter()
            .position(|s| *s == &r.truth)
            .ok_or(ValidationError("bootstrap truth outside class set"))?;
        let prediction = winner(r).and_then(|(c, _)| ids.iter().position(|s| s.as_str() == c));
        grouped
            .entry(&r.patient_group_id)
            .or_default()
            .push((truth, prediction));
    }
    if grouped.len() < 2 {
        return Err(ValidationError(
            "bootstrap needs at least two independent patient groups",
        ));
    }
    let groups: Vec<_> = grouped.values().collect();
    let score = |indexes: &[usize]| {
        let mut support = vec![0usize; ids.len()];
        let mut predicted = vec![0usize; ids.len()];
        let mut tp = vec![0usize; ids.len()];
        let mut n = 0usize;
        for &g in indexes {
            for &(truth, prediction) in groups[g] {
                n += 1;
                support[truth] += 1;
                if let Some(p) = prediction {
                    predicted[p] += 1;
                    if p == truth {
                        tp[truth] += 1;
                    }
                }
            }
        }
        let accuracy = tp.iter().sum::<usize>() as f64 / n as f64;
        let weighted = (0..ids.len())
            .map(|j| {
                if support[j] + predicted[j] > 0 {
                    support[j] as f64 * 2.0 * tp[j] as f64 / (support[j] + predicted[j]) as f64
                } else {
                    0.0
                }
            })
            .sum::<f64>()
            / n as f64;
        (accuracy, weighted)
    };
    let estimate = score(&(0..groups.len()).collect::<Vec<_>>());
    let mut rng = Random(seed);
    let mut accuracy = Vec::new();
    let mut f1 = Vec::new();
    for _ in 0..replicates {
        let indexes: Vec<_> = (0..groups.len()).map(|_| rng.index(groups.len())).collect();
        let (a, f) = score(&indexes);
        accuracy.push(a);
        f1.push(f);
    }
    let interval = |estimate, mut values: Vec<f64>| {
        values.sort_by(f64::total_cmp);
        let quantile = |p: f64| {
            let x = (values.len() - 1) as f64 * p;
            let lo = x.floor() as usize;
            let hi = x.ceil() as usize;
            values[lo] + (values[hi] - values[lo]) * (x - lo as f64)
        };
        Interval {
            estimate,
            lower: quantile(0.025),
            upper: quantile(0.975),
        }
    };
    Ok(Bootstrap{replicates,seed,patients:groups.len(),top1:interval(estimate.0,accuracy),weighted_f1:interval(estimate.1,f1),method:"95% patient-cluster percentile intervals; fixed class set; linear-interpolated quantiles; SplitMix64".into()})
}
pub struct Random(pub u64);
impl Random {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    pub fn index(&mut self, n: usize) -> usize {
        let bound = u64::MAX - u64::MAX % n as u64;
        loop {
            let v = self.next_u64();
            if v < bound {
                return (v % n as u64) as usize;
            }
        }
    }
    pub fn unit(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 0.5) / (1_u64 << 53) as f64
    }
}
