//! Indexed FASTA context extraction and reproducible experimental signature output.
use crate::workflows::{self, AppError};
use josh_core::{ValidationError, molecular::*};
use josh_features::signatures::{self, Catalogue, Fit, Spectrum};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    path::Path,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignatureRun {
    pub schema_version: u32,
    pub feature_sha256: String,
    pub fasta_sha256: String,
    pub fai_sha256: String,
    pub catalogue: Catalogue,
    pub min_mutations: u64,
    pub spectrum: Spectrum,
    pub fit: Fit,
}
struct Index {
    length: u64,
    offset: u64,
    line_bases: u64,
    line_bytes: u64,
}
pub struct Fasta {
    file: File,
    index: BTreeMap<String, Index>,
}
impl Fasta {
    pub fn open(path: &Path, index_path: &Path) -> Result<Self, AppError> {
        let file = File::open(path)?;
        let index_file = File::open(index_path)?;
        if index_file.metadata()?.len() > 1024 * 1024 {
            return Err(ValidationError("FASTA index too large").into());
        }
        let mut index = BTreeMap::new();
        for line in BufReader::new(index_file).lines() {
            let line = line?;
            let fields: Vec<_> = line.split('\t').collect();
            if fields.len() != 5 {
                return Err(ValidationError("invalid FASTA index").into());
            }
            let number = |i: usize| {
                fields[i]
                    .parse::<u64>()
                    .map_err(|_| ValidationError("invalid FASTA index coordinate"))
            };
            let row = Index {
                length: number(1)?,
                offset: number(2)?,
                line_bases: number(3)?,
                line_bytes: number(4)?,
            };
            if row.length == 0
                || row.line_bases == 0
                || row.line_bytes < row.line_bases
                || row.line_bytes - row.line_bases > 2
                || row.offset >= file.metadata()?.len()
                || index.insert(fields[0].to_string(), row).is_some()
            {
                return Err(ValidationError("invalid or duplicate FASTA index row").into());
            }
        }
        if index.is_empty() {
            return Err(ValidationError("empty FASTA index").into());
        }
        Ok(Self { file, index })
    }
    pub fn context(&mut self, chromosome: &str, position: u64) -> Result<[u8; 3], ValidationError> {
        let entry = self.index.get(chromosome).ok_or(ValidationError(
            "chromosome absent from FASTA index; aliases are not guessed",
        ))?;
        if position < 2 || position >= entry.length {
            return Err(ValidationError("variant lacks complete FASTA context"));
        }
        let mut context = [0; 3];
        for (i, base) in context.iter_mut().enumerate() {
            let pos = position - 2 + i as u64;
            let offset = pos / entry.line_bases;
            let offset = offset
                .checked_mul(entry.line_bytes)
                .and_then(|v| v.checked_add(entry.offset))
                .and_then(|v| v.checked_add(pos % entry.line_bases))
                .ok_or(ValidationError("FASTA offset overflow"))?;
            self.file
                .seek(SeekFrom::Start(offset))
                .map_err(|_| ValidationError("FASTA seek failed"))?;
            let mut bytes = [0];
            self.file
                .read_exact(&mut bytes)
                .map_err(|_| ValidationError("FASTA read failed"))?;
            *base = bytes[0].to_ascii_uppercase();
        }
        Ok(context)
    }
}
fn file_hash(path: &Path) -> Result<String, AppError> {
    // Stream the potentially large genome; avoid loading it into memory.
    use sha2::{Digest, Sha256};
    let mut digest = Sha256::new();
    let mut reader = BufReader::new(File::open(path)?);
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        digest.update(&buf[..n]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
pub fn derive(
    features: &Path,
    fasta: &Path,
    fai: &Path,
    catalogue: &Path,
    opportunity: &str,
    min_mutations: u64,
) -> Result<SignatureRun, AppError> {
    let set = crate::molecular::load_features(features)?;
    let catalogue: Catalogue = workflows::read_json(catalogue, 4 * 1024 * 1024)?;
    let mut reference = Fasta::open(fasta, fai)?;
    let spectrum = signatures::count(&set, &catalogue.reference_build, |chrom, pos| {
        reference.context(chrom, pos)
    })?;
    let fit = signatures::fit(&spectrum, &catalogue, opportunity, min_mutations)?;
    Ok(SignatureRun {
        schema_version: 1,
        feature_sha256: hash(&set)?,
        fasta_sha256: file_hash(fasta)?,
        fai_sha256: file_hash(fai)?,
        catalogue,
        min_mutations,
        spectrum,
        fit,
    })
}
pub fn attach(mut set: FeatureSet, run: SignatureRun) -> Result<FeatureSet, AppError> {
    if run.schema_version != 1 || run.feature_sha256 != hash(&set)? || !run.fit.converged {
        return Err(ValidationError(
            "signature derivation does not match features or did not converge",
        )
        .into());
    }
    let verified = signatures::fit(
        &run.spectrum,
        &run.catalogue,
        &run.catalogue.opportunity_profile,
        run.min_mutations,
    )?;
    if hash(&verified)? != hash(&run.fit)? {
        return Err(ValidationError("signature fit does not reproduce").into());
    }
    let sha = hash(&run)?;
    let dependent: Vec<_> = set
        .features
        .iter()
        .filter(|f| matches!(f.value, Some(FeatureValue::Mutation { .. })))
        .map(|f| f.id.clone())
        .collect();
    if dependent.is_empty() {
        return Err(ValidationError("signature derivation has no source variants").into());
    }
    // Derived signatures and their source observations are an inseparable game unit.
    // Merge whole existing groups so earlier dependency constraints remain intact.
    let groups: Vec<_> = set
        .features
        .iter()
        .filter(|f| dependent.contains(&f.id))
        .map(|f| f.group.clone())
        .collect();
    for f in &mut set.features {
        if groups.contains(&f.group) {
            f.group = "variants-and-derived-signatures".into();
        }
    }
    for (i, (signature, value)) in run.fit.exposures.iter().enumerate() {
        set.features.push(Feature {
            id: format!("derived-{signature}"),
            name: signature.clone(),
            modality: Modality::Signature,
            group: "variants-and-derived-signatures".into(),
            status: MeasurementStatus::Observed,
            value: Some(FeatureValue::Signature {
                signature: signature.clone(),
                value: *value,
                units: "fitted_mutation_count".into(),
                catalogue: run.catalogue.version.clone(),
                method: run.fit.method.clone(),
                mutation_count: run.fit.mutation_count,
            }),
            assay: run.catalogue.opportunity_profile.clone(),
            reference_build: Some(run.catalogue.reference_build.clone()),
            coverage: format!(
                "residual {:.4}; uncertainty not estimated",
                run.fit.relative_residual
            ),
            source: FeatureSource {
                source_id: "rust-sbs96-derivation".into(),
                sha256: sha.clone(),
                record: i as u64 + 1,
            },
            depends_on: dependent.clone(),
        });
    }
    set.validate()?;
    Ok(set)
}
