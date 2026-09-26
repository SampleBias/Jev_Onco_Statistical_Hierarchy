//! Built-in inputs use the real importer, not the analytical demo's fixed scores.
use crate::workflows::AppError;
use josh_core::{DataClass, molecular::FeatureSet};
use josh_ingest::molecular::{Format, Options};

pub struct Sample {
    pub name: &'static str,
    pub description: &'static str,
    number: &'static str,
    source: &'static str,
    bytes: &'static [u8],
}

pub const SAMPLES: &[Sample] = &[
    Sample {
        name: "001 · Nodal adenocarcinoma",
        description: "Targeted DNA and IHC; no primary on imaging.",
        number: "001",
        source: "sample-001",
        bytes: include_bytes!("../../../fixtures/cup-realistic-v2/sample-001.tsv"),
    },
    Sample {
        name: "002 · Hepatic gland-forming carcinoma",
        description: "Intestinal markers; incomplete endoscopy and MMR workup.",
        number: "002",
        source: "sample-002",
        bytes: include_bytes!("../../../fixtures/cup-realistic-v2/sample-002.tsv"),
    },
    Sample {
        name: "003 · Scant decalcified bone biopsy",
        description: "Low tumor content, failed stains and unresolved tumor-only variant.",
        number: "003",
        source: "sample-003",
        bytes: include_bytes!("../../../fixtures/cup-realistic-v2/sample-003.tsv"),
    },
    Sample {
        name: "004 · Axillary nodal presentation",
        description: "Hormone receptors and lineage markers; imaging unrevealing.",
        number: "004",
        source: "sample-004",
        bytes: include_bytes!("../../../fixtures/cup-realistic-v2/sample-004.tsv"),
    },
    Sample {
        name: "005 · Peritoneal high-grade carcinoma",
        description: "Overlapping gynecologic markers; exact site unresolved.",
        number: "005",
        source: "sample-005",
        bytes: include_bytes!("../../../fixtures/cup-realistic-v2/sample-005.tsv"),
    },
    Sample {
        name: "006 · Nodes and sclerotic bone lesions",
        description: "Variable staining intensity with a coherent lineage panel.",
        number: "006",
        source: "sample-006",
        bytes: include_bytes!("../../../fixtures/cup-realistic-v2/sample-006.tsv"),
    },
    Sample {
        name: "007 · Clear-cell soft-tissue metastasis",
        description: "Shared PAX8 marker; grouped mutation and copy-number evidence.",
        number: "007",
        source: "sample-007",
        bytes: include_bytes!("../../../fixtures/cup-realistic-v2/sample-007.tsv"),
    },
    Sample {
        name: "008 · Mucinous hepatic adenocarcinoma",
        description: "Pancreatic/biliary overlap; fusion testing unavailable.",
        number: "008",
        source: "sample-008",
        bytes: include_bytes!("../../../fixtures/cup-realistic-v2/sample-008.tsv"),
    },
    Sample {
        name: "009 · Discordant paired specimens",
        description: "Liver and node show incompatible patterns; clonality unresolved.",
        number: "009",
        source: "sample-009",
        bytes: include_bytes!("../../../fixtures/cup-realistic-v2/sample-009.tsv"),
    },
    Sample {
        name: "010 · Trabecular nodal carcinoma",
        description: "Differentiation panel exercises taxonomy coverage.",
        number: "010",
        source: "sample-010",
        bytes: include_bytes!("../../../fixtures/cup-realistic-v2/sample-010.tsv"),
    },
];

