use crate::*;
use nexus_core::{
    Case, CaseMetadata, Observation, ObservationStatus, SourceReference, categorical_status,
    prepare, provenance, valid_id,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
};

/// Snapshot and hash once before parsing. The bounded private temporary file prevents
/// source mutation between hashing and parsing and avoids loading a whole source in RAM.
pub fn import(mut input: impl Read, options: &Options) -> Result<ImportBundle, Error> {
    if !valid_id(&options.source_id)
        || !valid_id(&options.split_seed)
        || options
            .holdout_institution
            .as_ref()
            .is_some_and(|s| !valid_id(s))
    {
        return Err(Error::Options);
    }
    let mut snapshot = tempfile::tempfile()?;
    let mut digest = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 65_536];
    loop {
        let size = input.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        bytes += size as u64;
        if bytes > MAX_SOURCE_BYTES {
            return Err(Error::SourceTooLarge);
        }
        digest.update(&buffer[..size]);
        snapshot.write_all(&buffer[..size])?;
    }
    if bytes == 0 {
        return Err(Error::EmptySource);
    }
    snapshot.seek(SeekFrom::Start(0))?;
    let source = SourceManifest {
        source_id: options.source_id.clone(),
        sha256: format!("{:x}", digest.finalize()),
        format: options.format,
        bytes,
        records: 0,
    };
    let mut builder = Builder {
        source,
        cases: BTreeMap::new(),
        invalid: BTreeSet::new(),
        issues: vec![],
        warnings: vec![],
    };
    match options.format {
        InputFormat::Json => {
            let mut data = Vec::new();
            snapshot
                .take((MAX_RECORD_BYTES + 1) as u64)
                .read_to_end(&mut data)?;
            builder.source.records = 1;
            if data.len() > MAX_RECORD_BYTES {
                builder.reject(1, IssueCode::RecordTooLarge, None);
            } else {
                builder.json(&data, 1)?;
            }
        }
        InputFormat::Jsonl => {
            let mut reader = BufReader::new(snapshot);
            while let Some((data, oversized)) = bounded_line(&mut reader)? {
                builder.source.records += 1;
                if builder.source.records > MAX_RECORDS {
                    return Err(Error::TooManyRecords);
                }
                let record = builder.source.records as u64;
                if oversized {
                    builder.reject(record, IssueCode::RecordTooLarge, None);
                } else {
                    builder.json(&data, record)?;
                }
            }
        }
        InputFormat::Csv | InputFormat::Tsv => {
            let delimiter = if matches!(options.format, InputFormat::Tsv) {
                b'\t'
            } else {
                b','
            };
            let mut reader = csv::ReaderBuilder::new()
                .delimiter(delimiter)
                .flexible(true)
                .from_reader(snapshot);
            let headers = reader.headers().map_err(|_| Error::Columns)?.clone();
            let unique: BTreeSet<_> = headers.iter().collect();
            if unique.len() != headers.len()
                || headers.iter().any(|s| !tabular::COLUMNS.contains(&s))
                || tabular::REQUIRED.iter().any(|s| !unique.contains(s))
            {
                return Err(Error::Columns);
            }
            let id_column = headers
                .iter()
                .position(|s| s == "case_id")
                .ok_or(Error::Columns)?;
            for row in reader.byte_records() {
                builder.source.records += 1;
                if builder.source.records > MAX_RECORDS {
                    return Err(Error::TooManyRecords);
                }
                let record = builder.source.records as u64 + 1;
                let row = row.map_err(|_| Error::Columns)?;
                let id = row
                    .get(id_column)
                    .and_then(|v| std::str::from_utf8(v).ok())
                    .filter(|id| valid_id(id))
                    .map(str::to_owned);
                if row.len() != headers.len() || row.as_slice().len() > MAX_RECORD_BYTES {
                    let code = if row.as_slice().len() > MAX_RECORD_BYTES {
                        IssueCode::RecordTooLarge
                    } else {
                        IssueCode::InvalidRow
                    };
                    builder.reject(record, code, id.as_deref());
                    continue;
                }
                let Ok(row) = csv::StringRecord::from_byte_record(row) else {
                    builder.reject(record, IssueCode::InvalidRow, id.as_deref());
                    continue;
                };
                match tabular::parse(&row, &headers, &builder.source, record) {
                    Ok((case, label)) => builder.tabular(case, label, record)?,
                    Err(code) => builder.reject(record, code, id.as_deref()),
                }
            }
        }
    }
    if builder.source.records == 0 {
        return Err(Error::EmptySource);
    }
    builder.finish(options)
}

