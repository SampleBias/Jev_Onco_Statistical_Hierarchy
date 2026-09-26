use josh_app::{
    cohort::{Entry, Manifest},
    molecular,
    prompt_comparison::Comparison,
    samples,
};
use josh_core::{
    JevRequest, JevResponse,
    molecular::{self as core, FeatureSet},
};
use josh_explain::Evaluator;
use serde_json::Value;

fn protocol() -> (Manifest, Vec<FeatureSet>) {
    let features = vec![samples::load(0).unwrap(), samples::load(2).unwrap()];
    let manifest = Manifest {
        schema_version: 1,
        protocol_id: "synthetic-engineering-only".into(),
        taxonomy: core::onconpc_taxonomy(),
        cases: ["NSCLC", "COADREAD"]
            .iter()
            .enumerate()
            .map(|(i, truth)| Entry {
                features: format!("sample-{i}.json").into(),
                partition: "test".into(),
                truth: (*truth).into(),
            })
            .collect(),
    };
    (manifest, features)
}
struct FixtureEvaluator {
    calls: Vec<JevRequest>,
    corrupt_call: Option<usize>,
}
impl Evaluator for FixtureEvaluator {
    async fn evaluate(&mut self, request: &JevRequest) -> Result<JevResponse, josh_explain::Error> {
        self.calls.push(request.clone());
        let mut response = molecular::demo_response(request);
        if self.corrupt_call == Some(self.calls.len()) {
            response.model = "wrong-model".into();
        }
        Ok(response)
    }
}
fn read(path: impl AsRef<std::path::Path>) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[tokio::test]
async fn comparison_pairs_inputs_isolates_labels_retains_failures_and_enforces_budgets() {
    let tmp = tempfile::tempdir().unwrap();
    let (manifest, features) = protocol();
    let comparison = Comparison::new(manifest, features).unwrap();
    let mut fixture = FixtureEvaluator {
        calls: vec![],
        corrupt_call: Some(2),
    };
    let root = tmp.path().join("run");
    let report = comparison
        .run(&root, &mut fixture, 3, 100_000, 60)
        .await
        .unwrap();
    assert_eq!(fixture.calls.len(), 3);
    assert_eq!(report["attempts"], 3);
    // Invalid responses cannot release reservations through untrusted usage fields.
    assert_eq!(
        report["input_tokens_or_reserved_failed_tokens"],
        serde_json::to_vec(&fixture.calls[1]).unwrap().len() as u64
    );
    assert_eq!(report["partitions"]["test"]["baseline"]["eligible"], 2);
    assert_eq!(report["partitions"]["test"]["candidate"]["eligible"], 2);
    assert_eq!(report["partitions"]["test"]["baseline"]["failed"], 1);
    assert_eq!(report["partitions"]["test"]["candidate"]["failed"], 1);
    assert_eq!(
        report["records"]["baseline"][1]["failure"],
        "budget_not_run"
    );
    assert!(
        fixture.calls[0]
            .state
            .get("measurement_semantics")
            .is_none()
    );
    assert!(
        fixture.calls[1]
            .state
            .get("measurement_semantics")
            .is_some()
    );
    assert!(
        fixture.calls[2]
            .state
            .get("measurement_semantics")
            .is_some()
    );
    for request in &fixture.calls {
        let text = serde_json::to_string(request).unwrap();
        for forbidden in [
            "patient_group_id",
            "sample_id",
            "\"truth\"",
            "\"partition\"",
        ] {
            assert!(!text.contains(forbidden));
        }
    }
    assert!(
        std::fs::read_dir(root.join("0000-candidate"))
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("response-diagnostic-"))
    );
    assert!(!root.join("0000-candidate/inference.json").exists());
    assert!(
        read(root.join("0000-candidate/outcome.json"))["failure"]
            .as_str()
            .unwrap()
            .contains("model differs")
    );
    assert_eq!(read(root.join("execution.json"))["max_evaluations"], 3);
    assert!(
        comparison
            .run(&root, &mut fixture, 4, 100_000, 60)
            .await
            .is_err()
    );
    assert_eq!(fixture.calls.len(), 3);
    let mut no_calls = FixtureEvaluator {
        calls: vec![],
        corrupt_call: None,
    };
    let report = comparison
        .run(&tmp.path().join("budget"), &mut no_calls, 4, 1, 60)
        .await
        .unwrap();
    assert!(no_calls.calls.is_empty());
    assert_eq!(report["partitions"]["test"]["candidate"]["failed"], 2);
}

#[test]
fn comparison_preflight_rejects_patient_leakage_and_dry_run_sends_nothing() {
    let (mut manifest, mut features) = protocol();
    features[1].patient_group_id = features[0].patient_group_id.clone();
    manifest.cases[1].partition = "development".into();
    assert!(Comparison::new(manifest, features).is_err());
    let (manifest, features) = protocol();
    let comparison = Comparison::new(manifest, features).unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("prepared");
    let result = comparison.prepare(&root).unwrap();
    assert_eq!(result["protocol"]["planned_evaluations"], 4);
    assert_eq!(result["execution"], "prepared_only; no provider calls");
    assert!(!root.join("0000-baseline/attempt.json").exists());
    assert!(root.join("0000-baseline/request.json").exists());
    assert!(root.join("0000-candidate/request.json").exists());
}

#[tokio::test]
async fn inference_archives_specific_failure_and_preserves_the_original_request() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("failed");
    let f = samples::load(4).unwrap();
    let mut fixture = FixtureEvaluator {
        calls: vec![],
        corrupt_call: Some(1),
    };
    let error = molecular::infer_with_evaluator(&root, &f, &core::onconpc_taxonomy(), &mut fixture)
        .await
        .unwrap_err();
    assert!(josh_app::errors::user_message(&error).contains("model differs"));
    assert_eq!(fixture.calls.len(), 1);
    assert_eq!(
        read(root.join("run-status.json"))["status"],
        "failed_or_uncertain"
    );
    let diagnostic = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("response-diagnostic-")
        })
        .unwrap();
    assert_eq!(read(diagnostic)["code"], "model_mismatch");
    assert!(!root.join("inference.json").exists());
}
