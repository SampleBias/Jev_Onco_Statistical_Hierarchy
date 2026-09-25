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
    let s = SAMPLES.get(index).ok_or("Unknown built-in sample")?;
    Ok(josh_ingest::molecular::import(
        s.bytes,
        Format::Tsv,
        &Options {
            sample_id: format!("SYNTH-{}", s.number),
            patient_group_id: format!("SYNTH-P{}", s.number),
            data_class: DataClass::Synthetic,
            source_id: format!("synthetic-jev-v1-{}", s.source),
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
        for (i, count) in [20, 11, 16, 8, 14].into_iter().enumerate() {
            let f = load(i).unwrap();
            f.validate().unwrap();
            assert_eq!(f.data_class, DataClass::Synthetic);
            assert_eq!(f.features.len(), count);
            let request =
                josh_core::molecular::prepare(&f, &josh_core::molecular::onconpc_taxonomy())
                    .unwrap();
            assert_eq!(request.model, josh_core::MODEL);
        }
        assert_eq!(
            load(0).unwrap().patient_group_id,
            load(1).unwrap().patient_group_id
        );
        assert_eq!(
            load(2).unwrap().patient_group_id,
            load(3).unwrap().patient_group_id
        );
        assert!(load(99).is_err());
    }
}
