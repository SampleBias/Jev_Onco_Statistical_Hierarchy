//! Private new-directory exports. The completion manifest is always written last.
use crate::*;
use nexus_core::{Case, prepare, provenance, valid_id, valid_sha256};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

fn directory(path: &Path) -> Result<(), Error> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    Ok(())
}

fn new_file(path: &Path) -> Result<File, Error> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    Ok(options.open(path)?)
}

fn json_file(path: &Path, value: &impl serde::Serialize) -> Result<String, Error> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|_| Error::InvalidBundle)?;
    bytes.push(b'\n');
    new_file(path)?.write_all(&bytes)?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

pub fn write(root: &Path, bundle: &mut ImportBundle) -> Result<(), Error> {
    // Never merge with or overwrite an existing directory, including an empty one.
    directory(root)?;
    directory(&root.join("cases"))?;
    let mut jsonl = new_file(&root.join("cases.jsonl"))?;
    let mut digest = Sha256::new();
    for case in &bundle.cases {
        if !valid_id(&case.case_id) {
            return Err(Error::InvalidBundle);
        }
        let mut bytes = serde_json::to_vec(case).map_err(|_| Error::InvalidBundle)?;
        new_file(&root.join("cases").join(format!("{}.json", case.case_id)))?.write_all(&bytes)?;
        bytes.push(b'\n');
        jsonl.write_all(&bytes)?;
        digest.update(bytes);
    }
    jsonl.flush()?;
    bundle
        .report
        .artifacts_sha256
        .insert("cases.jsonl".into(), format!("{:x}", digest.finalize()));
    for (name, hash) in [
        (
            "labels.json",
            json_file(&root.join("labels.json"), &bundle.labels)?,
        ),
        (
            "splits.json",
            json_file(&root.join("splits.json"), &bundle.splits)?,
        ),
    ] {
        bundle.report.artifacts_sha256.insert(name.into(), hash);
    }
    json_file(&root.join("manifest.json"), &bundle.report)?;
    Ok(())
}

fn regular(path: &Path) -> Result<File, Error> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(Error::InvalidBundle);
    }
    Ok(File::open(path)?)
}

pub fn read_report(root: &Path) -> Result<ImportReport, Error> {
    let mut bytes = Vec::new();
    regular(&root.join("manifest.json"))?
        .take((MAX_MANIFEST_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(Error::InvalidBundle);
    }
    let report: ImportReport = serde_json::from_slice(&bytes).map_err(|_| Error::InvalidBundle)?;
    let mut ids = BTreeSet::new();
    if report.manifest_version != 1
        || report.importer_version != IMPORTER_VERSION
        || report.accepted_cases != report.cases.len()
        || report.cases.len() > MAX_CASES
        || report.source.records > MAX_RECORDS
        || report.source.bytes > MAX_SOURCE_BYTES
        || !valid_id(&report.source.source_id)
        || !valid_sha256(&report.source.sha256)
        || report.accepted_records.checked_add(report.rejected_records)
            != Some(report.source.records)
    {
        return Err(Error::InvalidBundle);
    }
    for entry in &report.cases {
        if !valid_id(&entry.case_id)
            || !ids.insert(&entry.case_id)
            || !valid_sha256(&entry.case_revision_sha256)
            || !valid_sha256(&entry.request_sha256)
        {
            return Err(Error::InvalidBundle);
        }
    }
    let expected: BTreeSet<_> = ["cases.jsonl", "labels.json", "splits.json"]
        .into_iter()
        .collect();
    if report
        .artifacts_sha256
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != expected
    {
        return Err(Error::InvalidBundle);
    }
    for (name, expected_hash) in &report.artifacts_sha256 {
        let mut file = regular(&root.join(name))?;
        let mut digest = Sha256::new();
        let mut total = 0u64;
        let mut buffer = [0u8; 65_536];
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            total += count as u64;
            if total > MAX_SOURCE_BYTES {
                return Err(Error::InvalidBundle);
            }
            digest.update(&buffer[..count]);
        }
        if format!("{:x}", digest.finalize()) != *expected_hash {
            return Err(Error::InvalidBundle);
        }
    }
    Ok(report)
}

pub fn read_case(root: &Path, entry: &CaseEntry) -> Result<Case, Error> {
    if !valid_id(&entry.case_id)
        || !fs::symlink_metadata(root.join("cases"))?
            .file_type()
            .is_dir()
    {
        return Err(Error::InvalidBundle);
    }
    let mut bytes = Vec::new();
    regular(&root.join("cases").join(format!("{}.json", entry.case_id)))?
        .take((nexus_core::MAX_CASE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > nexus_core::MAX_CASE_BYTES {
        return Err(Error::InvalidBundle);
    }
    let case: Case = serde_json::from_slice(&bytes).map_err(|_| Error::InvalidBundle)?;
    let revision = provenance::case_revision(&case).map_err(|_| Error::InvalidBundle)?;
    let request = prepare(&case).map_err(|_| Error::InvalidBundle)?;
    if case.case_id != entry.case_id
        || revision != entry.case_revision_sha256
        || provenance::request_sha256(&request).map_err(|_| Error::InvalidBundle)?
            != entry.request_sha256
    {
        return Err(Error::InvalidBundle);
    }
    Ok(case)
}
