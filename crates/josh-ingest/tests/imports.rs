use josh_core::{Case, ObservationStatus, prepare, provenance};
use josh_ingest::*;
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};

const CSV: &str = include_str!("../../../fixtures/import/synthetic-findings.csv");
fn options(format: InputFormat) -> Options {
    Options {
        format,
        source_id: "synthetic-v1".into(),
        split_seed: "study-v1".into(),
        holdout_institution: None,
    }
}
fn original() -> Case {
    serde_json::from_str(include_str!("../../../fixtures/synthetic-case.json")).unwrap()
}

#[test]
fn schema_three_import_preserves_clinical_context_reviews_and_request() {
    let mut case: Case =
        serde_json::from_str(include_str!("../../../fixtures/clinical-case.json")).unwrap();
    josh_core::guidance::record_review(
        &mut case,
        "scope",
        josh_core::clinical::ReviewAction::Acknowledged,
        "Synthetic review".into(),
        josh_core::clinical::Assessment {
            reviewer_id: "reviewer".into(),
            recorded_on: "2026-09-22".into(),
            source: "synthetic test".into(),
        },
    )
    .unwrap();
    let bytes = serde_json::to_vec(&case).unwrap();
    let mut imported = import(bytes.as_slice(), &options(InputFormat::Json)).unwrap();
    assert_eq!(imported.report.accepted_cases, 1);
    assert_eq!(
        serde_json::to_value(&imported.cases[0]).unwrap(),
        serde_json::to_value(&case).unwrap()
    );
    assert_eq!(
        provenance::request_sha256(&prepare(&imported.cases[0]).unwrap()).unwrap(),
        provenance::request_sha256(&prepare(&case).unwrap()).unwrap()
    );
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("clinical-bundle");
    bundle::write(&root, &mut imported).unwrap();
    let restored = bundle::read_case(&root, &imported.report.cases[0]).unwrap();
    assert_eq!(
        provenance::case_revision(&restored).unwrap(),
        provenance::case_revision(&case).unwrap()
    );
    assert!(josh_core::guidance::evaluate(&restored).unwrap().items[0].review_is_current);
}
fn rows(changes: impl FnOnce(&mut Vec<Vec<String>>)) -> String {
    let mut reader = csv::Reader::from_reader(CSV.as_bytes());
    let headers = reader.headers().unwrap().clone();
    let mut records: Vec<Vec<String>> = reader
        .records()
        .map(|r| r.unwrap().iter().map(str::to_owned).collect())
        .collect();
    changes(&mut records);
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(&headers).unwrap();
    for record in records {
        writer.write_record(record).unwrap();
    }
    String::from_utf8(writer.into_inner().unwrap()).unwrap()
}
fn codes(bundle: &ImportBundle) -> Vec<IssueCode> {
    bundle.report.issues.iter().map(|i| i.code).collect()
}

#[test]
fn csv_import_tracks_sources_status_censoring_and_patient_groups() {
    let batch = import(CSV.as_bytes(), &options(InputFormat::Csv)).unwrap();
    assert_eq!(batch.report.accepted_cases, 3);
    assert_eq!(batch.report.accepted_records, 7);
    assert_eq!(batch.report.rejected_records, 0);
    assert_eq!(
        batch.report.source.sha256,
        format!("{:x}", Sha256::digest(CSV.as_bytes()))
    );
    assert_eq!(batch.cases[0].schema_version, 2);
    assert_eq!(batch.cases[0].age_years, None);
    assert_eq!(batch.cases[0].age_lower_bound_exclusive, Some(89));
    assert_eq!(
        batch.cases[0]
            .metadata
            .as_ref()
            .unwrap()
            .patient_id
            .as_deref(),
        Some("0001")
    );
    assert_eq!(
        batch.cases[0].findings[1].source.as_ref().unwrap().record,
        3
    );
    assert_eq!(batch.report.observation_counts["not_tested"], 1);
    assert_eq!(batch.report.observation_counts["negative"], 1);
    assert_eq!(
        batch.splits.entries[0].partition,
        batch.splits.entries[1].partition
    );
    assert_eq!(batch.labels.len(), 3);
    assert!(!batch.report.token_budget_verified);
    assert_eq!(
        batch
            .report
            .warnings
            .iter()
            .find(|w| w.code == IssueCode::UnknownObservation)
            .unwrap()
            .record,
        8
    );
}