/// Migrate the existing normalized document, retaining all original observations.
/// An absent patient ID stays absent; it must not become an invented split group.
pub fn migrate(mut case: Case, source: &SourceManifest, record: u64) -> Result<Case, IssueCode> {
    case.validate().map_err(|_| IssueCode::InvalidCase)?;
    if case.schema_version == 1 {
        case.schema_version = 2;
        case.metadata = Some(CaseMetadata::default());
        for (index, finding) in case.findings.iter_mut().enumerate() {
            finding.observation = Some(Observation {
                status: categorical_status(&finding.value).unwrap_or(ObservationStatus::Observed),
                units: None,
                assay: None,
                timepoint: None,
                reference_build: None,
            });
            finding.source = Some(SourceReference {
                source_id: source.source_id.clone(),
                source_sha256: source.sha256.clone(),
                record,
                field: format!("findings[{index}]"),
            });
        }
    }
    if serde_json::to_vec(&case).expect("case serializes").len() > nexus_core::MAX_CASE_BYTES {
        return Err(IssueCode::CaseBudgetExceeded);
    }
    case.validate().map_err(|_| IssueCode::InvalidCase)?;
    Ok(case)
}

fn bounded_line(reader: &mut impl BufRead) -> Result<Option<(Vec<u8>, bool)>, Error> {
    let mut data = Vec::new();
    let mut oversized = false;
    let mut seen = false;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            break;
        }
        seen = true;
        let end = available.iter().position(|b| *b == b'\n');
        let count = end.map_or(available.len(), |i| i + 1);
        if data.len() + count > MAX_RECORD_BYTES + 2 {
            oversized = true;
        }
        if !oversized {
            data.extend_from_slice(&available[..count]);
        }
        reader.consume(count);
        if end.is_some() {
            break;
        }
    }
    if !seen {
        return Ok(None);
    }
    if data.last() == Some(&b'\n') {
        data.pop();
    }
    if data.last() == Some(&b'\r') {
        data.pop();
    }
    oversized |= data.len() > MAX_RECORD_BYTES;
    Ok(Some((data, oversized)))
}

struct Pending {
    case: Case,
    records: Vec<u64>,
    label: Option<String>,
}
struct Builder {
    source: SourceManifest,
    cases: BTreeMap<String, Pending>,
    invalid: BTreeSet<String>,
    issues: Vec<Issue>,
    warnings: Vec<Issue>,
}

impl Builder {
    fn reject(&mut self, record: u64, code: IssueCode, id: Option<&str>) {
        self.issues.push(Issue { record, code });
        if let Some(id) = id.filter(|id| valid_id(id)) {
            self.invalid.insert(id.into());
        }
    }

    fn json(&mut self, data: &[u8], record: u64) -> Result<(), Error> {
        let case: Case = match serde_json::from_slice(data) {
            Ok(case) => case,
            Err(_) => {
                // Recover only a valid case ID to invalidate other rows with that ID.
                let value = serde_json::from_slice::<serde_json::Value>(data).ok();
                let id = value
                    .as_ref()
                    .and_then(|v| v.get("case_id"))
                    .and_then(|v| v.as_str());
                self.reject(record, IssueCode::InvalidJson, id);
                return Ok(());
            }
        };
        let id = case.case_id.clone();
        let case = match migrate(case, &self.source, record) {
            Ok(case) => case,
            Err(code) => {
                self.reject(record, code, Some(&id));
                return Ok(());
            }
        };
        if self.cases.contains_key(&id) {
            self.reject(record, IssueCode::DuplicateCase, Some(&id));
            return Ok(());
        }
        if self.cases.len() >= MAX_CASES {
            return Err(Error::TooManyRecords);
        }
        self.cases.insert(
            id,
            Pending {
                case,
                records: vec![record],
                label: None,
            },
        );
        Ok(())
    }

