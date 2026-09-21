use nexus_core::{Case, DataClass};
use nexus_jev::{Error, classify};

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
