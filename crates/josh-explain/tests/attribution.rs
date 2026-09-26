use josh_core::{Answer, DataClass, JevRequest, JevResponse, Source, Usage, molecular::*};
use josh_explain::*;
use std::collections::BTreeMap;

fn features() -> FeatureSet {
    FeatureSet {
        schema_version: 1,
        pipeline_version: PIPELINE.into(),
        sample_id: "query".into(),
        patient_group_id: "query-patient".into(),
        data_class: DataClass::Synthetic,
        features: ["x", "y", "dummy"]
            .into_iter()
            .enumerate()
            .map(|(i, name)| Feature {
                id: name.into(),
                name: name.into(),
                modality: Modality::Mutation,
                group: name.into(),
                status: MeasurementStatus::Observed,
                value: Some(FeatureValue::Number {
                    value: 1.0,
                    units: "count".into(),
                }),
                assay: "test".into(),
                reference_build: None,
                coverage: "measured".into(),
                source: FeatureSource {
                    source_id: "invented".into(),
                    sha256: "a".repeat(64),
                    record: i as u64 + 1,
                },
                depends_on: vec![],
            })
            .collect(),
    }
}
fn response(request: &JevRequest) -> JevResponse {
    let value = |name| {
        request.state["features"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == name)
            .and_then(|v| v["value"]["value"].as_f64())
            .unwrap_or(0.0)
    };
    let (x, y) = (value("x"), value("y"));
    let p = 0.1 + 0.2 * x + 0.3 * y + 0.2 * x * y;
    let t = onconpc_taxonomy().criteria();
    let probabilities = t
        .keys()
        .map(|k| {
            (
                k.clone(),
                if k == "NSCLC" {
                    p
                } else {
                    (1.0 - p) / (t.len() - 1) as f64
                },
            )
        })
        .collect();
    JevResponse {
        model: request.model.clone(),
        answers: BTreeMap::from([
            (
                "primary_site".into(),
                Answer::Choice {
                    choice: "NSCLC".into(),
                    probabilities,
                    confidence: 0.0,
                },
            ),
            ("evidence_sufficient".into(), Answer::Noul { noul: 0.0 }),
            ("conflicting_evidence".into(), Answer::Noul { noul: 0.0 }),
        ]),
        usage: Usage {
            input_tokens: 0,
            output_tokens: 0,
        },
    }
}
struct Oracle {
    calls: usize,
}
impl Evaluator for Oracle {
    async fn evaluate(&mut self, r: &JevRequest) -> Result<JevResponse, Error> {
        self.calls += 1;
        Ok(response(r))
    }
}
fn archive(config: Config, background: Option<Background>) -> Archive {
    let f = features();
    let t = onconpc_taxonomy();
    let inference = interpret(&f, &t, response(&prepare(&f, &t).unwrap()), Source::Replay).unwrap();
    Archive::new(f, inference, config, background).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}