    fn tabular(&mut self, case: Case, label: Option<String>, record: u64) -> Result<(), Error> {
        let id = case.case_id.clone();
        if let Some(pending) = self.cases.get_mut(&id) {
            pending.records.push(record);
            let code = if !same_metadata(&pending.case, &case) {
                Some(IssueCode::ConflictingMetadata)
            } else if pending.label != label {
                Some(IssueCode::ConflictingLabel)
            } else if pending
                .case
                .findings
                .iter()
                .any(|f| f.id == case.findings[0].id)
            {
                Some(IssueCode::DuplicateFinding)
            } else if pending.case.findings.len() >= 64 {
                Some(IssueCode::CaseBudgetExceeded)
            } else {
                None
            };
            if let Some(code) = code {
                self.reject(record, code, Some(&id));
            } else {
                pending.case.findings.extend(case.findings);
            }
        } else {
            if self.cases.len() >= MAX_CASES {
                return Err(Error::TooManyRecords);
            }
            self.cases.insert(
                id,
                Pending {
                    case,
                    records: vec![record],
                    label,
                },
            );
        }
        Ok(())
    }

    fn finish(mut self, options: &Options) -> Result<ImportBundle, Error> {
        let mut samples: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (id, pending) in &self.cases {
            if let Some(sample) = pending
                .case
                .metadata
                .as_ref()
                .and_then(|m| m.sample_id.as_ref())
            {
                samples.entry(sample.clone()).or_default().push(id.clone());
            }
        }
        for ids in samples.values().filter(|ids| ids.len() > 1) {
            for id in ids {
                self.reject(
                    self.cases[id].records[0],
                    IssueCode::DuplicateSample,
                    Some(id),
                );
            }
        }
        let mut cases = vec![];
        let mut labels = vec![];
        let mut entries = vec![];
        let mut accepted_records = 0;
        let mut observation_counts = BTreeMap::new();
        let mut missingness = BTreeMap::new();
        let mut patient_records: BTreeMap<String, Vec<u64>> = BTreeMap::new();
        let pending_cases = std::mem::take(&mut self.cases);
        for (id, pending) in pending_cases {
            let first = pending.records[0];
            if self.invalid.contains(&id) {
                self.issues.push(Issue {
                    record: first,
                    code: IssueCode::InvalidatedCase,
                });
                continue;
            }
            let case = pending.case;
            if serde_json::to_vec(&case).expect("case serializes").len()
                > nexus_core::MAX_CASE_BYTES
            {
                self.reject(first, IssueCode::CaseBudgetExceeded, Some(&id));
                continue;
            }
            let request = match prepare(&case) {
                Ok(request) => request,
                Err(_) => {
                    self.reject(first, IssueCode::InvalidCase, Some(&id));
                    continue;
                }
            };
            let state_bytes = serde_json::to_vec(&request.state)
                .expect("state serializes")
                .len();
            let request_bytes = serde_json::to_vec(&request)
                .expect("request serializes")
                .len();
            if request_bytes > MAX_REQUEST_BYTES {
                self.reject(first, IssueCode::RequestBudgetExceeded, Some(&id));
                continue;
            }
            let metadata = case.metadata.as_ref().expect("migrated case has metadata");
            if let Some(patient) = &metadata.patient_id {
                patient_records
                    .entry(patient.clone())
                    .or_default()
                    .push(first);
            } else {
                self.warnings.push(Issue {
                    record: first,
                    code: IssueCode::MissingPatientGroup,
                });
            }
            for (field, missing) in [
                (
                    "age",
                    case.age_years.is_none() && case.age_lower_bound_exclusive.is_none(),
                ),
                (
                    "sex_at_birth",
                    case.sex_at_birth.is_none()
                        || case.sex_at_birth == Some(nexus_core::SexAtBirth::Unknown),
                ),
                ("specimen_site", case.specimen_site.is_none()),
                ("patient_id", metadata.patient_id.is_none()),
                ("sample_id", metadata.sample_id.is_none()),
                ("evidence_cutoff", metadata.evidence_cutoff.is_none()),
            ] {
                if missing {
                    increment(&mut missingness, field.into());
                }
            }
            for (index, finding) in case.findings.iter().enumerate() {
                let observation = finding
                    .observation
                    .as_ref()
                    .expect("migrated finding has observation");
                increment(
                    &mut observation_counts,
                    serde_json::to_value(observation.status)
                        .expect("status serializes")
                        .as_str()
                        .unwrap()
                        .into(),
                );
                if observation.status == ObservationStatus::Unknown {
                    self.warnings.push(Issue {
                        record: pending.records.get(index).copied().unwrap_or(first),
                        code: IssueCode::UnknownObservation,
                    });
                }
            }
            entries.push(CaseEntry {
                case_id: id.clone(),
                case_revision_sha256: provenance::case_revision(&case).expect("validated case"),
                request_sha256: provenance::request_sha256(&request).expect("validated request"),
                state_bytes,
                request_bytes,
            });
            if let Some(primary_origin) = pending.label {
                labels.push(LabelRecord {
                    case_id: id,
                    primary_origin,
                    taxonomy_version: nexus_core::TAXONOMY_VERSION.into(),
                });
            }
            accepted_records += pending.records.len();
            cases.push(case);
        }
        for records in patient_records.values().filter(|records| records.len() > 1) {
            self.warnings.push(Issue {
                record: records[0],
                code: IssueCode::MultipleSpecimens,
            });
        }
        let splits = split::partitions(&cases, options);
        let mut label_counts = BTreeMap::new();
        let mut split_counts = BTreeMap::new();
        for label in &labels {
            increment(&mut label_counts, label.primary_origin.clone());
        }
        for entry in &splits.entries {
            increment(
                &mut split_counts,
                serde_json::to_value(entry.partition)
                    .expect("partition serializes")
                    .as_str()
                    .unwrap()
                    .into(),
            );
        }
        let rejected_records = self.source.records - accepted_records;
        let status = if cases.is_empty() {
            ImportStatus::Rejected
        } else if rejected_records > 0 {
            ImportStatus::Partial
        } else {
            ImportStatus::Complete
        };
        self.issues.sort_by_key(|issue| issue.record);
        self.warnings.sort_by_key(|issue| issue.record);
        let report = ImportReport {
            manifest_version: 1,
            importer_version: IMPORTER_VERSION.into(),
            status,
            source: self.source,
            accepted_cases: cases.len(),
            rejected_cases: self.invalid.len(),
            accepted_records,
            rejected_records,
            issues: self.issues,
            warnings: self.warnings,
            observation_counts,
            missingness,
            label_counts,
            split_counts,
            token_budget_verified: false,
            cases: entries,
            artifacts_sha256: BTreeMap::new(),
        };
        Ok(ImportBundle {
            report,
            cases,
            labels,
            splits,
        })
    }
}

fn same_metadata(a: &Case, b: &Case) -> bool {
    a.data_class == b.data_class
        && a.age_years == b.age_years
        && a.age_lower_bound_exclusive == b.age_lower_bound_exclusive
        && a.sex_at_birth == b.sex_at_birth
        && a.specimen_site == b.specimen_site
        && a.metadata == b.metadata
}
fn increment(counts: &mut BTreeMap<String, usize>, key: String) {
    *counts.entry(key).or_default() += 1;
}
