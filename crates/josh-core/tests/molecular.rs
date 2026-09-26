use josh_core::{Answer, Source, molecular::*};
fn sample() -> FeatureSet {
    serde_json::from_str(include_str!(
        "../../../fixtures/molecular/synthetic-features.json"
    ))
    .unwrap()
}

#[test]
fn request_versions_preserve_legacy_bytes_and_describe_only_visible_measurements() {
    let mut f = sample();
    let t = onconpc_taxonomy();
    let legacy = prepare_versioned(&f, &t, LEGACY_PROMPT).unwrap();
    assert_eq!(
        hash(&legacy).unwrap(),
        "3f8af1083490217a5a8099e5d95028edacb6bb553bdaa63ebc78537604579544"
    );
    let previous = prepare_versioned(&f, &t, PREVIOUS_PROMPT).unwrap();
    assert_eq!(
        hash(&previous).unwrap(),
        "4474aa56d9fa26f0c62291f7d91e6e72fc801c226b45b97c6ceedab46f510298"
    );
    assert_eq!(
        hash(&prepare_versioned(&f, &t, V4_PROMPT).unwrap()).unwrap(),
        "0bce7420a9ee88a215c20b3cf212cc94d4d9c7707ade70637996c3a88ad1e034"
    );
    for call in -2..=2 {
        let cna = f
            .features
            .iter_mut()
            .find(|v| matches!(v.value, Some(FeatureValue::CopyNumber { .. })))
            .unwrap();
        let group = cna.group.clone();
        let id = cna.id.clone();
        if let Some(FeatureValue::CopyNumber { call: value, .. }) = &mut cna.value {
            *value = call;
        }
        let candidate = prepare(&f, &t).unwrap();
        let mut ordered = f.features.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|v| &v.id);
        let i = ordered.iter().position(|v| v.id == id).unwrap();
        let v = &candidate.state["features"][i]["value"];
        assert_eq!(v["call"], call);
        assert!(v["call_meaning"].as_str().is_some_and(|s| !s.is_empty()));
        let visible = f.groups().into_iter().filter(|g| g != &group).collect();
        let masked = prepare_masked(&f, &t, &visible).unwrap();
        assert_eq!(masked.state["features"][i].as_object().unwrap().len(), 2);
        assert_eq!(
            masked.state["features"][i]["status"],
            "withheld_for_attribution"
        );
        assert_eq!(
            hash(&masked.questions).unwrap(),
            hash(&candidate.questions).unwrap()
        );
    }
    let candidate = prepare(&f, &t).unwrap();
    assert!(candidate.state.get("measurement_semantics").is_some());
    assert_ne!(hash(&legacy).unwrap(), hash(&candidate).unwrap());
    assert!(prepare_versioned(&f, &t, "unknown-version").is_err());
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
    for version in [LEGACY_PROMPT, PREVIOUS_PROMPT, V4_PROMPT] {
        interpret_versioned(&f, &t, response.clone(), Source::Replay, version)
            .unwrap()
            .verify(&f)
            .unwrap();
    }
    if let Answer::Choice { probabilities, .. } = response.answers.get_mut("primary_site").unwrap()
    {
        probabilities.insert("foreign_class".into(), 0.0);
    }
    assert!(validate_response(&request, &response).is_err());
}

#[test]
fn v4_annotates_somatic_uncertainty_and_respects_custom_class_meaning() {
    let mut f = sample();
    let mut taxonomy = onconpc_taxonomy();
    taxonomy.classes[0].name = "Custom class with reused ID".into();
    for somatic_status in [None, Some(false), Some(true)] {
        for feature in &mut f.features {
            if let Some(FeatureValue::Mutation { somatic, .. }) = &mut feature.value {
                *somatic = somatic_status;
            }
        }
        let request = prepare(&f, &taxonomy).unwrap();
        let mutation = request.state["features"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["value"]["kind"] == "mutation")
            .unwrap();
        let meaning = mutation["value"]["somatic_interpretation"]
            .as_str()
            .unwrap();
        assert!(meaning.contains(match somatic_status {
            None => "unresolved",
            Some(false) => "germline",
            Some(true) => "Reported somatic",
        }));
        if let josh_core::Question::Choice { criteria, .. } = &request.questions["primary_site"] {
            assert!(!criteria["NSCLC"].contains("TTF-1"));
        }
    }
}