#[tokio::test]
async fn legacy_inference_and_explanation_keep_the_original_questions() {
    let f = features();
    let t = onconpc_taxonomy();
    let request = prepare_versioned(&f, &t, LEGACY_PROMPT).unwrap();
    let inference =
        interpret_versioned(&f, &t, response(&request), Source::Replay, LEGACY_PROMPT).unwrap();
    let mut a = Archive::new(
        f,
        inference,
        Config {
            algorithm: Algorithm::Exact,
            ..Config::default()
        },
        None,
    )
    .unwrap();
    let mut oracle = Oracle { calls: 0 };
    run(&mut a, &mut oracle, &Progress::default(), |_| Ok(()))
        .await
        .unwrap();
    a.validate().unwrap();
    for e in a.evaluations.values() {
        assert_eq!(
            hash(&e.request.questions).unwrap(),
            hash(&request.questions).unwrap()
        );
        assert!(e.request.state.get("measurement_semantics").is_none());
    }
    a.inference.prompt_version = PROMPT.into();
    assert!(a.validate().is_err());
}
#[tokio::test]
async fn exact_shapley_splits_interaction_and_assigns_dummy_zero() {
    let mut a = archive(
        Config {
            algorithm: Algorithm::Exact,
            ..Config::default()
        },
        None,
    );
    let mut evaluator = Oracle { calls: 0 };
    run(&mut a, &mut evaluator, &Progress::default(), |_| Ok(()))
        .await
        .unwrap();
    let r = a.result.as_ref().unwrap();
    close(r.baseline_probability, 0.1);
    close(r.full_probability, 0.8);
    let values: BTreeMap<_, _> = r
        .attributions
        .iter()
        .map(|r| (r.group.as_str(), r.contribution))
        .collect();
    close(values["x"], 0.3);
    close(values["y"], 0.4);
    close(values["dummy"], 0.0);
    assert_eq!(evaluator.calls, 7);
    assert_eq!(r.evaluations, 8);
    a.validate().unwrap();
    let replay: Archive = serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
    replay.validate().unwrap();
    let comparison = diagnostics::compare(&a, &replay, 2).unwrap();
    close(comparison.attribution_rmse, 0.0);
    close(comparison.top_k_overlap, 1.0);
    close(comparison.sign_agreement.unwrap(), 1.0);
    close(comparison.magnitude_rank_correlation.unwrap(), 1.0);
    a.retarget("BRCA").unwrap();
    close(a.result.as_ref().unwrap().full_probability, 0.2 / 23.0);
    close(a.result.as_ref().unwrap().additivity_residual, 0.0);
    a.validate().unwrap();
    assert_eq!(
        evaluator.calls, 7,
        "retarget must use archived distributions"
    );
}

#[tokio::test]
async fn interrupted_call_is_checkpointed_and_never_implicitly_resent() {
    struct Failure;
    impl Evaluator for Failure {
        async fn evaluate(&mut self, _: &JevRequest) -> Result<JevResponse, Error> {
            Err(Error::Provider)
        }
    }
    let mut a = archive(
        Config {
            algorithm: Algorithm::Exact,
            ..Config::default()
        },
        None,
    );
    let mut checkpoint = None;
    assert!(matches!(
        run(&mut a, &mut Failure, &Progress::default(), |a| {
            checkpoint = Some(a.clone());
            Ok(())
        })
        .await,
        Err(Error::Provider)
    ));
    let mut saved = checkpoint.unwrap();
    assert_eq!(saved.attempts, 2);
    assert!(saved.in_flight.is_some());
    assert!(saved.uncertain_input_tokens > 0);
    saved.validate().unwrap();
    let mut oracle = Oracle { calls: 0 };
    assert!(matches!(
        run(&mut saved, &mut oracle, &Progress::default(), |_| Ok(())).await,
        Err(Error::Uncertain)
    ));
    assert_eq!(oracle.calls, 0);
    let reserved = saved.uncertain_input_tokens;
    saved.in_flight = None; // Explicit caller-authorized retry.
    run(&mut saved, &mut oracle, &Progress::default(), |_| Ok(()))
        .await
        .unwrap();
    assert_eq!(saved.attempts, 9);
    assert_eq!(saved.uncertain_input_tokens, reserved);
    assert_eq!(oracle.calls, 7);
    saved.validate().unwrap();
}