/// Frozen V1 cases for historical paired experiments; not extra patients in the picker.
pub const LEGACY_SAMPLES: &[Sample] = &[
    Sample {
        name: "001 · Full molecular + pathology",
        description: "20 observations · invented lung-like pattern. No prediction included.",
        number: "001",
        source: "sample-001",
        bytes: include_bytes!("../../../fixtures/synthetic-jev/sample-001.tsv"),
    },
    Sample {
        name: "001 · Genomic-only comparison",
        description: "11 observations · same patient, without pathology. No prediction included.",
        number: "001",
        source: "sample-001-genomic",
        bytes: include_bytes!("../../../fixtures/synthetic-jev/sample-001-genomic.tsv"),
    },
    Sample {
        name: "002 · Full molecular + pathology",
        description: "16 observations · invented colorectal-like pattern. No prediction included.",
        number: "002",
        source: "sample-002",
        bytes: include_bytes!("../../../fixtures/synthetic-jev/sample-002.tsv"),
    },
    Sample {
        name: "002 · Genomic-only comparison",
        description: "8 observations · same patient, without pathology. No prediction included.",
        number: "002",
        source: "sample-002-genomic",
        bytes: include_bytes!("../../../fixtures/synthetic-jev/sample-002-genomic.tsv"),
    },
    Sample {
        name: "003 · Sparse / ambiguous evidence",
        description: "14 observations · 8 unavailable. Exercise uncertainty, not a guaranteed result.",
        number: "003",
        source: "sample-003",
        bytes: include_bytes!("../../../fixtures/synthetic-jev/sample-003.tsv"),
    },
];

pub fn load(index: usize) -> Result<FeatureSet, AppError> {
    load_from(SAMPLES, index, "synthetic-cup-v2")
}

pub fn load_legacy(index: usize) -> Result<FeatureSet, AppError> {
    load_from(LEGACY_SAMPLES, index, "synthetic-jev-v1")
}

fn load_from(samples: &[Sample], index: usize, collection: &str) -> Result<FeatureSet, AppError> {
    let s = samples.get(index).ok_or("Unknown built-in sample")?;
    Ok(josh_ingest::molecular::import(
        s.bytes,
        Format::Tsv,
        &Options {
            sample_id: format!("SYNTH-{}", s.number),
            patient_group_id: format!("SYNTH-P{}", s.number),
            data_class: DataClass::Synthetic,
            source_id: format!("{collection}-{}", s.source),
            assay: "synthetic-assay-v1".into(),
            reference_build: None,
        },
    )?
    .features)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_sample_is_valid_synthetic_input_with_no_cwd_or_provider_dependency() {
        let mut patients = std::collections::BTreeSet::new();
        assert_eq!(SAMPLES.len(), 10);
        for (i, count) in [23, 24, 19, 24, 24, 23, 23, 24, 21, 22]
            .into_iter()
            .enumerate()
        {
            let f = load(i).unwrap();
            f.validate().unwrap();
            assert_eq!(f.data_class, DataClass::Synthetic);
            assert_eq!(f.features.len(), count);
            assert!(patients.insert(f.patient_group_id.clone()));
            let request =
                josh_core::molecular::prepare(&f, &josh_core::molecular::onconpc_taxonomy())
                    .unwrap();
            assert_eq!(request.model, josh_core::MODEL);
            let state = serde_json::to_string(&request.state).unwrap();
            for forbidden in [
                &f.sample_id,
                &f.patient_group_id,
                &f.features[0].source.sha256,
            ] {
                assert!(!state.contains(forbidden));
            }
            assert!(f.features.iter().any(|v| v.name == "Workup context"));
            assert!(f.features.iter().all(|v| v.assay.starts_with("synthetic-")));
        }
        assert!(load(10).is_err());
    }

    #[test]
    fn legacy_pairs_remain_available_for_frozen_experiments() {
        for (i, count) in [20, 11, 16, 8, 14].into_iter().enumerate() {
            assert_eq!(load_legacy(i).unwrap().features.len(), count);
        }
        assert_eq!(
            load_legacy(0).unwrap().patient_group_id,
            load_legacy(1).unwrap().patient_group_id
        );
        assert_eq!(
            load_legacy(2).unwrap().patient_group_id,
            load_legacy(3).unwrap().patient_group_id
        );
        assert!(load(99).is_err());
    }
}
