use crate::{IssueCode, SourceManifest};
use josh_core::*;
use std::collections::BTreeMap;

pub const REQUIRED: &[&str] = &[
    "case_id",
    "data_class",
    "finding_id",
    "kind",
    "name",
    "value",
];
pub const COLUMNS: &[&str] = &[
    "case_id",
    "patient_id",
    "sample_id",
    "institution_id",
    "data_class",
    "age_years",
    "sex_at_birth",
    "specimen_site",
    "evidence_cutoff",
    "finding_id",
    "kind",
    "name",
    "value",
    "status",
    "units",
    "assay",
    "timepoint",
    "reference_build",
    "known_primary",
];

pub fn parse(
    row: &csv::StringRecord,
    headers: &csv::StringRecord,
    source: &SourceManifest,
    record: u64,
) -> Result<(Case, Option<String>), IssueCode> {
    let values: BTreeMap<_, _> = headers.iter().zip(row.iter()).collect();
    let get = |key: &str| values.get(key).copied().unwrap_or("");
    let optional = |key: &str| {
        let value = get(key);
        if value.is_empty() {
            None
        } else {
            Some(value.to_owned())
        }
    };
    let (age_years, age_lower_bound_exclusive) = age(get("age_years"))?;
    let data_class = match get("data_class") {
        "synthetic" => DataClass::Synthetic,
        "deidentified_research" => DataClass::DeidentifiedResearch,
        _ => return Err(IssueCode::InvalidRow),
    };
    let sex_at_birth = match get("sex_at_birth") {
        "" => None,
        "female" => Some(SexAtBirth::Female),
        "male" => Some(SexAtBirth::Male),
        "intersex" => Some(SexAtBirth::Intersex),
        "unknown" => Some(SexAtBirth::Unknown),
        _ => return Err(IssueCode::InvalidRow),
    };
    let kind = match get("kind") {
        "histology" => EvidenceKind::Histology,
        "ihc" => EvidenceKind::Ihc,
        "molecular" => EvidenceKind::Molecular,
        "clinical" => EvidenceKind::Clinical,
        _ => return Err(IssueCode::InvalidRow),
    };
    let inferred = categorical_status(get("value"));
    let status = match get("status") {
        "" => inferred.unwrap_or(ObservationStatus::Observed),
        "observed" => ObservationStatus::Observed,
        "positive" => ObservationStatus::Positive,
        "negative" => ObservationStatus::Negative,
        "equivocal" => ObservationStatus::Equivocal,
        "not_tested" => ObservationStatus::NotTested,
        "unknown" => ObservationStatus::Unknown,
        _ => return Err(IssueCode::InvalidStatus),
    };
    if inferred.is_some_and(|s| s != status) {
        return Err(IssueCode::ConflictingStatus);
    }
    let case = Case {
        schema_version: 2,
        clinical: None,
        case_id: get("case_id").into(),
        data_class,
        age_years,
        age_lower_bound_exclusive,
        sex_at_birth,
        specimen_site: optional("specimen_site"),
        metadata: Some(CaseMetadata {
            patient_id: optional("patient_id"),
            sample_id: optional("sample_id"),
            institution_id: optional("institution_id"),
            evidence_cutoff: optional("evidence_cutoff"),
        }),
        findings: vec![Finding {
            id: get("finding_id").into(),
            kind,
            name: get("name").into(),
            value: get("value").into(),
            observation: Some(Observation {
                status,
                units: optional("units"),
                assay: optional("assay"),
                timepoint: optional("timepoint"),
                reference_build: optional("reference_build"),
            }),
            source: Some(SourceReference {
                source_id: source.source_id.clone(),
                source_sha256: source.sha256.clone(),
                record,
                field: "value".into(),
            }),
        }],
    };
    case.validate().map_err(|_| IssueCode::InvalidCase)?;
    let label = optional("known_primary");
    if label
        .as_ref()
        .is_some_and(|label| !taxonomy().contains_key(label) || label == "insufficient_evidence")
    {
        return Err(IssueCode::InvalidLabel);
    }
    Ok((case, label))
}

fn age(value: &str) -> Result<(Option<u8>, Option<u8>), IssueCode> {
    if value.is_empty() {
        return Ok((None, None));
    }
    let (bound, digits) = value
        .strip_prefix('>')
        .map_or((false, value), |rest| (true, rest));
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(IssueCode::InvalidAge);
    }
    let years: u8 = digits.parse().map_err(|_| IssueCode::InvalidAge)?;
    if years > 120 || (bound && years >= 120) {
        return Err(IssueCode::InvalidAge);
    }
    Ok(if bound {
        (None, Some(years))
    } else {
        (Some(years), None)
    })
}
