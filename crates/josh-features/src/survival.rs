//! Offline single-event survival analysis. Times use (entry, exit] risk intervals.
//! One independent patient per row; no competing risks or automatic imputation.
use josh_core::ValidationError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Concordance {
    Concordant,
    Discordant,
    Empiric,
    Unreviewed,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub patient_id: String,
    pub predicted_class: String,
    pub entry: f64,
    pub time: f64,
    pub event: bool,
    pub concordance: Concordance,
    /// P(concordant | baseline covariates), supplied by a documented external model.
    pub propensity: Option<f64>,
    pub covariates: BTreeMap<String, f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Outcomes {
    pub endpoint: String,
    pub time_origin: String,
    pub time_unit: String,
    pub source: String,
    /// Frozen numeric baseline covariates; categorical variables must be explicitly encoded.
    pub adjustment_covariates: Vec<String>,
    /// Required when propensity scores are supplied. No score is fitted or clipped silently.
    pub propensity_model: Option<String>,
    pub observations: Vec<Observation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub time: f64,
    pub at_risk: usize,
    pub events: usize,
    pub censored: usize,
    pub weighted_at_risk: f64,
    pub survival: f64,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Curve {
    pub label: String,
    pub patients: usize,
    pub events: usize,
    pub start: f64,
    pub end: f64,
    pub median: Option<f64>,
    pub weighted: bool,
    pub points: Vec<Point>,
    pub risk_table: Vec<(f64, usize)>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRank {
    pub chi_squared: f64,
    pub p_value: f64,
    pub observed_minus_expected: f64,
    pub variance: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupLogRank {
    pub groups: Vec<String>,
    pub degrees_of_freedom: usize,
    pub chi_squared: f64,
    pub p_value: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coefficient {
    pub name: String,
    pub log_hazard_ratio: f64,
    pub standard_error: f64,
    pub hazard_ratio: f64,
    pub lower: f64,
    pub upper: f64,
    pub p_value: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoxFit {
    pub patients: usize,
    pub events: usize,
    pub iterations: usize,
    pub log_likelihood: f64,
    pub coefficients: Vec<Coefficient>,
    pub method: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightDiagnostics {
    pub min_propensity: f64,
    pub max_propensity: f64,
    pub max_weight: f64,
    pub concordant_effective_n: f64,
    pub discordant_effective_n: f64,
    pub balance: Vec<Balance>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub covariate: String,
    pub unweighted_smd: Option<f64>,
    pub weighted_smd: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub patients: usize,
    pub events: usize,
    pub delayed_entries: usize,
    pub excluded_from_concordance: usize,
    pub by_class: Vec<Curve>,
    pub subtype_log_rank: Option<GroupLogRank>,
    pub subtype_log_rank_unavailable: Option<String>,
    pub by_concordance: Vec<Curve>,
    pub weighted_concordance: Vec<Curve>,
    pub log_rank: Option<LogRank>,
    pub log_rank_unavailable: Option<String>,
    pub cox: Option<CoxFit>,
    pub cox_unavailable: Option<String>,
    pub weights: Option<WeightDiagnostics>,
    pub weighted_unavailable: Option<String>,
    pub limitations: Vec<String>,
}

fn label(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 256 && !s.chars().any(char::is_control)
}
pub fn validate(data: &Outcomes) -> Result<(), ValidationError> {
    if !label(&data.endpoint)
        || !label(&data.time_origin)
        || !label(&data.source)
        || !matches!(data.time_unit.as_str(), "months" | "days" | "years")
        || data.observations.is_empty()
        || data.observations.len() > 20_000
        || data.adjustment_covariates.len() > 8
        || data
            .adjustment_covariates
            .iter()
            .any(|s| !label(s) || s == "concordant")
        || data
            .adjustment_covariates
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != data.adjustment_covariates.len()
        || data.propensity_model.as_ref().is_some_and(|s| !label(s))
    {
        return Err(ValidationError(
            "invalid survival endpoint, covariates or population (maximum 20,000 patients)",
        ));
    }
    let mut patients = BTreeSet::new();
    for r in &data.observations {
        if !label(&r.patient_id)
            || !patients.insert(&r.patient_id)
            || !label(&r.predicted_class)
            || !r.entry.is_finite()
            || !r.time.is_finite()
            || r.entry < 0.0
            || r.time <= r.entry
            || r.covariates.len() > 8
            || r.covariates
                .iter()
                .any(|(k, v)| !label(k) || !v.is_finite())
            || r.propensity
                .is_some_and(|p| !p.is_finite() || p <= 0.0 || p >= 1.0)
            || (r.propensity.is_some() && data.propensity_model.is_none())
        {
            return Err(ValidationError(
                "invalid survival row: require unique patient, finite entry < exit, and documented propensity in (0,1)",
            ));
        }
    }
    Ok(())
}

/// Product-limit estimate. Fractional IPTW weights never use naive Greenwood intervals.
pub fn kaplan_meier(
    label: &str,
    rows: &[&Observation],
    weights: Option<&[f64]>,
    ticks: &[f64],
) -> Result<Curve, ValidationError> {
    if rows.is_empty()
        || weights
            .is_some_and(|w| w.len() != rows.len() || w.iter().any(|v| !v.is_finite() || *v <= 0.0))
        || rows.iter().any(|r| {
            !r.entry.is_finite() || !r.time.is_finite() || r.entry < 0.0 || r.time <= r.entry
        })
    {
        return Err(ValidationError(
            "invalid Kaplan-Meier observations or weights",
        ));
    }
    let weight = |i: usize| weights.map_or(1.0, |w| w[i]);
    let mut exits: Vec<_> = (0..rows.len()).collect();
    exits.sort_by(|a, b| rows[*a].time.total_cmp(&rows[*b].time));
    let mut entries = exits.clone();
    entries.sort_by(|a, b| rows[*a].entry.total_cmp(&rows[*b].entry));
    let (mut entered, mut index, mut risk, mut weighted_risk) = (0, 0, 0usize, 0.0);
    let (mut survival, mut greenwood) = (1.0_f64, 0.0_f64);
    let mut points = Vec::new();
    while index < exits.len() {
        let time = rows[exits[index]].time;
        while entered < entries.len() && rows[entries[entered]].entry < time {
            risk += 1;
            weighted_risk += weight(entries[entered]);
            entered += 1;
        }
        let start = index;
        let (mut deaths, mut censored, mut weighted_deaths, mut removed) = (0, 0, 0.0, 0.0);
        while index < exits.len() && rows[exits[index]].time == time {
            let i = exits[index];
            if rows[i].event {
                deaths += 1;
                weighted_deaths += weight(i);
            } else {
                censored += 1;
            }
            removed += weight(i);
            index += 1;
        }
        if deaths > 0 {
            survival *= (1.0 - weighted_deaths / weighted_risk).clamp(0.0, 1.0);
            if risk > deaths {
                greenwood += deaths as f64 / (risk as f64 * (risk - deaths) as f64);
            }
        }
        let (lower, upper) = if weights.is_some() {
            (None, None)
        } else if survival == 0.0 || survival == 1.0 {
            (Some(survival), Some(survival))
        } else {
            let z = (-survival.ln()).ln();
            let se = greenwood.sqrt() / survival.ln().abs();
            (
                Some((-(z + 1.959963984540054 * se).exp()).exp()),
                Some((-(z - 1.959963984540054 * se).exp()).exp()),
            )
        };
        points.push(Point {
            time,
            at_risk: risk,
            events: deaths,
            censored,
            weighted_at_risk: weighted_risk,
            survival,
            lower,
            upper,
        });
        risk -= index - start;
        weighted_risk = (weighted_risk - removed).max(0.0);
    }
    Ok(Curve {
        label: label.into(),
        patients: rows.len(),
        events: rows.iter().filter(|r| r.event).count(),
        start: rows.iter().map(|r| r.entry).fold(f64::INFINITY, f64::min),
        end: points.last().unwrap().time,
        median: points.iter().find(|p| p.survival <= 0.5).map(|p| p.time),
        weighted: weights.is_some(),
        points,
        risk_table: ticks
            .iter()
            .map(|&t| {
                (
                    t,
                    rows.iter()
                        .filter(|r| (r.entry < t || (t == 0.0 && r.entry == 0.0)) && r.time >= t)
                        .count(),
                )
            })
            .collect(),
    })
}

// erfc approximation (maximum absolute error ~1.5e-7), used only for reported p-values.
fn erfc(x: f64) -> f64 {
    let z = x.abs();
    let t = 1.0 / (1.0 + 0.5 * z);
    let ans = t
        * (-z * z - 1.26551223
            + t * (1.00002368
                + t * (0.37409196
                    + t * (0.09678418
                        + t * (-0.18628806
                            + t * (0.27886807
                                + t * (-1.13520398
                                    + t * (1.48851587 + t * (-0.82215223 + t * 0.17087277)))))))))
            .exp();
    (if x >= 0.0 { ans } else { 2.0 - ans }).clamp(0.0, 2.0)
}
pub fn log_rank(rows: &[&Observation]) -> Result<LogRank, ValidationError> {
    let groups: BTreeSet<_> = rows
        .iter()
        .map(|r| r.concordance == Concordance::Concordant)
        .collect();
    if groups.len() != 2
        || rows.iter().any(|r| {
            !matches!(
                r.concordance,
                Concordance::Concordant | Concordance::Discordant
            )
        })
    {
        return Err(ValidationError(
            "log-rank requires concordant and discordant patients",
        ));
    }
    let ticks: Vec<f64> = vec![];
    let a: Vec<_> = rows
        .iter()
        .copied()
        .filter(|r| r.concordance == Concordance::Concordant)
        .collect();
    let b: Vec<_> = rows
        .iter()
        .copied()
        .filter(|r| r.concordance == Concordance::Discordant)
        .collect();
    let _ = kaplan_meier("validate", rows, None, &ticks)?;
    let mut times: Vec<_> = rows.iter().filter(|r| r.event).map(|r| r.time).collect();
    times.sort_by(f64::total_cmp);
    times.dedup();
    let (mut delta, mut variance) = (0.0, 0.0);
    for t in times {
        let risk =
            |g: &[&Observation]| g.iter().filter(|r| r.entry < t && r.time >= t).count() as f64;
        let deaths =
            |g: &[&Observation]| g.iter().filter(|r| r.event && r.time == t).count() as f64;
        let (n1, n0, d1, d0) = (risk(&a), risk(&b), deaths(&a), deaths(&b));
        let (n, d) = (n1 + n0, d1 + d0);
        delta += d1 - d * n1 / n;
        if n > 1.0 {
            variance += n1 * n0 * d * (n - d) / (n * n * (n - 1.0));
        }
    }
    if variance <= 1e-12 {
        return Err(ValidationError(
            "log-rank unavailable: no informative event risk sets",
        ));
    }
    let chi_squared = delta * delta / variance;
    Ok(LogRank {
        chi_squared,
        p_value: erfc((chi_squared / 2.0).sqrt()).min(1.0),
        observed_minus_expected: delta,
        variance,
    })
}

/// Global, unweighted k-sample log-rank test with hypergeometric tied-event variance.
pub fn grouped_log_rank(rows: &[&Observation]) -> Result<GroupLogRank, ValidationError> {
    let groups: Vec<_> = rows
        .iter()
        .map(|r| r.predicted_class.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if !(2..=32).contains(&groups.len()) || rows.len() > 2_000 {
        return Err(ValidationError(
            "global log-rank requires 2–32 groups and at most 2,000 patients",
        ));
    }
    kaplan_meier("validate", rows, None, &[])?;
    let indices: Vec<_> = rows
        .iter()
        .map(|r| groups.binary_search(&r.predicted_class).unwrap())
        .collect();
    let mut times: Vec<_> = rows.iter().filter(|r| r.event).map(|r| r.time).collect();
    times.sort_by(f64::total_cmp);
    times.dedup();
    let k = groups.len() - 1;
    let mut delta = vec![0.; k];
    let mut variance = vec![vec![0.; k]; k];
    for t in times {
        let mut risk = vec![0.; k + 1];
        let mut deaths = vec![0.; k + 1];
        for (r, &g) in rows.iter().zip(&indices) {
            if r.entry < t && r.time >= t {
                risk[g] += 1.;
            }
            if r.event && r.time == t {
                deaths[g] += 1.;
            }
        }
        let n: f64 = risk.iter().sum();
        let d: f64 = deaths.iter().sum();
        for i in 0..k {
            let pi = risk[i] / n;
            delta[i] += deaths[i] - d * pi;
            if n > 1. {
                for (j, nj) in risk.iter().enumerate().take(k) {
                    variance[i][j] +=
                        d * (n - d) / (n - 1.) * ((if i == j { pi } else { 0. }) - pi * nj / n);
                }
            }
        }
    }
    let inv = inverse(&variance).ok_or(ValidationError(
        "global log-rank unavailable: singular event covariance",
    ))?;
    let chi_squared = delta
        .iter()
        .enumerate()
        .map(|(i, d)| d * inv[i].iter().zip(&delta).map(|(v, e)| v * e).sum::<f64>())
        .sum::<f64>()
        .max(0.);
    let z = chi_squared / 2.;
    // Integer degrees of freedom: half-integer incomplete-gamma recurrence.
    let p_value = if k % 2 == 0 {
        let mut term = (-z).exp();
        let mut sum = term;
        for j in 1..k / 2 {
            term *= z / j as f64;
            sum += term;
        }
        sum
    } else {
        let mut sum = erfc(z.sqrt());
        let mut term = 2. * (z / std::f64::consts::PI).sqrt() * (-z).exp();
        for j in 0..k / 2 {
            sum += term;
            term *= z / (j as f64 + 1.5);
        }
        sum
    }
    .clamp(0., 1.);
    Ok(GroupLogRank {
        groups,
        degrees_of_freedom: k,
        chi_squared,
        p_value,
    })
}

fn inverse(matrix: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = matrix.len();
    let scale = matrix
        .iter()
        .flatten()
        .copied()
        .map(f64::abs)
        .fold(0.0, f64::max);
    if scale == 0.0 || !scale.is_finite() {
        return None;
    }
    let mut a: Vec<Vec<f64>> = matrix
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let mut r = row.clone();
            r.extend((0..n).map(|j| f64::from(i == j)));
            r
        })
        .collect();
    for j in 0..n {
        let pivot = (j..n).max_by(|x, y| a[*x][j].abs().total_cmp(&a[*y][j].abs()))?;
        if a[pivot][j].abs() < scale * 1e-10 {
            return None;
        }
        a.swap(j, pivot);
        let value = a[j][j];
        for v in &mut a[j] {
            *v /= value;
        }
        let row = a[j].clone();
        for (i, r) in a.iter_mut().enumerate() {
            if i != j {
                let factor = r[j];
                for k in 0..2 * n {
                    r[k] -= factor * row[k];
                }
            }
        }
    }
    Some(a.into_iter().map(|r| r[n..].to_vec()).collect())
}
struct PartialLikelihood {
    value: f64,
    gradient: Vec<f64>,
    information: Vec<Vec<f64>>,
}
fn likelihood(rows: &[&Observation], x: &[Vec<f64>], beta: &[f64]) -> PartialLikelihood {
    let p = beta.len();
    let eta: Vec<_> = x
        .iter()
        .map(|r| r.iter().zip(beta).map(|(x, b)| x * b).sum::<f64>())
        .collect();
    let shift = eta.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let weights: Vec<_> = eta.iter().map(|e| (e - shift).exp()).collect();
    let mut result = PartialLikelihood {
        value: 0.0,
        gradient: vec![0.0; p],
        information: vec![vec![0.0; p]; p],
    };
    let mut entries: Vec<_> = (0..rows.len()).collect();
    entries.sort_by(|a, b| rows[*a].entry.total_cmp(&rows[*b].entry));
    let mut exits = entries.clone();
    exits.sort_by(|a, b| rows[*a].time.total_cmp(&rows[*b].time));
    let events: Vec<_> = exits.iter().copied().filter(|&i| rows[i].event).collect();
    let (mut added, mut removed, mut event_index) = (0, 0, 0);
    let mut s0 = 0.0;
    let mut s1 = vec![0.0; p];
    let mut s2 = vec![vec![0.0; p]; p];
    let update = |i: usize, sign: f64, s0: &mut f64, s1: &mut Vec<f64>, s2: &mut Vec<Vec<f64>>| {
        let w = weights[i] * sign;
        *s0 += w;
        for j in 0..p {
            s1[j] += w * x[i][j];
            for k in 0..p {
                s2[j][k] += w * x[i][j] * x[i][k];
            }
        }
    };
    while event_index < events.len() {
        let t = rows[events[event_index]].time;
        while added < entries.len() && rows[entries[added]].entry < t {
            update(entries[added], 1.0, &mut s0, &mut s1, &mut s2);
            added += 1;
        }
        while removed < exits.len() && rows[exits[removed]].time < t {
            update(exits[removed], -1.0, &mut s0, &mut s1, &mut s2);
            removed += 1;
        }
        let mut d = 0.0;
        while event_index < events.len() && rows[events[event_index]].time == t {
            let i = events[event_index];
            d += 1.0;
            result.value += eta[i];
            for (j, g) in result.gradient.iter_mut().enumerate() {
                *g += x[i][j];
            }
            event_index += 1;
        }
        result.value -= d * (s0.ln() + shift);
        for j in 0..p {
            result.gradient[j] -= d * s1[j] / s0;
            for k in 0..p {
                result.information[j][k] += d * (s2[j][k] / s0 - s1[j] * s1[k] / (s0 * s0));
            }
        }
    }
    result
}

/// Unpenalized Cox PH with Breslow ties and model-based Wald intervals.
/// Refuses singular, separated or nonconverged fits instead of returning a plausible HR.
pub fn cox(rows: &[&Observation], covariates: &[String]) -> Result<CoxFit, ValidationError> {
    if covariates.len() > 8
        || rows.iter().any(|r| {
            !r.entry.is_finite()
                || !r.time.is_finite()
                || r.entry < 0.0
                || r.time <= r.entry
                || !matches!(
                    r.concordance,
                    Concordance::Concordant | Concordance::Discordant
                )
        })
    {
        return Err(ValidationError(
            "invalid Cox intervals, treatment groups or covariate count",
        ));
    }
    let events = rows.iter().filter(|r| r.event).count();
    let p = covariates.len() + 1;
    if rows.len() > 2_000 {
        return Err(ValidationError(
            "Cox fit limit is 2,000 patients; curves remain available",
        ));
    }
    if events <= p
        || rows.len() <= p + 1
        || ![Concordance::Concordant, Concordance::Discordant]
            .iter()
            .all(|g| rows.iter().any(|r| r.concordance == *g && r.event))
    {
        return Err(ValidationError(
            "Cox fit needs informative events in both treatment groups",
        ));
    }
    let mut x = Vec::new();
    for r in rows {
        let mut values = vec![f64::from(r.concordance == Concordance::Concordant)];
        for name in covariates {
            values.push(*r.covariates.get(name).ok_or(ValidationError(
                "Cox fit unavailable: missing declared baseline covariate; no imputation",
            ))?);
        }
        if values.iter().any(|v| !v.is_finite()) {
            return Err(ValidationError("nonfinite Cox covariate"));
        }
        x.push(values);
    }
    // Center/scale for conditioning; report coefficients in the original units.
    let means: Vec<_> = (0..p)
        .map(|j| x.iter().map(|r| r[j]).sum::<f64>() / x.len() as f64)
        .collect();
    let scales: Vec<_> = (0..p)
        .map(|j| (x.iter().map(|r| (r[j] - means[j]).powi(2)).sum::<f64>() / x.len() as f64).sqrt())
        .collect();
    if scales.iter().any(|v| *v < 1e-12) {
        return Err(ValidationError("Cox fit unavailable: constant covariate"));
    }
    for r in &mut x {
        for j in 0..p {
            r[j] = (r[j] - means[j]) / scales[j];
        }
    }
    let mut beta = vec![0.0; p];
    for iteration in 0..50 {
        let current = likelihood(rows, &x, &beta);
        let inv = inverse(&current.information).ok_or(ValidationError(
            "Cox fit unavailable: singular information or separation",
        ))?;
        let step: Vec<_> = inv
            .iter()
            .map(|r| {
                r.iter()
                    .zip(&current.gradient)
                    .map(|(a, b)| a * b)
                    .sum::<f64>()
            })
            .collect();
        if step.iter().any(|v| !v.is_finite()) || beta.iter().any(|v| v.abs() > 20.0) {
            return Err(ValidationError(
                "Cox fit unavailable: unstable or separated coefficients",
            ));
        }
        if step.iter().copied().map(f64::abs).fold(0.0, f64::max) < 1e-7 {
            let names = std::iter::once("concordant".to_string()).chain(covariates.iter().cloned());
            let mut coefficients = Vec::new();
            for (j, name) in names.enumerate() {
                let b = beta[j] / scales[j];
                let se = inv[j][j].sqrt() / scales[j];
                let (lo, hi) = (
                    (b - 1.959963984540054 * se).exp(),
                    (b + 1.959963984540054 * se).exp(),
                );
                if !se.is_finite() || se <= 0.0 || !hi.is_finite() {
                    return Err(ValidationError(
                        "Cox fit unavailable: nonfinite uncertainty",
                    ));
                }
                coefficients.push(Coefficient {
                    name,
                    log_hazard_ratio: b,
                    standard_error: se,
                    hazard_ratio: b.exp(),
                    lower: lo,
                    upper: hi,
                    p_value: erfc((b / se).abs() / 2.0_f64.sqrt()).min(1.0),
                });
            }
            return Ok(CoxFit {
                patients: rows.len(),
                events,
                iterations: iteration + 1,
                log_likelihood: current.value,
                coefficients,
                method:
                    "Cox PH; Breslow ties; model-based 95% Wald intervals; concordant vs discordant"
                        .into(),
            });
        }
        let mut factor = 1.0;
        loop {
            let trial: Vec<_> = beta
                .iter()
                .zip(&step)
                .map(|(b, d)| b + factor * d)
                .collect();
            let next = likelihood(rows, &x, &trial);
            if next.value.is_finite() && next.value >= current.value - 1e-10 {
                beta = trial;
                break;
            }
            factor *= 0.5;
            if factor < 1e-8 {
                return Err(ValidationError("Cox fit unavailable: line search failed"));
            }
        }
    }
    Err(ValidationError("Cox fit unavailable: iteration limit"))
}

fn weighted_mean_var(values: &[(f64, f64)]) -> (f64, f64) {
    let n = values.iter().map(|(_, w)| w).sum::<f64>();
    let mean = values.iter().map(|(v, w)| v * w).sum::<f64>() / n;
    (
        mean,
        values
            .iter()
            .map(|(v, w)| w * (v - mean).powi(2))
            .sum::<f64>()
            / n,
    )
}
fn weighting(
    rows: &[&Observation],
    covariates: &[String],
) -> Result<(Vec<f64>, WeightDiagnostics), ValidationError> {
    if rows.is_empty()
        || ![Concordance::Concordant, Concordance::Discordant]
            .iter()
            .all(|g| rows.iter().any(|r| r.concordance == *g))
    {
        return Err(ValidationError("IPTW needs both concordance groups"));
    }
    let mut weights = Vec::new();
    let mut min = 1.0_f64;
    let mut max = 0.0_f64;
    for r in rows {
        let p = r.propensity.ok_or(ValidationError(
            "IPTW unavailable: provide a propensity and its model for every included patient",
        ))?;
        if !(0.01..=0.99).contains(&p) {
            return Err(ValidationError(
                "IPTW unavailable: propensity outside [0.01,0.99]; review overlap, no silent clipping",
            ));
        }
        min = min.min(p);
        max = max.max(p);
        weights.push(if r.concordance == Concordance::Concordant {
            1.0 / p
        } else {
            1.0 / (1.0 - p)
        });
    }
    let ess = |g| {
        let w: Vec<_> = rows
            .iter()
            .zip(&weights)
            .filter(|(r, _)| r.concordance == g)
            .map(|(_, w)| *w)
            .collect();
        w.iter().sum::<f64>().powi(2) / w.iter().map(|x| x * x).sum::<f64>()
    };
    let mut balance = Vec::new();
    for name in covariates {
        let vals: Option<Vec<_>> = rows
            .iter()
            .map(|r| r.covariates.get(name).copied())
            .collect();
        let mut item = Balance {
            covariate: name.clone(),
            unweighted_smd: None,
            weighted_smd: None,
        };
        if let Some(vals) = vals {
            let group = |g, weighted| {
                rows.iter()
                    .enumerate()
                    .filter(|(_, r)| r.concordance == g)
                    .map(|(i, _)| (vals[i], if weighted { weights[i] } else { 1.0 }))
                    .collect::<Vec<_>>()
            };
            let (m1, v1) = weighted_mean_var(&group(Concordance::Concordant, false));
            let (m0, v0) = weighted_mean_var(&group(Concordance::Discordant, false));
            let scale = ((v1 + v0) / 2.0).sqrt();
            if scale > 0.0 {
                item.unweighted_smd = Some((m1 - m0) / scale);
                item.weighted_smd = Some(
                    (weighted_mean_var(&group(Concordance::Concordant, true)).0
                        - weighted_mean_var(&group(Concordance::Discordant, true)).0)
                        / scale,
                );
            }
        }
        balance.push(item);
    }
    let diagnostics = WeightDiagnostics {
        min_propensity: min,
        max_propensity: max,
        max_weight: weights.iter().copied().fold(0.0, f64::max),
        concordant_effective_n: ess(Concordance::Concordant),
        discordant_effective_n: ess(Concordance::Discordant),
        balance,
    };
    Ok((weights, diagnostics))
}

pub fn analyze(data: &Outcomes) -> Result<Report, ValidationError> {
    validate(data)?;
    let max = data.observations.iter().map(|r| r.time).fold(0.0, f64::max);
    let ticks: Vec<_> = (0..=4).map(|i| max * i as f64 / 4.0).collect();
    let mut groups: BTreeMap<&str, Vec<&Observation>> = BTreeMap::new();
    for r in &data.observations {
        groups.entry(&r.predicted_class).or_default().push(r);
    }
    if groups.len() > 32 {
        return Err(ValidationError("at most 32 survival groups are supported"));
    }
    let by_class = groups
        .iter()
        .map(|(g, r)| kaplan_meier(g, r, None, &ticks))
        .collect::<Result<Vec<_>, _>>()?;
    let subtype_lr = grouped_log_rank(&data.observations.iter().collect::<Vec<_>>());
    let included: Vec<_> = data
        .observations
        .iter()
        .filter(|r| {
            matches!(
                r.concordance,
                Concordance::Concordant | Concordance::Discordant
            )
        })
        .collect();
    let mut by_concordance = Vec::new();
    for (kind, name) in [
        (Concordance::Concordant, "Concordant"),
        (Concordance::Discordant, "Discordant"),
    ] {
        let rows: Vec<_> = included
            .iter()
            .copied()
            .filter(|r| r.concordance == kind)
            .collect();
        if !rows.is_empty() {
            by_concordance.push(kaplan_meier(name, &rows, None, &ticks)?);
        }
    }
    let lr = if included.len() <= 2_000 {
        log_rank(&included)
    } else {
        Err(ValidationError(
            "log-rank limit is 2,000 patients; curves remain available",
        ))
    };
    let cox_result = if data.adjustment_covariates.is_empty() {
        Err(ValidationError(
            "adjusted Cox unavailable: declare baseline adjustment covariates",
        ))
    } else {
        cox(&included, &data.adjustment_covariates)
    };
    let mut weighted_concordance = Vec::new();
    let weighted = weighting(&included, &data.adjustment_covariates);
    if let Ok((w, _)) = &weighted {
        for (kind, name) in [
            (Concordance::Concordant, "Concordant"),
            (Concordance::Discordant, "Discordant"),
        ] {
            let mut r = Vec::new();
            let mut weights = Vec::new();
            for (i, row) in included.iter().enumerate() {
                if row.concordance == kind {
                    r.push(*row);
                    weights.push(w[i]);
                }
            }
            weighted_concordance.push(kaplan_meier(name, &r, Some(&weights), &ticks)?);
        }
    }
    Ok(Report {
        patients:data.observations.len(),events:data.observations.iter().filter(|r|r.event).count(),delayed_entries:data.observations.iter().filter(|r|r.entry>0.0).count(),
        excluded_from_concordance:data.observations.len()-included.len(),by_class,by_concordance,weighted_concordance,
        subtype_log_rank_unavailable:subtype_lr.as_ref().err().map(ToString::to_string),subtype_log_rank:subtype_lr.ok(),
        log_rank_unavailable:lr.as_ref().err().map(ToString::to_string),log_rank:lr.ok(),
        cox_unavailable:cox_result.as_ref().err().map(ToString::to_string),cox:cox_result.ok(),
        weighted_unavailable:weighted.as_ref().err().map(ToString::to_string),weights:weighted.ok().map(|(_,d)|d),
        limitations:vec![
            "Descriptive research analysis; retrospective association is not a causal treatment effect.".into(),
            "Independent patients; one event; right censoring and delayed entry. Risk intervals are (entry, exit]; tied exits share a risk set. No competing-risk analysis.".into(),
            "Unweighted curves use pointwise 95% Greenwood log-log intervals; boundaries 0/1 are degenerate. Median not reached is unavailable, not zero.".into(),
            "IPTW uses supplied P(concordant | baseline covariates), unstabilized ATE weights and an explicit overlap gate. Weighted intervals and weighted log-rank are not estimated.".into(),
            "Cox uses Breslow ties and model-based Wald intervals. Proportional hazards, independent censoring, covariate selection and unmeasured confounding require review; no automatic PH test is provided.".into(),
        ],
    })
}
