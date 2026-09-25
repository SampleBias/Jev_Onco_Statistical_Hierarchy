use josh_features::{study, survival::*};
use std::collections::BTreeMap;

fn row(id: &str, time: f64, event: bool, concordant: bool) -> Observation {
    Observation {
        patient_id: id.into(),
        predicted_class: "A".into(),
        entry: 0.0,
        time,
        event,
        concordance: if concordant {
            Concordance::Concordant
        } else {
            Concordance::Discordant
        },
        propensity: None,
        covariates: BTreeMap::new(),
    }
}
#[test]
fn global_log_rank_matches_two_group_and_balanced_three_group_oracles() {
    let mut rows = [
        row("1", 1., true, true),
        row("2", 2., true, false),
        row("3", 3., true, true),
        row("4", 4., false, false),
    ];
    for r in &mut rows {
        if r.concordance == Concordance::Discordant {
            r.predicted_class = "B".into();
        }
    }
    let refs: Vec<_> = rows.iter().collect();
    let two = log_rank(&refs).unwrap();
    let global = grouped_log_rank(&refs).unwrap();
    assert!((two.chi_squared - global.chi_squared).abs() < 1e-12);
    assert!((two.p_value - global.p_value).abs() < 1e-12);
    let mut balanced = Vec::new();
    for group in ["A", "B", "C"] {
        for t in 1..=3 {
            let mut r = row(&format!("{group}{t}"), t as f64, t < 3, true);
            r.predicted_class = group.into();
            balanced.push(r);
        }
    }
    let global = grouped_log_rank(&balanced.iter().collect::<Vec<_>>()).unwrap();
    assert_eq!(global.degrees_of_freedom, 2);
    assert_eq!(global.chi_squared, 0.);
    assert_eq!(global.p_value, 1.);
}
#[test]
fn kaplan_meier_ties_censoring_and_greenwood_match_hand_calculation() {
    let rows = [
        row("1", 1., true, true),
        row("2", 2., false, true),
        row("3", 2., true, true),
        row("4", 3., true, true),
    ];
    let refs: Vec<_> = rows.iter().collect();
    let c = kaplan_meier("A", &refs, None, &[0., 1., 2., 3., 4.]).unwrap();
    assert_eq!(
        c.points.iter().map(|p| p.at_risk).collect::<Vec<_>>(),
        [4, 3, 1]
    );
    assert_eq!(
        c.points.iter().map(|p| p.survival).collect::<Vec<_>>(),
        [0.75, 0.5, 0.]
    );
    assert_eq!(c.median, Some(2.));
    assert_eq!(c.risk_table, [(0., 4), (1., 4), (2., 3), (3., 1), (4., 0)]);
    let se = 0.5 / 0.5_f64.ln().abs();
    let z = (-0.5_f64.ln()).ln();
    assert!(
        (c.points[1].lower.unwrap() - (-(z + 1.959963984540054 * se).exp()).exp()).abs() < 1e-12
    );
    assert!(
        (c.points[1].upper.unwrap() - (-(z - 1.959963984540054 * se).exp()).exp()).abs() < 1e-12
    );
    assert_eq!(c.points[1].censored, 1);
}
#[test]
fn delayed_entry_is_strict_and_weighted_curves_do_not_invent_intervals() {
    let mut rows = [
        row("1", 1., true, true),
        row("2", 3., false, true),
        row("3", 2., true, true),
    ];
    rows[2].entry = 1.;
    let refs: Vec<_> = rows.iter().collect();
    let c = kaplan_meier("A", &refs, None, &[0., 1., 2.]).unwrap();
    assert_eq!(c.points[0].at_risk, 2);
    assert_eq!(c.points[0].survival, 0.5);
    assert_eq!(c.points[1].at_risk, 2);
    assert_eq!(c.points[1].survival, 0.25);
    let w = kaplan_meier("A", &refs, Some(&[2., 1., 1.]), &[]).unwrap();
    assert!((w.points[0].survival - 1. / 3.).abs() < 1e-12);
    assert!(
        w.points
            .iter()
            .all(|p| p.lower.is_none() && p.upper.is_none())
    );
}
#[test]
fn all_censored_has_no_median_and_invalid_intervals_are_rejected() {
    let mut rows = [row("1", 1., false, true), row("2", 2., false, true)];
    let c = kaplan_meier("A", &rows.iter().collect::<Vec<_>>(), None, &[]).unwrap();
    assert!(c.median.is_none());
    assert!(c.points.iter().all(|p| p.survival == 1.));
    rows[0].entry = 1.;
    assert!(kaplan_meier("A", &rows.iter().collect::<Vec<_>>(), None, &[]).is_err());
}
#[test]
fn multivariable_cox_delayed_entry_and_ties_match_independent_likelihood_derivatives() {
    // Independent brute-force risk sets and numerical derivatives (no fitting helper reuse).
    let mut rows = Vec::new();
    for i in 0..36 {
        let mut r = row(&i.to_string(), 2. + (i % 9) as f64, i % 5 != 0, i % 3 == 0);
        r.entry = if i % 4 == 0 { 1.5 } else { 0. };
        r.covariates
            .insert("baseline".into(), ((i * 7) % 11) as f64 / 3.);
        rows.push(r);
    }
    let refs: Vec<_> = rows.iter().collect();
    let fit = cox(&refs, &["baseline".into()]).unwrap();
    let beta = [
        fit.coefficients[0].log_hazard_ratio,
        fit.coefficients[1].log_hazard_ratio,
    ];
    let ll = |b: [f64; 2]| {
        let eta = |r: &Observation| {
            b[0] * f64::from(r.concordance == Concordance::Concordant)
                + b[1] * r.covariates["baseline"]
        };
        rows.iter()
            .filter(|r| r.event)
            .map(|event| {
                eta(event)
                    - rows
                        .iter()
                        .filter(|r| r.entry < event.time && r.time >= event.time)
                        .map(|r| eta(r).exp())
                        .sum::<f64>()
                        .ln()
            })
            .sum::<f64>()
    };
    assert!((ll(beta) - fit.log_likelihood).abs() < 1e-9);
    let h = 1e-4;
    let mut info = [[0.; 2]; 2];
    for j in 0..2 {
        let mut plus = beta;
        let mut minus = beta;
        plus[j] += h;
        minus[j] -= h;
        assert!(((ll(plus) - ll(minus)) / (2. * h)).abs() < 1e-5);
        info[j][j] = -(ll(plus) - 2. * ll(beta) + ll(minus)) / (h * h);
    }
    info[0][1] = -(ll([beta[0] + h, beta[1] + h])
        - ll([beta[0] + h, beta[1] - h])
        - ll([beta[0] - h, beta[1] + h])
        + ll([beta[0] - h, beta[1] - h]))
        / (4. * h * h);
    let determinant = info[0][0] * info[1][1] - info[0][1].powi(2);
    for j in 0..2 {
        let se = (info[1 - j][1 - j] / determinant).sqrt();
        assert!((se - fit.coefficients[j].standard_error).abs() < 1e-5);
    }
    rows.reverse();
    let permuted = cox(&rows.iter().collect::<Vec<_>>(), &["baseline".into()]).unwrap();
    for (a, b) in fit.coefficients.iter().zip(&permuted.coefficients) {
        assert!((a.log_hazard_ratio - b.log_hazard_ratio).abs() < 1e-10);
    }
}
#[test]
fn cox_binary_fit_matches_closed_form_hazard_ratio_and_information() {
    // A, B, A, B events. Score equation: x^2 - x - 4 = 0, x=exp(beta).
    let rows = [
        row("1", 1., true, true),
        row("2", 2., true, false),
        row("3", 3., true, true),
        row("4", 4., true, false),
    ];
    let fit = cox(&rows.iter().collect::<Vec<_>>(), &[]).unwrap();
    let x = (1. + 17.0_f64.sqrt()) / 2.;
    assert!((fit.coefficients[0].hazard_ratio - x).abs() < 1e-6);
    let information = 2. * x / (x + 1.).powi(2) + 2. * x / (x + 2.).powi(2);
    assert!((fit.coefficients[0].standard_error - 1. / information.sqrt()).abs() < 1e-6);
    assert!(fit.coefficients[0].lower < x && fit.coefficients[0].upper > x);
}
#[test]
fn balanced_ties_have_unit_hazard_and_zero_logrank() {
    let rows = [
        row("1", 1., true, true),
        row("2", 1., true, false),
        row("3", 2., true, true),
        row("4", 2., true, false),
    ];
    let refs: Vec<_> = rows.iter().collect();
    let fit = cox(&refs, &[]).unwrap();
    assert!((fit.coefficients[0].hazard_ratio - 1.).abs() < 1e-12);
    assert!((fit.coefficients[0].standard_error - 1.).abs() < 1e-12);
    let lr = log_rank(&refs).unwrap();
    assert_eq!(lr.chi_squared, 0.);
    assert!((lr.p_value - 1.).abs() < 1e-6);
}
#[test]
fn logrank_matches_manual_hypergeometric_variance() {
    let rows = [
        row("1", 1., true, true),
        row("2", 2., true, false),
        row("3", 3., true, true),
        row("4", 4., true, false),
    ];
    let lr = log_rank(&rows.iter().collect::<Vec<_>>()).unwrap();
    let expected_delta = 0.5 - 1. / 3. + 0.5;
    let expected_variance = 0.25 + 2. / 9. + 0.25;
    assert!((lr.observed_minus_expected - expected_delta).abs() < 1e-12);
    assert!((lr.variance - expected_variance).abs() < 1e-12);
}
#[test]
fn separated_or_collinear_cox_fits_are_unavailable() {
    let mut rows = [
        row("1", 1., true, true),
        row("2", 2., true, true),
        row("3", 3., true, false),
        row("4", 4., true, false),
    ];
    assert!(cox(&rows.iter().collect::<Vec<_>>(), &[]).is_err());
    for r in &mut rows {
        r.covariates.insert("constant".into(), 1.);
    }
    assert!(cox(&rows.iter().collect::<Vec<_>>(), &["constant".into()]).is_err());
}
#[test]
fn survival_validation_rejects_duplicate_patients_and_missing_propensity_provenance() {
    let mut data = Outcomes {
        endpoint: "overall survival".into(),
        time_origin: "sequencing".into(),
        time_unit: "months".into(),
        source: "invented".into(),
        adjustment_covariates: vec![],
        propensity_model: None,
        observations: vec![row("1", 1., true, true), row("2", 2., false, false)],
    };
    assert!(validate(&data).is_ok());
    data.observations[1].patient_id = "1".into();
    assert!(validate(&data).is_err());
    data.observations[1].patient_id = "2".into();
    data.observations[0].propensity = Some(0.5);
    assert!(validate(&data).is_err());
    data.propensity_model = Some("prespecified logistic model".into());
    assert!(validate(&data).is_ok());
    let report = analyze(&data).unwrap();
    assert!(report.weighted_unavailable.is_some());
    data.observations[1].propensity = Some(0.5);
    let report = analyze(&data).unwrap();
    assert_eq!(report.weighted_concordance.len(), 2);
    data.observations[1].propensity = Some(0.001);
    assert!(analyze(&data).unwrap().weighted_unavailable.is_some());
}
#[test]
fn stable_winner_uses_alphabetical_tie_break_and_keeps_unknown() {
    let r = josh_features::evaluation::Record {
        sample_id: "s".into(),
        patient_group_id: "p".into(),
        partition: "test".into(),
        truth: "a".into(),
        probabilities: Some(BTreeMap::from([("a".into(), 0.5), ("b".into(), 0.5)])),
        accepted: false,
        failure: None,
    };
    assert_eq!(study::winner(&r), Some(("a", 0.5)));
}