#[test]
fn labels_and_local_metadata_never_change_provider_payload() {
    let a = import(CSV.as_bytes(), &options(InputFormat::Csv)).unwrap();
    let altered = rows(|records| {
        for row in records {
            row[1] = "LOCAL_ONLY_PATIENT".into();
            row[3] = "LOCAL_ONLY_INSTITUTION".into();
            row[8] = "LOCAL_ONLY_CUTOFF".into();
            row[18] = "breast".into();
        }
    });
    let mut config = options(InputFormat::Csv);
    config.source_id = "LOCAL_ONLY_SOURCE".into();
    let b = import(altered.as_bytes(), &config).unwrap();
    for (a, b) in a.cases.iter().zip(&b.cases) {
        let ra = prepare(a).unwrap();
        let rb = prepare(b).unwrap();
        assert_eq!(
            serde_json::to_value(&ra).unwrap(),
            serde_json::to_value(&rb).unwrap()
        );
        assert_ne!(
            provenance::case_revision(a).unwrap(),
            provenance::case_revision(b).unwrap()
        );
        let payload = serde_json::to_string(&rb).unwrap();
        assert!(!payload.contains("LOCAL_ONLY"));
        assert!(!rb.state.to_string().contains("known_primary"));
        assert!(!rb.state.to_string().contains("source_sha256"));
    }
    assert_eq!(b.labels[0].primary_origin, "breast");
}

#[test]
fn identical_sources_produce_identical_reports_cases_and_splits() {
    let a = import(CSV.as_bytes(), &options(InputFormat::Csv)).unwrap();
    let b = import(CSV.as_bytes(), &options(InputFormat::Csv)).unwrap();
    assert_eq!(
        serde_json::to_value(&a.report).unwrap(),
        serde_json::to_value(&b.report).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&a.cases).unwrap(),
        serde_json::to_value(&b.cases).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&a.splits).unwrap(),
        serde_json::to_value(&b.splits).unwrap()
    );
}

#[test]
fn v1_migration_preserves_evidence_and_does_not_invent_patient_groups() {
    let input = serde_json::to_vec(&original()).unwrap();
    let batch = import(Cursor::new(input), &options(InputFormat::Json)).unwrap();
    let case = &batch.cases[0];
    for (old, migrated) in original().findings.iter().zip(&case.findings) {
        assert_eq!(old.value, migrated.value);
        assert!(migrated.source.is_some());
    }
    assert_eq!(batch.splits.entries[0].partition, Partition::Unassigned);
    assert!(
        batch
            .report
            .warnings
            .iter()
            .any(|w| w.code == IssueCode::MissingPatientGroup)
    );
    assert!(batch.labels.is_empty());
    assert_eq!(
        josh_core::prompt_version(case),
        josh_core::EVIDENCE_PROMPT_VERSION
    );
}

#[test]
fn malformed_and_oversized_jsonl_records_are_reported_without_losing_next_case() {
    let valid = serde_json::to_string(&original()).unwrap();
    let input = format!(
        "{}\n{{broken-private-text\n{valid}\n",
        "x".repeat(MAX_RECORD_BYTES + 100)
    );
    let batch = import(input.as_bytes(), &options(InputFormat::Jsonl)).unwrap();
    assert_eq!(batch.report.source.records, 3);
    assert_eq!(batch.report.accepted_records, 1);
    assert_eq!(batch.report.rejected_records, 2);
    assert_eq!(
        codes(&batch),
        vec![IssueCode::RecordTooLarge, IssueCode::InvalidJson]
    );
    assert!(
        !serde_json::to_string(&batch.report)
            .unwrap()
            .contains("private-text")
    );
    assert_eq!(
        batch.cases[0].findings[0].source.as_ref().unwrap().record,
        3
    );
}

#[test]
fn duplicate_case_ids_withdraw_all_occurrences() {
    let valid = serde_json::to_string(&original()).unwrap();
    let batch = import(
        format!("{valid}\n{valid}\n").as_bytes(),
        &options(InputFormat::Jsonl),
    )
    .unwrap();
    assert!(batch.cases.is_empty());
    assert!(codes(&batch).contains(&IssueCode::DuplicateCase));
    assert_eq!(batch.report.rejected_records, 2);
}

