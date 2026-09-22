use josh_core::{Case, DataClass};

#[tokio::test]
async fn unobserved_schema_two_evidence_fails_before_network() {
    let mut case = case();
    case.schema_version = 2;
    case.metadata = Some(josh_core::CaseMetadata::default());
    for finding in &mut case.findings {
        finding.value = "not tested".into();
        finding.observation = Some(josh_core::Observation {
            status: josh_core::ObservationStatus::NotTested,
            units: None,
            assay: None,
            timepoint: None,
            reference_build: None,
        });
        finding.source = Some(josh_core::SourceReference {
            source_id: "synthetic".into(),
            source_sha256: "a".repeat(64),
            record: 1,
            field: "finding".into(),
        });
    }
    case.validate().unwrap();
    assert!(matches!(
        josh_jev::classify(&case, "synthetic-test-key").await,
        Err(josh_jev::Error::Validation(_))
    ));
}
use josh_jev::{Error, classify};

fn case() -> Case {
    serde_json::from_str(include_str!("../../../fixtures/synthetic-case.json")).unwrap()
}

#[tokio::test]
async fn research_data_cannot_leave_via_initial_adapter() {
    let mut case = case();
    case.data_class = DataClass::DeidentifiedResearch;
    assert!(matches!(
        classify(&case, "dummy-never-sent").await,
        Err(Error::DataPolicy)
    ));
}

#[tokio::test]
async fn missing_key_and_empty_evidence_fail_locally() {
    assert!(matches!(
        classify(&case(), " ").await,
        Err(Error::MissingKey)
    ));
    let mut case = case();
    case.findings.clear();
    assert!(matches!(
        classify(&case, "dummy-never-sent").await,
        Err(Error::Validation(_))
    ));
}
