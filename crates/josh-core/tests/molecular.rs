use josh_core::{Answer, Source, molecular::*};
fn sample() -> FeatureSet {
    serde_json::from_str(include_str!(
        "../../../fixtures/molecular/synthetic-features.json"
    ))
    .unwrap()
}
#[test]
fn molecular_fixture_preserves_encodings_and_missing_states() {
    let mut f = sample();
    f.validate().unwrap();
    assert_eq!(onconpc_taxonomy().classes.len(), 22);
    let q = prepare(&f, &onconpc_taxonomy()).unwrap();
    let json = serde_json::to_string(&q.state).unwrap();
    assert!(!json.contains("INVENTED-01"));
    assert!(!json.contains("source_id"));
    assert!(!json.contains("sha256"));
    f.features[0].status = MeasurementStatus::NotTested;
    assert!(f.validate().is_err());
    f.features[0].value = None;
    f.validate().unwrap();
    f.features[1].value = Some(FeatureValue::Number {
        value: f64::NAN,
        units: "invalid".into(),
    });
    assert!(f.validate().is_err());
}
#[test]
fn fractions_are_compositional_and_builds_are_explicit() {
    let mut f = sample();
    for feature in f.features.iter_mut().take(2) {
        if let Some(FeatureValue::Signature { units, value, .. }) = &mut feature.value {
            *units = "fraction".into();
            *value = 0.2;
        }
    }
    assert!(f.validate().is_err());
    f.features[0].group = "signatures".into();
    f.features[1].group = "signatures".into();
    f.validate().unwrap();
    f.features[2].reference_build = None;
    assert!(f.validate().is_err());
}
#[test]
fn response_validation_uses_actual_request_options() {
    let f = sample();
    let mut t = onconpc_taxonomy();
    t.classes.truncate(2);
    let request = prepare(&f, &t).unwrap();
    let mut response = josh_core::mock_response();
    if let Some(a) = response.answers.get_mut("primary_site") {
        *a = Answer::Choice {
            choice: t.unknown_id.clone(),
            probabilities: t.criteria().keys().map(|k| (k.clone(), 0.25)).collect(),
            confidence: 0.0,
        };
    }
    validate_response(&request, &response).unwrap();
    let run = interpret(&f, &t, response.clone(), Source::Replay).unwrap();
    run.verify(&f).unwrap();
    if let Answer::Choice { probabilities, .. } = response.answers.get_mut("primary_site").unwrap()
    {
        probabilities.insert("foreign_class".into(), 0.0);
    }
    assert!(validate_response(&request, &response).is_err());
}