#[tokio::test]
async fn usage_over_reservation_stops_calls_but_archive_can_be_inspected() {
    struct UnexpectedUsage {
        calls: usize,
    }
    impl Evaluator for UnexpectedUsage {
        async fn evaluate(&mut self, r: &JevRequest) -> Result<JevResponse, Error> {
            self.calls += 1;
            let mut r = response(r);
            r.usage.input_tokens = 60_000;
            Ok(r)
        }
    }
    let mut a = archive(
        Config {
            algorithm: Algorithm::Exact,
            max_input_tokens: 50_000,
            ..Config::default()
        },
        None,
    );
    let mut evaluator = UnexpectedUsage { calls: 0 };
    assert!(matches!(
        run(&mut a, &mut evaluator, &Progress::default(), |_| Ok(())).await,
        Err(Error::Budget)
    ));
    assert_eq!(evaluator.calls, 1);
    a.validate().unwrap();
    assert!(a.result.is_none());
    a.config.max_input_tokens = 1_000_000;
    run(&mut a, &mut evaluator, &Progress::default(), |_| Ok(()))
        .await
        .unwrap();
    assert_eq!(evaluator.calls, 7);
    a.validate().unwrap();
}
#[tokio::test]
async fn sampled_pairs_reconcile_and_match_second_order_oracle() {
    let mut a = archive(
        Config {
            permutation_pairs: 32,
            ..Config::default()
        },
        None,
    );
    run(
        &mut a,
        &mut Oracle { calls: 0 },
        &Progress::default(),
        |_| Ok(()),
    )
    .await
    .unwrap();
    let r = a.result.as_ref().unwrap();
    for v in &r.attributions {
        let expected = match v.group.as_str() {
            "x" => 0.3,
            "y" => 0.4,
            _ => 0.0,
        };
        close(v.contribution, expected);
        assert!(v.sampling_standard_error.unwrap() < 1e-10);
    }
    close(r.additivity_residual, 0.0);
}
#[tokio::test]
async fn background_is_averaged_and_query_patient_cannot_leak() {
    let mut b1 = features();
    b1.sample_id = "b1".into();
    b1.patient_group_id = "p1".into();
    for f in &mut b1.features {
        f.value = Some(FeatureValue::Number {
            value: 0.0,
            units: "count".into(),
        });
    }
    let mut b2 = b1.clone();
    b2.sample_id = "b2".into();
    b2.patient_group_id = "p2".into();
    b2.features[0].value = Some(FeatureValue::Number {
        value: 1.0,
        units: "count".into(),
    });
    let b = Background {
        release_id: "development-v1".into(),
        partition: "development".into(),
        samples: vec![b1, b2],
    };
    let mut a = archive(
        Config {
            algorithm: Algorithm::Exact,
            baseline: Baseline::ReferenceBackground,
            ..Config::default()
        },
        Some(b),
    );
    run(
        &mut a,
        &mut Oracle { calls: 0 },
        &Progress::default(),
        |_| Ok(()),
    )
    .await
    .unwrap();
    let r = a.result.as_ref().unwrap();
    close(r.baseline_probability, 0.2);
    let v: BTreeMap<_, _> = r
        .attributions
        .iter()
        .map(|r| (r.group.as_str(), r.contribution))
        .collect();
    close(v["x"], 0.15);
    close(v["y"], 0.45);
    a.background.as_mut().unwrap().samples[0].patient_group_id =
        a.features.patient_group_id.clone();
    assert!(a.validate().is_err());
}
#[tokio::test]
async fn budgets_cancel_and_resume_preserve_successful_calls() {
    let mut a = archive(
        Config {
            algorithm: Algorithm::Exact,
            max_evaluations: 2,
            ..Config::default()
        },
        None,
    );
    let mut evaluator = Oracle { calls: 0 };
    assert!(matches!(
        run(&mut a, &mut evaluator, &Progress::default(), |_| Ok(())).await,
        Err(Error::Budget)
    ));
    assert_eq!(a.evaluations.len(), 2);
    assert!(a.result.is_none());
    a.config.max_evaluations = 8;
    let progress = Progress::default();
    progress
        .cancelled
        .store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(
        run(&mut a, &mut evaluator, &progress, |_| Ok(())).await,
        Err(Error::Cancelled)
    ));
    run(&mut a, &mut evaluator, &Progress::default(), |_| Ok(()))
        .await
        .unwrap();
    assert_eq!(evaluator.calls, 7);
    a.result.as_mut().unwrap().attributions[0].contribution += 0.1;
    assert!(a.validate().is_err());
}
#[test]
fn masking_removes_names_values_and_sources_and_groups_dependencies() {
    let f = features();
    let r = prepare_masked(&f, &onconpc_taxonomy(), &Default::default()).unwrap();
    let s = serde_json::to_string(&r.state).unwrap();
    assert!(!s.contains("query"));
    assert!(!s.contains("invented"));
    assert!(!s.contains("\"name\""));
    assert!(!s.contains("\"value\""));
    let mut f = f;
    f.features[0].depends_on.push("y".into());
    assert!(f.validate().is_err());
    f.features[0].group = "y".into();
    f.validate().unwrap();
}