#[test]
fn canonical_label_fields_are_rejected_and_poison_earlier_matching_case() {
    let valid = serde_json::to_value(original()).unwrap();
    let mut leaked = valid.clone();
    leaked["known_primary"] = "lung".into();
    let batch = import(
        format!("{valid}\n{leaked}\n").as_bytes(),
        &options(InputFormat::Jsonl),
    )
    .unwrap();
    assert!(batch.cases.is_empty());
    assert_eq!(batch.report.rejected_records, 2);
}

#[test]
fn invalid_tabular_rows_withdraw_whole_cases_instead_of_partial_evidence() {
    for (column, value, expected) in [
        (9, "F1", IssueCode::DuplicateFinding),
        (5, "42", IssueCode::ConflictingMetadata),
        (18, "breast", IssueCode::ConflictingLabel),
        (18, "not-a-label", IssueCode::InvalidLabel),
        (5, ">=89", IssueCode::InvalidAge),
        (13, "negative", IssueCode::ConflictingStatus),
    ] {
        let input = rows(|rows| rows[1][column] = value.into());
        let batch = import(input.as_bytes(), &options(InputFormat::Csv)).unwrap();
        assert_eq!(batch.report.accepted_cases, 2, "{expected:?}");
        assert_eq!(batch.report.rejected_records, 2, "{expected:?}");
        assert!(codes(&batch).contains(&expected));
        assert!(batch.cases.iter().all(|case| case.case_id != "DEMO-001"));
        assert!(batch.labels.iter().all(|label| label.case_id != "DEMO-001"));
        assert!(
            batch
                .splits
                .entries
                .iter()
                .all(|entry| entry.case_id != "DEMO-001")
        );
    }
}

#[test]
fn duplicate_samples_across_cases_are_rejected_while_multiple_patient_samples_are_allowed() {
    let input = rows(|rows| {
        rows[2][2] = "S001".into();
        rows[3][2] = "S001".into();
    });
    let batch = import(input.as_bytes(), &options(InputFormat::Csv)).unwrap();
    assert_eq!(batch.cases.len(), 1);
    assert!(codes(&batch).contains(&IssueCode::DuplicateSample));
    assert_eq!(batch.report.rejected_records, 4);
}

#[test]
fn institutional_holdout_moves_whole_patient_even_with_cross_site_samples() {
    let input = rows(|rows| {
        rows[2][3] = "SITE-C".into();
        rows[3][3] = "SITE-C".into();
    });
    let mut config = options(InputFormat::Csv);
    config.holdout_institution = Some("SITE-C".into());
    let batch = import(input.as_bytes(), &config).unwrap();
    assert_eq!(batch.splits.entries[0].partition, Partition::Test);
    assert_eq!(batch.splits.entries[1].partition, Partition::Test);
    // Reordering cases/source rows never changes the patient-group assignment.
    let reversed = rows(|rows| rows.reverse());
    let a = import(CSV.as_bytes(), &options(InputFormat::Csv)).unwrap();
    let b = import(reversed.as_bytes(), &options(InputFormat::Csv)).unwrap();
    assert_eq!(
        serde_json::to_value(&a.splits).unwrap(),
        serde_json::to_value(&b.splits).unwrap()
    );
}

#[test]
fn tsv_and_quoted_multiline_csv_preserve_values_and_record_ordinals() {
    let input = rows(|rows| rows[0][12] = "synthetic, multiline\nobservation".into());
    let csv_batch = import(input.as_bytes(), &options(InputFormat::Csv)).unwrap();
    assert_eq!(
        csv_batch.cases[0].findings[0].value,
        "synthetic, multiline\nobservation"
    );
    assert_eq!(
        csv_batch.cases[0].findings[1]
            .source
            .as_ref()
            .unwrap()
            .record,
        3
    );
    let mut reader = csv::Reader::from_reader(input.as_bytes());
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .from_writer(vec![]);
    writer.write_record(reader.headers().unwrap()).unwrap();
    for record in reader.records() {
        writer.write_record(&record.unwrap()).unwrap();
    }
    let tsv_batch = import(
        writer.into_inner().unwrap().as_slice(),
        &options(InputFormat::Tsv),
    )
    .unwrap();
    assert_eq!(tsv_batch.cases.len(), 3);
    assert_eq!(
        prepare(&csv_batch.cases[0]).unwrap().state,
        prepare(&tsv_batch.cases[0]).unwrap().state
    );
}

