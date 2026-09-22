use crate::{Options, Partition, SplitEntry, SplitManifest};
use nexus_core::Case;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub fn partitions(cases: &[Case], options: &Options) -> SplitManifest {
    // A held-out institution moves the WHOLE patient group, including other specimens.
    let held_out: BTreeSet<_> = cases
        .iter()
        .filter_map(|case| {
            let metadata = case.metadata.as_ref()?;
            if options.holdout_institution.is_some()
                && metadata.institution_id == options.holdout_institution
            {
                metadata.patient_id.clone()
            } else {
                None
            }
        })
        .collect();
    let entries = cases
        .iter()
        .map(|case| {
            let patient_id = case.metadata.as_ref().and_then(|m| m.patient_id.clone());
            let partition = match &patient_id {
                None => Partition::Unassigned,
                Some(id) if held_out.contains(id) => Partition::Test,
                Some(id) => {
                    let hash = Sha256::digest(format!("{}\0{id}", options.split_seed));
                    let bucket =
                        u64::from_be_bytes(hash[..8].try_into().expect("SHA256 has 8 bytes"))
                            % 10_000;
                    match bucket {
                        0..6000 => Partition::Development,
                        6000..8000 => Partition::Calibration,
                        _ => Partition::Test,
                    }
                }
            };
            SplitEntry {
                case_id: case.case_id.clone(),
                patient_id,
                partition,
            }
        })
        .collect();
    SplitManifest {
        version: 1,
        algorithm: "sha256-patient-60-20-20-v1".into(),
        seed: options.split_seed.clone(),
        holdout_institution: options.holdout_institution.clone(),
        entries,
    }
}
