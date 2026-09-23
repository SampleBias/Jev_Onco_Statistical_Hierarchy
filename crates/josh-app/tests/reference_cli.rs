use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_josh"))
        .args(args)
        .env_remove("TYPESAFE_API_KEY")
        .output()
        .unwrap()
}
fn fixture(path: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(path)
        .display()
        .to_string()
}
fn ok(args: &[&str]) -> Value {
    let o = run(args);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}
fn import(temp: &Path, name: &str, source: &str, platform: &str) -> PathBuf {
    let root = temp.join(name);
    ok(&[
        "dataset",
        "import",
        source,
        "--dataset-id",
        name,
        "--units",
        "tpm",
        "--transform",
        "log2-one-plus",
        "--platform",
        platform,
        "--gene-map",
        &fixture("expression/hgnc-subset.tsv"),
        "--gene-map-release",
        "fixture-v1",
        "--synthetic",
        "--out-dir",
        root.to_str().unwrap(),
    ]);
    root
}
fn prepared() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
    let t = tempfile::tempdir().unwrap();
    let reference = import(
        t.path(),
        "reference",
        &fixture("reference/synthetic-reference.tsv"),
        "tutorial-v1",
    );
    let query = import(
        t.path(),
        "query",
        &fixture("reference/synthetic-query.tsv"),
        "tutorial-v1",
    );
    let r = ok(&[
        "reference",
        "build",
        reference.to_str().unwrap(),
        "--labels",
        &fixture("reference/synthetic-labels.tsv"),
        "--release-id",
        "test-v1",
        "--citation",
        "invented software fixture",
        "--min-genes",
        "3",
    ]);
    let path = t.path().join("reference.json");
    std::fs::write(&path, serde_json::to_vec(&r).unwrap()).unwrap();
    (t, reference, query, path)
}
fn compare(reference: &Path, query: &Path) -> Output {
    run(&[
        "reference",
        "compare",
        reference.to_str().unwrap(),
        "--dataset",
        query.to_str().unwrap(),
        "--sample",
        "QUERY-01",
    ])
}
#[test]
fn reference_build_compare_request_and_schema_validation() {
    let (_t, _, query, r) = prepared();
    let output = compare(&r, &query);
    assert!(output.status.success());
    let evidence: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(evidence["status"], "compared");
    assert_eq!(evidence["common_genes"], 6);
    assert_eq!(evidence["similarities"][0]["class_id"], "demo_a");
    // Independent Python statistics.correlation oracle on log2(TPM+1) and class means.
    assert!(
        (evidence["similarities"][0]["pearson_r"].as_f64().unwrap() - 0.9999369933165126).abs()
            < 1e-12
    );
    assert!(
        (evidence["similarities"][1]["pearson_r"].as_f64().unwrap() + 0.8792974315304484).abs()
            < 1e-12
    );
    let repeated = compare(&r, &query);
    assert_eq!(output.stdout, repeated.stdout);
    let request = ok(&[
        "reference",
        "prepare",
        r.to_str().unwrap(),
        "--dataset",
        query.to_str().unwrap(),
        "--sample",
        "QUERY-01",
    ]);
    assert_eq!(request["sends_to_provider"], false);
    assert!(
        request["request"]["questions"]["primary_site"]["criteria"]
            .get("unknown")
            .is_some()
    );
    let state = request["request"]["state"].to_string();
    assert!(!state.contains("QUERY-01"));
    assert!(!state.contains("GROUP-"));
    assert!(!state.contains("source_sha256"));
    let hash = josh_ingest::dataset::artifact(
        "request.json",
        &serde_json::to_vec(
            &serde_json::from_value::<josh_core::JevRequest>(request["request"].clone()).unwrap(),
        )
        .unwrap(),
    )
    .sha256;
    assert_eq!(request["request_sha256"], hash);
    for (kind, document) in [
        (
            "reference",
            serde_json::from_slice(&std::fs::read(&r).unwrap()).unwrap(),
        ),
        ("molecular-evidence", evidence),
    ] {
        let schema = ok(&["schema", kind]);
        jsonschema::validator_for(&schema)
            .unwrap()
            .validate(&document)
            .unwrap();
    }
}
#[test]
fn rejects_reference_self_comparison_and_preserves_export() {
    let (t, reference, query, r) = prepared();
    let o = run(&[
        "reference",
        "compare",
        r.to_str().unwrap(),
        "--dataset",
        reference.to_str().unwrap(),
        "--sample",
        "REF-A1",
    ]);
    assert_eq!(o.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&o.stdout).contains("query_overlaps_reference"));
    let export = t.path().join("analysis.json");
    let args = [
        "reference",
        "compare",
        r.to_str().unwrap(),
        "--dataset",
        query.to_str().unwrap(),
        "--sample",
        "QUERY-01",
        "--output",
        export.to_str().unwrap(),
    ];
    assert!(run(&args).status.success());
    let bytes = std::fs::read(&export).unwrap();
    assert!(!run(&args).status.success());
    assert_eq!(std::fs::read(&export).unwrap(), bytes);
}
#[test]
fn incompatible_platform_blocks_comparison_and_request() {
    let (t, _, _, r) = prepared();
    let q = import(
        t.path(),
        "incompatible",
        &fixture("reference/synthetic-query.tsv"),
        "different-platform",
    );
    let o = compare(&r, &q);
    assert_eq!(o.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&o.stdout).contains("incompatible_expression"));
    let o = run(&[
        "reference",
        "prepare",
        r.to_str().unwrap(),
        "--dataset",
        q.to_str().unwrap(),
        "--sample",
        "QUERY-01",
    ]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("reference_configuration"));
}
#[test]
fn missingness_and_constants_fail_explicitly() {
    let (t, _, _, r) = prepared();
    for (name, data, status) in [
        (
            "missing",
            "gene\tQUERY-01\nTP53\t11\nKRAS\t21\n",
            "insufficient_data",
        ),
        (
            "constant",
            "gene\tQUERY-01\nTP53\t1\nKRAS\t1\nKRT7\t1\nKRT20\t1\nCDX2\t1\nNKX2-1\t1\n",
            "undefined_similarity",
        ),
    ] {
        let file = t.path().join(format!("{name}.tsv"));
        std::fs::write(&file, data).unwrap();
        let q = import(t.path(), name, file.to_str().unwrap(), "tutorial-v1");
        let o = compare(&r, &q);
        assert_eq!(o.status.code(), Some(3));
        let e: Value = serde_json::from_slice(&o.stdout).unwrap();
        assert_eq!(e["status"], status);
    }
}
#[test]
fn malformed_reference_vectors_reserved_classes_and_duplicate_patients_rejected() {
    let (t, _, _, r) = prepared();
    let original: Value = serde_json::from_slice(&std::fs::read(&r).unwrap()).unwrap();
    for i in 0..4 {
        let mut v = original.clone();
        match i {
            0 => v["classes"][0]["mean_expression"] = serde_json::json!([1.]),
            1 => v["classes"][0]["class_id"] = "unknown".into(),
            2 => v["members"][1]["patient_group_id"] = v["members"][0]["patient_group_id"].clone(),
            _ => v["synthetic"] = false.into(),
        };
        let path = t.path().join(format!("bad-{i}.json"));
        std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
        assert!(
            !run(&["reference", "inspect", path.to_str().unwrap()])
                .status
                .success()
        );
    }
}
#[test]
fn source_labels_must_cover_samples_exactly_and_have_unique_groups() {
    let (t, reference, _, _) = prepared();
    let original = std::fs::read_to_string(fixture("reference/synthetic-labels.tsv")).unwrap();
    for (i, data) in [
        original.replace("GROUP-A2", "GROUP-A1"),
        original.replace("REF-A2", "MISSING-REF"),
        original.lines().take(3).collect::<Vec<_>>().join("\n"),
    ]
    .iter()
    .enumerate()
    {
        let labels = t.path().join(format!("bad-labels-{i}.tsv"));
        std::fs::write(&labels, data).unwrap();
        let output = run(&[
            "reference",
            "build",
            reference.to_str().unwrap(),
            "--labels",
            labels.to_str().unwrap(),
            "--release-id",
            "invalid",
            "--citation",
            "fixture",
            "--min-genes",
            "3",
        ]);
        assert!(!output.status.success());
    }
}
