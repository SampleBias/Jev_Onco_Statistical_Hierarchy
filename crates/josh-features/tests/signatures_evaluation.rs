use josh_features::{
    evaluation::{self, Record},
    signatures::*,
};
use std::collections::{BTreeMap, BTreeSet};
#[test]
fn sbs96_has_all_channels_and_reverse_complements_match() {
    assert_eq!(channels().iter().collect::<BTreeSet<_>>().len(), 96);
    assert_eq!(channel(*b"ACA", b'A').unwrap(), 0);
    assert_eq!(channel(*b"TGT", b'T').unwrap(), 0);
    assert!(channel(*b"ANA", b'A').is_err());
    assert!(channel(*b"ACA", b'C').is_err());
}
#[test]
fn native_nnls_recovers_known_nonnegative_mixture_and_rejects_incompatible_opportunity() {
    let mut a = vec![0.0; 96];
    a[0] = 1.0;
    let mut b = vec![0.0; 96];
    b[1] = 1.0;
    let cat = Catalogue {
        version: "invented-v1".into(),
        reference_build: "GRCh38".into(),
        opportunity_profile: "whole-genome-fixture".into(),
        channels: channels(),
        signatures: BTreeMap::from([("SBS1".into(), a), ("SBS2".into(), b)]),
    };
    let mut counts = vec![0; 96];
    counts[0] = 30;
    counts[1] = 70;
    let spectrum = Spectrum {
        channels: channels(),
        counts,
        counted: 100,
        excluded_non_somatic_or_non_snv: 0,
    };
    let f = fit(&spectrum, &cat, "whole-genome-fixture", 50).unwrap();
    assert!(f.converged);
    assert!((f.exposures["SBS1"] - 30.0).abs() < 1e-8);
    assert!((f.exposures["SBS2"] - 70.0).abs() < 1e-8);
    assert!(f.relative_residual < 1e-10);
    assert!((f.cosine_similarity - 1.0).abs() < 1e-10);
    assert!(fit(&spectrum, &cat, "targeted-panel", 50).is_err());
    assert!(fit(&spectrum, &cat, "whole-genome-fixture", 101).is_err());
}
fn record(id: &str, truth: &str, p: Option<(f64, f64)>, accepted: bool) -> Record {
    Record {
        sample_id: id.into(),
        patient_group_id: id.into(),
        partition: "test".into(),
        truth: truth.into(),
        probabilities: p.map(|(a, b)| BTreeMap::from([("a".into(), a), ("b".into(), b)])),
        accepted,
        failure: p.is_none().then(|| "provider_error".into()),
    }
}
#[test]
fn evaluation_keeps_failures_in_denominator_and_scores_calibration() {
    let classes = BTreeSet::from(["a".into(), "b".into()]);
    let rows = vec![
        record("1", "a", Some((0.8, 0.2)), true),
        record("2", "b", Some((0.6, 0.4)), false),
        record("3", "b", None, false),
    ];
    let r = evaluation::evaluate(&rows, &classes, &classes, "test").unwrap();
    assert_eq!(r.eligible, 3);
    assert_eq!(r.failed, 1);
    assert_eq!(r.accepted_accuracy, Some(1.0));
    assert!((r.top1_all_eligible - 1.0 / 3.0).abs() < 1e-12);
    assert!((r.brier.unwrap() - 0.4).abs() < 1e-12);
    assert!((r.ece.unwrap() - 0.4).abs() < 1e-12);
    assert_eq!(r.per_class["b"].count, 2);
    assert_eq!(r.confusion["b"]["__failed__"], 1);
    let mut leaked = rows;
    leaked[1].patient_group_id = "1".into();
    leaked[1].partition = "development".into();
    assert!(evaluation::evaluate(&leaked, &classes, &classes, "test").is_err());
}