#[test]
fn unknown_duplicate_and_missing_headers_fail_without_echoing_data() {
    for header in [
        "case_id,data_class,finding_id,kind,name,value,private-extra\n",
        "case_id,data_class,finding_id,kind,name,value,case_id\n",
        "case_id,value\n",
    ] {
        let error = import(header.as_bytes(), &options(InputFormat::Csv))
            .err()
            .unwrap();
        assert!(matches!(error, Error::Columns));
        assert!(!error.to_string().contains("private-extra"));
    }
}

#[test]
fn import_bounds_are_enforced_before_outputs_exist() {
    assert!(matches!(
        import(
            std::io::repeat(b'x').take(MAX_SOURCE_BYTES + 1),
            &options(InputFormat::Jsonl)
        ),
        Err(Error::SourceTooLarge)
    ));
    assert!(matches!(
        import(
            "{}\n".repeat(MAX_RECORDS + 1).as_bytes(),
            &options(InputFormat::Jsonl)
        ),
        Err(Error::TooManyRecords)
    ));
    assert!(matches!(
        import(&b""[..], &options(InputFormat::Json)),
        Err(Error::EmptySource)
    ));
}

#[test]
fn metadata_expansion_exceeding_case_budget_is_rejected_without_truncation() {
    let mut case = original();
    let finding = case.findings[0].clone();
    case.findings = (0..40)
        .map(|i| josh_core::Finding {
            id: format!("F{i}"),
            value: "x".repeat(150),
            ..finding.clone()
        })
        .collect();
    case.validate().unwrap();
    let batch = import(
        serde_json::to_vec(&case).unwrap().as_slice(),
        &options(InputFormat::Json),
    )
    .unwrap();
    assert!(batch.cases.is_empty());
    assert!(codes(&batch).contains(&IssueCode::CaseBudgetExceeded));
}

#[test]
fn all_unknown_or_not_tested_evidence_cannot_pass_review_gates() {
    let input = "case_id,data_class,finding_id,kind,name,value,status\nA,synthetic,F1,ihc,marker,not tested,not_tested\nA,synthetic,F2,ihc,other,unknown,unknown\n";
    let batch = import(input.as_bytes(), &options(InputFormat::Csv)).unwrap();
    assert!(!batch.cases[0].has_observed_evidence());
    assert_eq!(
        batch.cases[0].findings[0]
            .observation
            .as_ref()
            .unwrap()
            .status,
        ObservationStatus::NotTested
    );
    let result = josh_core::interpret(
        &batch.cases[0],
        josh_core::mock_response(),
        josh_core::Source::Jev,
    )
    .unwrap();
    assert!(result.reasons.contains(&"no_observed_findings"));
}

#[test]
fn bundle_roundtrip_is_private_refuses_overwrite_and_detects_changed_cases_or_sidecars() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("run");
    let mut batch = import(CSV.as_bytes(), &options(InputFormat::Csv)).unwrap();
    bundle::write(&root, &mut batch).unwrap();
    let report = bundle::read_report(&root).unwrap();
    for entry in &report.cases {
        bundle::read_case(&root, entry).unwrap();
    }
    assert!(bundle::write(&root, &mut batch).is_err());
    let mut case = bundle::read_case(&root, &report.cases[0]).unwrap();
    case.findings[0].value = "changed observation".into();
    std::fs::write(
        root.join("cases/DEMO-001.json"),
        serde_json::to_vec(&case).unwrap(),
    )
    .unwrap();
    assert!(bundle::read_case(&root, &report.cases[0]).is_err());
    std::fs::write(root.join("labels.json"), b"[]").unwrap();
    assert!(bundle::read_report(&root).is_err());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&root).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(root.join("manifest.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[test]
fn bundle_rejects_traversal_paths_and_incomplete_exports() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("run");
    let mut batch = import(CSV.as_bytes(), &options(InputFormat::Csv)).unwrap();
    bundle::write(&root, &mut batch).unwrap();
    let mut manifest = serde_json::to_value(&batch.report).unwrap();
    manifest["cases"][0]["case_id"] = "../../escape".into();
    std::fs::write(
        root.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    assert!(bundle::read_report(&root).is_err());
    assert!(bundle::read_report(dir.path()).is_err());
}
