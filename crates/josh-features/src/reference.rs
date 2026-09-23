//! Numerical comparison only. Jev remains the origin classifier.
use josh_core::{reference::*, sample::*};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ReferenceError(pub &'static str);

pub fn validate(r: &ReferenceRelease) -> Result<(), ReferenceError> {
    let bad = || ReferenceError("invalid, oversized or unsupported reference release");
    if r.schema_version != 1
        || r.pipeline_version != REFERENCE_PIPELINE
        || !valid_label(&r.release_id)
        || !valid_label(&r.citation)
        || !(2..=100).contains(&r.classes.len())
        || r.genes.len() > 100_000
        || r.genes.len() < r.minimum_genes
        || r.minimum_genes < 3
        || (!r.synthetic && r.minimum_genes < 100)
        || !r.minimum_overlap.is_finite()
        || !(0.5..=1.).contains(&r.minimum_overlap)
        || r.members.len() > 512
        || r.members.is_empty()
        || r.classes.len() * r.genes.len() > 1_000_000
        || !valid_label(&r.compatibility.platform)
        || r.compatibility.organism != "Homo sapiens"
        || !matches!(
            (r.compatibility.units, r.compatibility.transform),
            (ExpressionUnit::Tpm, Transform::Log2OnePlus)
        )
        || [
            &r.source_manifest_sha256,
            &r.source_input_sha256,
            &r.source_labels_sha256,
            &r.compatibility.gene_map_sha256,
        ]
        .iter()
        .any(|s| !josh_core::valid_sha256(s))
    {
        return Err(bad());
    }
    let mut genes = BTreeSet::new();
    for g in &r.genes {
        if !valid_label(&g.symbol) || !g.hgnc_id.starts_with("HGNC:") || !genes.insert(&g.hgnc_id) {
            return Err(bad());
        }
    }
    let mut classes = BTreeSet::new();
    let mut names = BTreeSet::new();
    for c in &r.classes {
        if !valid_label(&c.class_id)
            || !valid_label(&c.cancer_type)
            || !classes.insert(&c.class_id)
            || !names.insert(&c.cancer_type)
            || c.class_id == "unknown"
            || c.class_id == "other_origin"
            || c.samples == 0
            || c.mean_expression.len() != r.genes.len()
            || c.mean_expression
                .iter()
                .any(|v| !v.is_finite() || *v < 0. || *v > 1024.)
        {
            return Err(bad());
        }
        if r.members
            .iter()
            .filter(|m| m.class_id == c.class_id)
            .count()
            != c.samples
        {
            return Err(bad());
        }
    }
    let mut ids = BTreeSet::new();
    let mut groups = BTreeSet::new();
    for m in &r.members {
        if !valid_label(&m.sample_id)
            || !valid_label(&m.patient_group_id)
            || !ids.insert(&m.sample_id)
            || !groups.insert(&m.patient_group_id)
            || !classes.contains(&m.class_id)
            || !josh_core::valid_sha256(&m.measurement_sha256)
        {
            return Err(bad());
        }
    }
    Ok(())
}

/// Stable Pearson correlation with centering and scaling; constants are undefined.
pub fn pearson(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() != y.len() || x.len() < 3 || x.iter().chain(y).any(|v| !v.is_finite()) {
        return None;
    }
    let scale_x = x.iter().map(|v| v.abs()).fold(0., f64::max);
    let scale_y = y.iter().map(|v| v.abs()).fold(0., f64::max);
    if scale_x == 0. || scale_y == 0. {
        return None;
    }
    let n = x.len() as f64;
    let mx = x.iter().map(|v| v / scale_x / n).sum::<f64>();
    let my = y.iter().map(|v| v / scale_y / n).sum::<f64>();
    let (mut xx, mut yy, mut xy) = (0., 0., 0.);
    for (a, b) in x.iter().zip(y) {
        let a = a / scale_x - mx;
        let b = b / scale_y - my;
        xx += a * a;
        yy += b * b;
        xy += a * b;
    }
    if xx <= f64::EPSILON || yy <= f64::EPSILON {
        return None;
    }
    let r = xy / (xx.sqrt() * yy.sqrt());
    r.is_finite().then(|| r.clamp(-1., 1.))
}

pub fn measured(
    records: &[ExpressionRecord],
) -> Result<BTreeMap<String, (GeneIdentity, f64)>, ReferenceError> {
    let mut map = BTreeMap::new();
    for r in records {
        if let (Some(g), Some(v)) = (&r.mapping.gene, r.transformed_expression)
            && (!v.is_finite() || map.insert(g.hgnc_id.clone(), (g.clone(), v)).is_some())
        {
            return Err(ReferenceError(
                "duplicate canonical gene or nonfinite expression",
            ));
        }
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn correlation_is_signed_and_constants_are_undefined() {
        assert!((pearson(&[1., 2., 3.], &[10., 20., 30.]).unwrap() - 1.).abs() < 1e-12);
        assert!((pearson(&[1., 2., 3.], &[30., 20., 10.]).unwrap() + 1.).abs() < 1e-12);
        assert!(pearson(&[1., 1., 1.], &[1., 2., 3.]).is_none());
        assert!(pearson(&[f64::NAN, 2., 3.], &[1., 2., 3.]).is_none());
        assert!(
            (pearson(&[1e300, 2e300, 3e300], &[3e300, 2e300, 1e300]).unwrap() + 1.).abs() < 1e-12
        );
    }
}
