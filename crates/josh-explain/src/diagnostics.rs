//! Offline comparisons of completed explanations. Descriptive, not clinical uncertainty.
use crate::{Archive, Error};
use josh_core::{ValidationError, molecular::hash};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Serialize)]
pub struct Comparison {
    pub target_class: String,
    pub left_method: String,
    pub right_method: String,
    pub same_background: bool,
    pub full_probability_difference: f64,
    pub baseline_probability_difference: f64,
    pub attribution_rmse: f64,
    pub maximum_absolute_difference: f64,
    pub nonzero_groups: usize,
    pub sign_agreement: Option<f64>,
    pub magnitude_rank_correlation: Option<f64>,
    pub top_k: usize,
    pub top_k_overlap: f64,
    pub limitations: Vec<String>,
}

fn ranks(values: &[f64]) -> Vec<f64> {
    let mut order: Vec<_> = (0..values.len()).collect();
    order.sort_by(|a, b| values[*a].total_cmp(&values[*b]));
    let mut ranks = vec![0.0; values.len()];
    let mut start = 0;
    while start < order.len() {
        let mut end = start + 1;
        while end < order.len() && values[order[start]] == values[order[end]] {
            end += 1;
        }
        for i in &order[start..end] {
            ranks[*i] = (start + end - 1) as f64 / 2.0;
        }
        start = end;
    }
    ranks
}

fn correlation(a: &[f64], b: &[f64]) -> Option<f64> {
    let mean_a = a.iter().sum::<f64>() / a.len() as f64;
    let mean_b = b.iter().sum::<f64>() / b.len() as f64;
    let mut cov = 0.0;
    let mut va = 0.0;
    let mut vb = 0.0;
    for (a, b) in a.iter().zip(b) {
        cov += (a - mean_a) * (b - mean_b);
        va += (a - mean_a).powi(2);
        vb += (b - mean_b).powi(2);
    }
    (va > 0.0 && vb > 0.0).then(|| (cov / (va * vb).sqrt()).clamp(-1.0, 1.0))
}

pub fn compare(left: &Archive, right: &Archive, top_k: usize) -> Result<Comparison, Error> {
    left.validate()?;
    right.validate()?;
    let (Some(a), Some(b)) = (&left.result, &right.result) else {
        return Err(ValidationError("stability requires two complete explanations").into());
    };
    if top_k == 0
        || hash(&left.features)? != hash(&right.features)?
        || hash(&left.inference.request)? != hash(&right.inference.request)?
        || left.inference.source != right.inference.source
        || a.target_class != b.target_class
    {
        return Err(ValidationError(
            "stability requires the same sample, source, frozen request and target",
        )
        .into());
    }
    let aa: BTreeMap<_, _> = a
        .attributions
        .iter()
        .map(|v| (&v.group, v.contribution))
        .collect();
    let bb: BTreeMap<_, _> = b
        .attributions
        .iter()
        .map(|v| (&v.group, v.contribution))
        .collect();
    if aa.keys().ne(bb.keys()) || aa.is_empty() {
        return Err(ValidationError("stability attribution groups differ").into());
    }
    let mut squares = 0.0;
    let mut maximum = 0.0_f64;
    let mut nonzero = 0;
    let mut same_sign = 0;
    for (key, value) in &aa {
        let other = bb[key];
        squares += (value - other).powi(2);
        maximum = maximum.max((value - other).abs());
        if value.abs() > 1e-10 || other.abs() > 1e-10 {
            nonzero += 1;
            same_sign += usize::from(
                value.abs() > 1e-10 && other.abs() > 1e-10 && value.signum() == other.signum(),
            );
        }
    }
    let rank_a = ranks(&aa.values().map(|v| v.abs()).collect::<Vec<_>>());
    let rank_b = ranks(&bb.values().map(|v| v.abs()).collect::<Vec<_>>());
    let top_k = top_k.min(aa.len());
    let top = |values: &BTreeMap<&String, f64>| {
        let mut rows: Vec<_> = values.iter().collect();
        rows.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()).then(a.0.cmp(b.0)));
        rows.into_iter()
            .take(top_k)
            .map(|(k, _)| (*k).clone())
            .collect::<BTreeSet<_>>()
    };
    Ok(Comparison {
        target_class: a.target_class.clone(), left_method: a.method.clone(), right_method: b.method.clone(),
        same_background: hash(&left.background)? == hash(&right.background)?,
        full_probability_difference: b.full_probability - a.full_probability,
        baseline_probability_difference: b.baseline_probability - a.baseline_probability,
        attribution_rmse: (squares / aa.len() as f64).sqrt(), maximum_absolute_difference: maximum,
        nonzero_groups: nonzero, sign_agreement: (nonzero > 0).then(|| same_sign as f64 / nonzero as f64),
        magnitude_rank_correlation: correlation(&rank_a, &rank_b), top_k,
        top_k_overlap: top(&aa).intersection(&top(&bb)).count() as f64 / top_k as f64,
        limitations: vec!["Differences can reflect baseline choice, permutation sampling or hosted model variability; this comparison cannot isolate their causes.".into(), "Rank correlation uses mean ranks for ties; top-k ties use group ID order. Sign comparison uses a 1e-10 probability tolerance.".into(), "These descriptive statistics are not clinical confidence intervals.".into()],
    })
}
