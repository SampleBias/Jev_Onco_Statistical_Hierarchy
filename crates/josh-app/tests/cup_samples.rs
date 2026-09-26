use josh_app::{molecular, samples};
use josh_core::{
    Source,
    molecular::{self as core, FeatureValue, MeasurementStatus, Modality},
};
use std::collections::BTreeSet;

#[test]
fn ten_cases_keep_labels_local_and_round_trip_through_provider_validation_and_archives() {
    let scenarios: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/cup-realistic-v2/scenarios.json"
    ))
    .unwrap();
    let taxonomy = core::onconpc_taxonomy();
    let mut patients = BTreeSet::new();
    for i in 0..10 {
        let features = samples::load(i).unwrap();
        assert_eq!(scenarios["scenarios"][i]["sample_id"], features.sample_id);
        assert!(patients.insert(features.patient_group_id.clone()));
        for version in [core::PREVIOUS_PROMPT, core::PROMPT] {
            let request = core::prepare_versioned(&features, &taxonomy, version).unwrap();
            let bytes = serde_json::to_vec(&request).unwrap();
            assert!(bytes.len() <= core::MAX_REQUEST_BYTES);
            let state = serde_json::to_string(&request.state).unwrap();
            for forbidden in [
                "scenario",
                "candidates",
                "patient_group_id",
                "sample_id",
                "source_id",
                "partition",
                "truth",
            ] {
                assert!(
                    !state.contains(forbidden),
                    "{forbidden} leaked into {}",
                    features.sample_id
                );
            }
            let response = molecular::demo_response(&request);
            let decoded =
                josh_jev::decode_response(&request, &serde_json::to_vec(&response).unwrap(), 200)
                    .unwrap();
            let run =
                core::interpret_versioned(&features, &taxonomy, decoded, Source::Mock, version)
                    .unwrap();
            run.verify(&features).unwrap();
            assert_eq!(run.status, "abstained"); // Offline fixture is never a live classification.
        }
    }
}

#[test]
fn realistic_missingness_and_measured_zero_are_distinct_and_ablation_stays_valid() {
    let sparse = samples::load(2).unwrap();
    assert!(
        sparse
            .features
            .iter()
            .any(|f| f.status == MeasurementStatus::Unknown)
    );
    assert!(
        sparse
            .features
            .iter()
            .any(|f| f.status == MeasurementStatus::NotTested)
    );
    for i in 0..10 {
        let f = samples::load(i).unwrap();
        assert!(
            f.features
                .iter()
                .filter(|f| f.status != MeasurementStatus::Observed)
                .all(|f| f.value.is_none())
        );
        assert!(
            f.features
                .iter()
                .filter(|f| f.modality == Modality::Signature)
                .all(|f| f.status == MeasurementStatus::NotTested)
        );
        for genomic in [true, false] {
            let mut ablated = f.clone();
            ablated.features.retain(|f| {
                f.modality == Modality::Demographic
                    || (matches!(
                        f.modality,
                        Modality::Mutation | Modality::CopyNumber | Modality::Signature
                    ) == genomic
                        && f.modality != Modality::Expression)
            });
            ablated.validate().unwrap();
            assert_eq!(ablated.patient_group_id, f.patient_group_id);
            core::prepare(&ablated, &core::onconpc_taxonomy()).unwrap();
        }
    }
    let full = samples::load(0).unwrap();
    assert!(full.features.iter().any(|f| matches!(
        f.value,
        Some(FeatureValue::Number { value: 0.0, .. })
    ) && f.status == MeasurementStatus::Observed));
    assert!(
        full.features
            .iter()
            .any(|f| matches!(f.value, Some(FeatureValue::CopyNumber { call: 0, .. })))
    );
}

#[test]
fn masked_inventory_does_not_reveal_hidden_names_assays_or_modality() {
    let f = samples::load(6).unwrap();
    let taxonomy = core::onconpc_taxonomy();
    let full = core::prepare(&f, &taxonomy).unwrap();
    let empty = core::prepare_masked(&f, &taxonomy, &BTreeSet::new()).unwrap();
    assert_eq!(
        empty.state["evidence_inventory"]["withheld_observations"]
            .as_u64()
            .unwrap() as usize,
        f.features
            .iter()
            .filter(|f| f.status == MeasurementStatus::Observed)
            .count()
    );
    assert!(
        empty.state["evidence_inventory"]["by_modality"]["ihc"]
            .get("observed")
            .is_none()
    );
    assert!(
        !serde_json::to_string(&empty.state)
            .unwrap()
            .contains("PAX8")
    );
    for group in f.groups() {
        let visible = f.groups().into_iter().filter(|g| g != &group).collect();
        let masked = core::prepare_masked(&f, &taxonomy, &visible).unwrap();
        let mut altered = f.clone();
        for feature in &mut altered.features {
            if feature.group == group && feature.status == MeasurementStatus::Observed {
                feature.name = "SECRET_HIDDEN_NAME".into();
                feature.assay = "SECRET_HIDDEN_ASSAY".into();
                feature.coverage = "SECRET_HIDDEN_COVERAGE".into();
            }
        }
        let changed = core::prepare_masked(&altered, &taxonomy, &visible).unwrap();
        assert_eq!(core::hash(&masked).unwrap(), core::hash(&changed).unwrap());
        assert_eq!(
            core::hash(&masked.questions).unwrap(),
            core::hash(&full.questions).unwrap()
        );
    }
}
