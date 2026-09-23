use josh_core::sample::*;
use josh_ingest::{
    dataset,
    expression::{self, DatasetAdapter, ExpressionTableAdapter, ImportOptions},
};

fn options() -> ImportOptions {
    ImportOptions {
        dataset_id: "expression-test".into(),
        source_name: "synthetic.tsv".into(),
        data_class: josh_core::DataClass::Synthetic,
        config: ExpressionConfig {
            units: ExpressionUnit::Tpm,
            transform: Transform::Log2OnePlus,
            platform: Some("synthetic".into()),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn mapping() -> Option<(Vec<u8>, String)> {
    Some((
        include_bytes!("../../../fixtures/expression/hgnc-subset.tsv").to_vec(),
        "fixture-v1".into(),
    ))
}
fn wide() -> Vec<u8> {
    include_bytes!("../../../fixtures/expression/synthetic-expression.tsv").to_vec()
}

#[test]
fn wide_matrix_yields_independent_samples_with_no_patient_profile() {
    let result = expression::import(wide(), &options(), mapping()).unwrap();
    assert_eq!(result.manifest.samples.len(), 2);
    assert_eq!(result.records[0].len(), 6);
    assert!(
        result
            .manifest
            .samples
            .iter()
            .all(|s| s.patient_group_id.is_none())
    );
    assert_eq!(result.records[0][0].source_record, 2);
    assert_eq!(result.records[0][0].source_column, 2);
    assert_eq!(result.records[1][0].source_column, 3);
    assert_eq!(
        result.manifest.samples[0].assays[0]
            .qc
            .as_ref()
            .unwrap()
            .mapped_records,
        6
    );
}

#[test]
fn quoted_csv_and_long_tables_preserve_original_sample_ids() {
    let source =
        b"gene,sample_id,expression\nTP53,\"sample,A.1\",3\nKRAS,\"sample,A.1\",7\nTP53,B,1\n"
            .to_vec();
    let imported = expression::import(source, &options(), mapping()).unwrap();
    assert_eq!(imported.manifest.samples[1].sample_id, "sample,A.1");
    assert_eq!(
        imported.manifest.detection.unwrap().delimiter,
        Delimiter::Csv
    );
}

#[test]
fn single_sample_table_requires_identity_then_imports() {
    let source = b"gene\texpression\nTP53\t3\n";
    assert!(matches!(
        expression::import(source.to_vec(), &options(), None),
        Err(dataset::DatasetError::SampleId)
    ));
    let mut o = options();
    o.config.single_sample_id = Some("CUP.001".into());
    let data = expression::import(source.to_vec(), &o, mapping()).unwrap();
    assert_eq!(data.manifest.samples[0].sample_id, "CUP.001");
}

#[test]
fn detection_can_be_corrected_for_nonstandard_headers() {
    let bytes = b"ensembl_id,TPM\nENSG00000141510,3\n";
    assert!(
        ExpressionTableAdapter
            .detect(bytes, &options().config)
            .is_err()
    );
    let mut o = options();
    o.config.gene_column = "ensembl_id".into();
    o.config.value_column = "TPM".into();
    o.config.single_sample_id = Some("one".into());
    let result = expression::import(bytes.to_vec(), &o, mapping()).unwrap();
    assert_eq!(
        result.records[0][0].mapping.gene.as_ref().unwrap().symbol,
        "TP53"
    );
}

#[test]
fn malformed_sources_and_duplicate_headers_are_rejected() {
    for source in [
        b"gene\tA\tA\nTP53\t1\t2\n".as_slice(),
        b"gene\tA\nTP53\t1\t2\n",
        b"gene\tA\n\xff\t2\n",
        b"gene\tA\n",
    ] {
        assert!(expression::import(source.to_vec(), &options(), None).is_err());
    }
}

#[test]
fn transcriptome_sized_input_is_not_limited_to_64_findings() {
    let mut source = String::from("gene\tS1\tS2\n");
    for i in 0..20_000 {
        source.push_str(&format!("UNKNOWN_{i}\t{i}\t{}\n", i + 1));
    }
    let result = expression::import(source.into_bytes(), &options(), None).unwrap();
    assert_eq!(result.records[0].len(), 20_000);
    assert_eq!(result.records[1].len(), 20_000);
    assert_eq!(
        result.manifest.samples[0].assays[0]
            .qc
            .as_ref()
            .unwrap()
            .measured_values,
        20_000
    );
}

#[test]
fn archive_retains_source_and_reproduces_mapping_values_and_qc() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("dataset");
    let result = expression::import(wide(), &options(), mapping()).unwrap();
    dataset::write(&root, &result).unwrap();
    assert_eq!(std::fs::read(root.join("source/input")).unwrap(), wide());
    let read = dataset::read(&root).unwrap();
    assert_eq!(
        dataset::read_records(&root, &read.samples[0])
            .unwrap()
            .len(),
        6
    );
    dataset::reproduce(&root).unwrap();
    assert!(
        dataset::write(&root, &result).is_err(),
        "must not overwrite an existing output"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&root).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(root.join("source/input"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[test]
fn artifact_tampering_and_path_escape_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("dataset");
    let mut result = expression::import(wide(), &options(), mapping()).unwrap();
    dataset::write(&root, &result).unwrap();
    std::fs::write(root.join("samples/000000.jsonl"), b"changed").unwrap();
    assert!(dataset::read(&root).is_err());
    result.manifest.source.artifact.path = "../outside".into();
    std::fs::write(
        root.join("dataset.json"),
        serde_json::to_vec(&result.manifest).unwrap(),
    )
    .unwrap();
    assert!(dataset::read(&root).is_err());
}

#[cfg(unix)]
#[test]
fn symlinked_artifacts_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("dataset");
    let result = expression::import(wide(), &options(), mapping()).unwrap();
    dataset::write(&root, &result).unwrap();
    let target = temp.path().join("original");
    std::fs::rename(root.join("source/input"), &target).unwrap();
    std::os::unix::fs::symlink(&target, root.join("source/input")).unwrap();
    assert!(dataset::read(&root).is_err());
}

#[test]
fn legacy_clinical_case_is_preserved_without_fabricating_expression() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("legacy");
    let bytes = include_bytes!("../../../fixtures/clinical-case.json").to_vec();
    let result = dataset::import_legacy(bytes.clone(), "legacy", "case.json", &root).unwrap();
    assert_eq!(
        result.samples[0].assays[0].modality,
        Modality::LegacyAnnotations
    );
    assert_eq!(std::fs::read(root.join("source/input")).unwrap(), bytes);
    assert!(
        dataset::read_records(&root, &result.samples[0])
            .unwrap()
            .is_empty()
    );
    dataset::read(&root).unwrap();
}

#[test]
fn changed_qc_metadata_is_detected_by_reproduction() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("dataset");
    let mut result = expression::import(wide(), &options(), mapping()).unwrap();
    dataset::write(&root, &result).unwrap();
    result.manifest.samples[0].assays[0]
        .qc
        .as_mut()
        .unwrap()
        .measured_values = 99;
    std::fs::write(
        root.join("dataset.json"),
        serde_json::to_vec(&result.manifest).unwrap(),
    )
    .unwrap();
    assert!(dataset::reproduce(&root).is_err());
}
