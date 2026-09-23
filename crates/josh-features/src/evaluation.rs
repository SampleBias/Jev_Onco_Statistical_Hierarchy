//! Cohort metrics over frozen predictions; labels never enter a provider request.
use josh_core::ValidationError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub sample_id: String,
    pub patient_group_id: String,
    pub partition: String,
    pub truth: String,
    pub probabilities: Option<BTreeMap<String, f64>>,
    pub accepted: bool,
    pub failure: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClassMetrics {
    pub count: usize,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReliabilityBin {
    pub count: usize,
    pub mean_confidence: Option<f64>,
    pub accuracy: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SelectivePoint {
    pub threshold: f64,
    pub retained: usize,
    pub coverage: f64,
    pub error: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Report {
    pub eligible: usize,
    pub predicted: usize,
    pub failed: usize,
    pub accepted: usize,
    pub coverage: f64,
    pub top1_all_eligible: f64,
    pub top3_all_eligible: f64,
    pub accepted_accuracy: Option<f64>,
    pub macro_f1: f64,
    pub weighted_f1: f64,
    pub brier: Option<f64>,
    pub log_loss: Option<f64>,
    pub ece: Option<f64>,
    pub per_class: BTreeMap<String, ClassMetrics>,
    pub confusion: BTreeMap<String, BTreeMap<String, usize>>,
    pub reliability: Vec<ReliabilityBin>,
    pub selective: Vec<SelectivePoint>,
    pub limitations: Vec<String>,
}
pub fn evaluate(
    records: &[Record],
    classes: &BTreeSet<String>,
    outcomes: &BTreeSet<String>,
    partition: &str,
) -> Result<Report, ValidationError> {
    if records.is_empty()
        || records.len() > 100_000
        || classes.len() < 2
        || !classes.is_subset(outcomes)
        || !matches!(partition, "development" | "calibration" | "test")
    {
        return Err(ValidationError(
            "invalid evaluation population or class set",
        ));
    }
    let mut patient_partition = BTreeMap::new();
    let mut samples = BTreeSet::new();
    for r in records {
        if r.sample_id.is_empty()
            || r.patient_group_id.is_empty()
            || !samples.insert(&r.sample_id)
            || !classes.contains(&r.truth)
            || !matches!(r.partition.as_str(), "development" | "calibration" | "test")
        {
            return Err(ValidationError(
                "invalid or duplicate labeled evaluation record",
            ));
        }
        if patient_partition
            .insert(&r.patient_group_id, &r.partition)
            .is_some_and(|p| p != &r.partition)
        {
            return Err(ValidationError(
                "patient group crosses evaluation partitions",
            ));
        }
        if r.probabilities.is_some() == r.failure.is_some()
            || (r.probabilities.is_none() && r.accepted)
        {
            return Err(ValidationError(
                "evaluation must retain explicit success/failure status",
            ));
        }
        if let Some(p) = &r.probabilities
            && (p.keys().cloned().collect::<BTreeSet<_>>() != *outcomes
                || p.values()
                    .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
                || (p.values().sum::<f64>() - 1.0).abs() > 1e-6)
        {
            return Err(ValidationError("invalid frozen probability vector"));
        }
    }
    let selected: Vec<_> = records
        .iter()
        .filter(|r| r.partition == partition)
        .collect();
    if selected.is_empty() {
        return Err(ValidationError("selected evaluation partition is empty"));
    }
    let n = selected.len();
    let mut predicted = 0;
    let mut accepted = 0;
    let mut accepted_correct = 0;
    let mut top1 = 0;
    let mut top3 = 0;
    let mut brier = 0.0;
    let mut loss = 0.0;
    let mut confusion: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut bins = vec![(0usize, 0.0, 0usize); 10];
    let mut scores = Vec::new();
    for r in selected {
        let row = confusion.entry(r.truth.clone()).or_default();
        if let Some(p) = &r.probabilities {
            predicted += 1;
            let mut order: Vec<_> = p.iter().collect();
            order.sort_by(|a, b| b.1.total_cmp(a.1).then(a.0.cmp(b.0)));
            let correct = order[0].0 == &r.truth;
            top1 += usize::from(correct);
            top3 += usize::from(order.iter().take(3).any(|(c, _)| *c == &r.truth));
            *row.entry(order[0].0.clone()).or_default() += 1;
            brier += p
                .iter()
                .map(|(c, p)| (p - f64::from(c == &r.truth)).powi(2))
                .sum::<f64>();
            loss -= p[&r.truth].max(1e-15).ln();
            let index = ((*order[0].1 * 10.0).floor() as usize).min(9);
            bins[index].0 += 1;
            bins[index].1 += order[0].1;
            bins[index].2 += usize::from(correct);
            scores.push((*order[0].1, correct, classes.contains(order[0].0)));
            if r.accepted {
                accepted += 1;
                accepted_correct += usize::from(correct);
            }
        } else {
            *row.entry("__failed__".into()).or_default() += 1;
        }
    }
    let mut per_class = BTreeMap::new();
    for c in classes {
        let count = confusion.get(c).map_or(0, |r| r.values().sum());
        let tp = confusion
            .get(c)
            .and_then(|r| r.get(c))
            .copied()
            .unwrap_or(0);
        let predicted_count: usize = confusion
            .values()
            .map(|r| r.get(c).copied().unwrap_or(0))
            .sum();
        let precision = if predicted_count > 0 {
            tp as f64 / predicted_count as f64
        } else {
            0.0
        };
        let recall = if count > 0 {
            tp as f64 / count as f64
        } else {
            0.0
        };
        let f1 = if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        };
        per_class.insert(
            c.clone(),
            ClassMetrics {
                count,
                precision,
                recall,
                f1,
            },
        );
    }
    let reliability: Vec<_> = bins
        .iter()
        .map(|(count, sum, correct)| ReliabilityBin {
            count: *count,
            mean_confidence: (*count > 0).then(|| sum / (*count as f64)),
            accuracy: (*count > 0).then(|| *correct as f64 / (*count as f64)),
        })
        .collect();
    let ece = (predicted > 0).then(|| {
        reliability
            .iter()
            .filter(|b| b.count > 0)
            .map(|b| {
                (b.mean_confidence.unwrap() - b.accuracy.unwrap()).abs() * b.count as f64
                    / predicted as f64
            })
            .sum()
    });
    let selective = [0.0, 0.5, 0.7, 0.8, 0.9, 0.95]
        .into_iter()
        .map(|threshold| {
            let rows: Vec<_> = scores
                .iter()
                .filter(|(p, _, biological)| *p >= threshold && *biological)
                .collect();
            let retained = rows.len();
            SelectivePoint {
                threshold,
                retained,
                coverage: retained as f64 / n as f64,
                error: (retained > 0).then(|| {
                    rows.iter().filter(|(_, correct, _)| !*correct).count() as f64 / retained as f64
                }),
            }
        })
        .collect();
    Ok(Report {eligible:n,predicted,failed:n-predicted,accepted,coverage:accepted as f64/n as f64,top1_all_eligible:top1 as f64/n as f64,top3_all_eligible:top3 as f64/n as f64,accepted_accuracy:(accepted>0).then(||accepted_correct as f64/accepted as f64),macro_f1:per_class.values().map(|r|r.f1).sum::<f64>()/classes.len() as f64,weighted_f1:per_class.values().map(|r|r.f1*r.count as f64).sum::<f64>()/n as f64,brier:(predicted>0).then(||brier/predicted as f64),log_loss:(predicted>0).then(||loss/predicted as f64),ece,per_class,confusion,reliability,selective,limitations:vec!["Brier, log loss and reliability are conditional on returned predictions; failures remain in eligible accuracy and coverage denominators.".into(),"Macro F1 includes declared classes with zero support; inspect class counts.".into(),"Log loss clips at 1e-15 for scoring only; stored probabilities are unchanged.".into(),"This report does not fit or authorize clinical calibration; no confidence intervals are estimated.".into()]})
}
