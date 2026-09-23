//! Experimental SBS96 counting and nonnegative least-squares refitting in Rust.
//! The catalogue and opportunity context are external, versioned research assets.
use josh_core::{
    ValidationError,
    molecular::{FeatureSet, FeatureValue},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Catalogue {
    pub version: String,
    pub reference_build: String,
    pub opportunity_profile: String,
    pub channels: Vec<String>,
    pub signatures: BTreeMap<String, Vec<f64>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Spectrum {
    pub channels: Vec<String>,
    pub counts: Vec<u64>,
    pub counted: usize,
    pub excluded_non_somatic_or_non_snv: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Fit {
    pub method: String,
    pub catalogue_version: String,
    pub opportunity_profile: String,
    pub mutation_count: u64,
    pub exposures: BTreeMap<String, f64>,
    pub reconstructed: Vec<f64>,
    pub relative_residual: f64,
    pub cosine_similarity: f64,
    pub iterations: usize,
    pub converged: bool,
    pub uncertainty: String,
    pub limitations: Vec<String>,
}
pub fn channels() -> Vec<String> {
    let mut labels = Vec::new();
    for (reference, alternate) in [
        ('C', 'A'),
        ('C', 'G'),
        ('C', 'T'),
        ('T', 'A'),
        ('T', 'C'),
        ('T', 'G'),
    ] {
        for left in ['A', 'C', 'G', 'T'] {
            for right in ['A', 'C', 'G', 'T'] {
                labels.push(format!("{left}[{reference}>{alternate}]{right}"));
            }
        }
    }
    labels
}
fn complement(base: u8) -> Option<u8> {
    match base {
        b'A' => Some(b'T'),
        b'C' => Some(b'G'),
        b'G' => Some(b'C'),
        b'T' => Some(b'A'),
        _ => None,
    }
}
pub fn channel(mut context: [u8; 3], mut alternate: u8) -> Result<usize, ValidationError> {
    if context.iter().any(|b| complement(*b).is_none())
        || complement(alternate).is_none()
        || context[1] == alternate
    {
        return Err(ValidationError("invalid SBS context or alternate base"));
    }
    if matches!(context[1], b'A' | b'G') {
        context = [
            complement(context[2]).unwrap(),
            complement(context[1]).unwrap(),
            complement(context[0]).unwrap(),
        ];
        alternate = complement(alternate).unwrap();
    }
    let substitution = match (context[1], alternate) {
        (b'C', b'A') => 0,
        (b'C', b'G') => 1,
        (b'C', b'T') => 2,
        (b'T', b'A') => 3,
        (b'T', b'C') => 4,
        (b'T', b'G') => 5,
        _ => return Err(ValidationError("invalid pyrimidine substitution")),
    };
    let base = |b| b"ACGT".iter().position(|x| *x == b).unwrap();
    Ok(substitution * 16 + base(context[0]) * 4 + base(context[2]))
}
pub fn count(
    set: &FeatureSet,
    build: &str,
    mut context: impl FnMut(&str, u64) -> Result<[u8; 3], ValidationError>,
) -> Result<Spectrum, ValidationError> {
    set.validate()?;
    let mut counts = vec![0; 96];
    let mut excluded = 0;
    let mut seen = BTreeSet::new();
    for f in &set.features {
        if let Some(FeatureValue::Mutation {
            chromosome,
            position,
            reference,
            alternate,
            somatic,
            ..
        }) = &f.value
        {
            if *somatic != Some(true) || reference.len() != 1 || alternate.len() != 1 {
                excluded += 1;
                continue;
            }
            if f.reference_build.as_deref() != Some(build) {
                return Err(ValidationError("variant and FASTA reference builds differ"));
            }
            if !seen.insert((chromosome, *position, reference, alternate)) {
                return Err(ValidationError("duplicate SNV in spectrum"));
            }
            let context = context(chromosome, *position)?;
            if context[1] != reference.as_bytes()[0] {
                return Err(ValidationError("reference allele does not match FASTA"));
            }
            counts[channel(context, alternate.as_bytes()[0])?] += 1;
        }
    }
    Ok(Spectrum {
        channels: channels(),
        counted: counts.iter().sum::<u64>() as usize,
        counts,
        excluded_non_somatic_or_non_snv: excluded,
    })
}
pub fn fit(
    spectrum: &Spectrum,
    catalogue: &Catalogue,
    opportunity: &str,
    min_mutations: u64,
) -> Result<Fit, ValidationError> {
    if spectrum.channels != channels()
        || catalogue.channels != channels()
        || spectrum.counts.len() != 96
        || spectrum.counts.iter().any(|n| *n > 1_000_000_000)
        || catalogue.version.trim().is_empty()
        || catalogue.version.len() > 128
        || !matches!(catalogue.reference_build.as_str(), "GRCh37" | "GRCh38")
        || catalogue.opportunity_profile != opportunity
        || opportunity.trim().is_empty()
        || catalogue.signatures.is_empty()
        || catalogue.signatures.len() > 128
        || min_mutations == 0
    {
        return Err(ValidationError(
            "incompatible signature spectrum, catalogue or opportunity profile",
        ));
    }
    let total: u64 = spectrum.counts.iter().sum();
    if total < min_mutations {
        return Err(ValidationError(
            "too few eligible mutations for configured signature QC",
        ));
    }
    if spectrum.counted != total as usize {
        return Err(ValidationError("spectrum count mismatch"));
    }
    for (name, column) in &catalogue.signatures {
        if !name.starts_with("SBS")
            || name.len() > 32
            || !name.bytes().all(|b| b.is_ascii_alphanumeric())
            || column.len() != 96
            || column.iter().any(|v| !v.is_finite() || *v < 0.0)
            || (column.iter().sum::<f64>() - 1.0).abs() > 1e-6
        {
            return Err(ValidationError(
                "signature columns must be nonnegative, named SBS and sum to one",
            ));
        }
    }
    let columns: Vec<_> = catalogue.signatures.values().collect();
    let n = columns.len();
    let y: Vec<_> = spectrum.counts.iter().map(|n| *n as f64).collect();
    let mut x = vec![0.0; n];
    let mut residual = y.clone();
    let mut iterations = 0;
    let mut converged = false;
    let norms: Vec<f64> = columns
        .iter()
        .map(|c| c.iter().map(|v| v * v).sum())
        .collect();
    for iteration in 0..5000 {
        let mut max_change = 0.0_f64;
        for j in 0..n {
            let gradient: f64 = columns[j].iter().zip(&residual).map(|(a, r)| a * r).sum();
            let next = (x[j] + gradient / norms[j]).max(0.0);
            let delta = next - x[j];
            x[j] = next;
            max_change = max_change.max(delta.abs());
            for (r, a) in residual.iter_mut().zip(columns[j]) {
                *r -= delta * a;
            }
        }
        iterations = iteration + 1;
        if max_change < 1e-9 * total as f64 {
            converged = true;
            break;
        }
    }
    let reconstruction: Vec<_> = y.iter().zip(&residual).map(|(y, r)| y - r).collect();
    let norm_y = y.iter().map(|v| v * v).sum::<f64>().sqrt();
    let norm_reconstruction = reconstruction.iter().map(|v| v * v).sum::<f64>().sqrt();
    let cosine = if norm_reconstruction > 0.0 {
        (y.iter()
            .zip(&reconstruction)
            .map(|(a, b)| a * b)
            .sum::<f64>()
            / (norm_y * norm_reconstruction))
            .clamp(-1.0, 1.0)
    } else {
        0.0
    };
    Ok(Fit {method:"rust-nnls-coordinate-descent-sbs96-v1".into(),catalogue_version:catalogue.version.clone(),opportunity_profile:opportunity.into(),mutation_count:total,exposures:catalogue.signatures.keys().cloned().zip(x).collect(),reconstructed:reconstruction,relative_residual:residual.iter().map(|v|v*v).sum::<f64>().sqrt()/norm_y,cosine_similarity:cosine,iterations,converged,uncertainty:"not_estimated; correlated catalogue columns may have nonunique exposures".into(),limitations:vec!["Experimental numerical refit; no validated panel-specific operating threshold.".into(),"Opportunity compatibility is declared by the supplied catalogue; no territory correction is inferred.".into(),"Exposure coefficients are not cancer probabilities and do not establish an exposure history.".into()]})
}
