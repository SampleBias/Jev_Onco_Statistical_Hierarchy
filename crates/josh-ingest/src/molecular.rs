//! Strict single-sample molecular imports. Unsupported encodings are rejected.
use josh_core::{DataClass, ValidationError, molecular::*};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
};

pub const MAX_BYTES: usize = 16 * 1024 * 1024;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Validation(#[from] ValidationError),
    #[error("molecular import exceeds byte or feature limit")]
    Limit,
    #[error("invalid molecular table, unsupported columns or value encoding")]
    Format,
    #[error("molecular source cannot be read")]
    Io,
}
#[derive(Clone)]
pub struct Options {
    pub sample_id: String,
    pub patient_group_id: String,
    pub data_class: DataClass,
    pub source_id: String,
    pub assay: String,
    pub reference_build: Option<String>,
}
#[derive(Debug, Clone, Copy)]
pub enum Format {
    Csv,
    Tsv,
    Maf,
    Vcf,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub source_sha256: String,
    pub accepted_records: usize,
    pub features: FeatureSet,
    pub notices: Vec<String>,
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Row {
    sample_id: String,
    id: String,
    name: String,
    modality: String,
    value: String,
    units: String,
    status: String,
    assay: String,
    coverage: String,
    reference_build: String,
    group: String,
    gene: String,
    chromosome: String,
    position: String,
    reference: String,
    alternate: String,
    consequence: String,
    somatic: String,
    catalogue: String,
    method: String,
    mutation_count: String,
    reference_sha256: String,
    depends_on: String,
}
fn number(s: &str) -> Result<f64, Error> {
    let n: f64 = s.parse().map_err(|_| Error::Format)?;
    if n.is_finite() {
        Ok(n)
    } else {
        Err(Error::Format)
    }
}
fn integer<T: std::str::FromStr>(s: &str) -> Result<T, Error> {
    s.parse().map_err(|_| Error::Format)
}
fn somatic(s: &str) -> Result<Option<bool>, Error> {
    match s {
        "true" | "Somatic" => Ok(Some(true)),
        "false" | "Germline" => Ok(Some(false)),
        "" | "unknown" => Ok(None),
        _ => Err(Error::Format),
    }
}
fn row(r: Row, o: &Options, hash: &str, record: usize) -> Result<Feature, Error> {
    if r.sample_id != o.sample_id {
        return Err(ValidationError("source sample does not match selected sample").into());
    }
    let modality = match r.modality.as_str() {
        "mutation" => Modality::Mutation,
        "copy_number" => Modality::CopyNumber,
        "signature" => Modality::Signature,
        "expression" => Modality::Expression,
        "demographic" => Modality::Demographic,
        "ihc" => Modality::Ihc,
        "histology" => Modality::Histology,
        _ => return Err(Error::Format),
    };
    let status = match r.status.as_str() {
        "observed" => MeasurementStatus::Observed,
        "unknown" => MeasurementStatus::Unknown,
        "not_tested" => MeasurementStatus::NotTested,
        _ => return Err(Error::Format),
    };
    let value = if status != MeasurementStatus::Observed {
        if [
            &r.value,
            &r.reference,
            &r.alternate,
            &r.position,
            &r.somatic,
        ]
        .iter()
        .any(|s| !s.is_empty())
        {
            return Err(Error::Format);
        }
        None
    } else {
        Some(match modality {
            Modality::Mutation if !r.gene.is_empty() => FeatureValue::Mutation {
                gene: r.gene,
                chromosome: r.chromosome,
                position: integer(&r.position)?,
                reference: r.reference,
                alternate: r.alternate,
                consequence: r.consequence,
                somatic: somatic(&r.somatic)?,
            },
            Modality::CopyNumber if r.units == "discrete_call" => FeatureValue::CopyNumber {
                gene: r.gene,
                call: integer(&r.value)?,
            },
            Modality::Signature => FeatureValue::Signature {
                signature: r.name.clone(),
                value: number(&r.value)?,
                units: r.units,
                catalogue: r.catalogue,
                method: r.method,
                mutation_count: integer(&r.mutation_count)?,
            },
            Modality::Demographic if r.units == "years" => FeatureValue::Age {
                years: integer(r.value.trim_start_matches('>'))?,
                lower_bound_exclusive: r.value.starts_with('>'),
            },
            Modality::Demographic | Modality::Ihc | Modality::Histology if r.units.is_empty() => {
                FeatureValue::Category { value: r.value }
            }
            Modality::Expression if !r.reference_sha256.is_empty() => FeatureValue::Vector {
                values: serde_json::from_str(&r.value).map_err(|_| Error::Format)?,
                units: r.units,
                reference_sha256: r.reference_sha256,
            },
            _ => FeatureValue::Number {
                value: number(&r.value)?,
                units: r.units,
            },
        })
    };
    Ok(Feature {
        id: r.id.clone(),
        name: r.name,
        modality,
        group: if r.group.is_empty() { r.id } else { r.group },
        status,
        value,
        assay: if r.assay.is_empty() {
            o.assay.clone()
        } else {
            r.assay
        },
        reference_build: if r.reference_build.is_empty() {
            o.reference_build.clone()
        } else {
            Some(r.reference_build)
        },
        coverage: if r.coverage.is_empty() {
            "unknown".into()
        } else {
            r.coverage
        },
        source: FeatureSource {
            source_id: o.source_id.clone(),
            sha256: hash.into(),
            record: record as u64,
        },
        depends_on: r
            .depends_on
            .split(';')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect(),
    })
}
pub fn import(mut reader: impl Read, format: Format, options: &Options) -> Result<Report, Error> {
    let mut bytes = Vec::new();
    reader
        .by_ref()
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Io)?;
    if bytes.len() > MAX_BYTES {
        return Err(Error::Limit);
    }
    let hash = bytes_hash(&bytes);
    let mut features = Vec::new();
    match format {
        Format::Csv | Format::Tsv => {
            let mut csv = csv::ReaderBuilder::new()
                .delimiter(if matches!(format, Format::Csv) {
                    b','
                } else {
                    b'\t'
                })
                .from_reader(bytes.as_slice());
            let headers = csv.headers().map_err(|_| Error::Format)?;
            if headers.iter().collect::<BTreeSet<_>>().len() != headers.len() {
                return Err(Error::Format);
            }
            for (i, record) in csv.deserialize::<Row>().enumerate() {
                if i >= MAX_FEATURES {
                    return Err(Error::Limit);
                }
                features.push(row(
                    record.map_err(|_| Error::Format)?,
                    options,
                    &hash,
                    i + 2,
                )?);
            }
        }
        Format::Maf => {
            let mut csv = csv::ReaderBuilder::new()
                .delimiter(b'\t')
                .comment(Some(b'#'))
                .from_reader(bytes.as_slice());
            let headers = csv.headers().map_err(|_| Error::Format)?.clone();
            if headers.iter().collect::<BTreeSet<_>>().len() != headers.len() {
                return Err(Error::Format);
            }
            for (i, record) in csv.records().enumerate() {
                if i >= MAX_FEATURES {
                    return Err(Error::Limit);
                }
                let record = record.map_err(|_| Error::Format)?;
                let cols: BTreeMap<_, _> = headers.iter().zip(record.iter()).collect();
                let get = |k| cols.get(k).copied().ok_or(Error::Format);
                let build = match get("NCBI_Build")? {
                    "37" | "GRCh37" => "GRCh37",
                    "38" | "GRCh38" => "GRCh38",
                    _ => return Err(Error::Format),
                };
                if options.reference_build.as_ref().is_some_and(|b| b != build) {
                    return Err(Error::Format);
                }
                let gene = get("Hugo_Symbol")?.to_string();
                let position = get("Start_Position")?.to_string();
                features.push(row(
                    Row {
                        sample_id: get("Tumor_Sample_Barcode")?.into(),
                        id: format!("variant-{}", i + 1),
                        name: gene.clone(),
                        modality: "mutation".into(),
                        status: "observed".into(),
                        gene,
                        chromosome: get("Chromosome")?.into(),
                        position,
                        reference: get("Reference_Allele")?.into(),
                        alternate: get("Tumor_Seq_Allele2")?.into(),
                        consequence: get("Variant_Classification")?.into(),
                        somatic: cols
                            .get("Mutation_Status")
                            .copied()
                            .unwrap_or("unknown")
                            .into(),
                        reference_build: build.into(),
                        ..Row::default()
                    },
                    options,
                    &hash,
                    i + 2,
                )?);
            }
        }
        Format::Vcf => {
            let source = std::str::from_utf8(&bytes).map_err(|_| Error::Format)?;
            let mut header = false;
            for (i, line) in source.lines().enumerate() {
                if line.starts_with("##") {
                    continue;
                }
                if line.starts_with("#CHROM\t") {
                    let fields: Vec<_> = line.split('\t').collect();
                    if header || fields.len() != 10 || fields[9] != options.sample_id {
                        return Err(ValidationError(
                            "VCF requires exactly the selected single sample",
                        )
                        .into());
                    }
                    header = true;
                    continue;
                }
                if !header || line.is_empty() {
                    return Err(Error::Format);
                }
                if features.len() >= MAX_FEATURES {
                    return Err(Error::Limit);
                }
                let c: Vec<_> = line.split('\t').collect();
                if c.len() != 10 || c[6] != "PASS" || c[4].contains(',') {
                    return Err(ValidationError(
                        "VCF subset requires PASS, annotated, biallelic records",
                    )
                    .into());
                }
                let info: BTreeMap<_, _> = c[7]
                    .split(';')
                    .map(|v| v.split_once('=').unwrap_or((v, "")))
                    .collect();
                let gene = info.get("GENE").ok_or(Error::Format)?.to_string();
                let consequence = info.get("CONSEQUENCE").ok_or(Error::Format)?.to_string();
                // Require an observed alternate genotype; a VCF record alone is not an alteration in this sample.
                let format: Vec<_> = c[8].split(':').collect();
                let sample: Vec<_> = c[9].split(':').collect();
                let gt = format
                    .iter()
                    .position(|s| *s == "GT")
                    .and_then(|j| sample.get(j))
                    .ok_or(Error::Format)?;
                if !matches!(*gt, "0/1" | "1/0" | "1/1" | "0|1" | "1|0" | "1|1" | "1") {
                    return Err(Error::Format);
                }
                features.push(row(
                    Row {
                        sample_id: options.sample_id.clone(),
                        id: format!("variant-{}", features.len() + 1),
                        name: gene.clone(),
                        modality: "mutation".into(),
                        status: "observed".into(),
                        gene,
                        chromosome: c[0].into(),
                        position: c[1].into(),
                        reference: c[3].into(),
                        alternate: c[4].into(),
                        consequence,
                        somatic: if info.contains_key("SOMATIC") {
                            "true"
                        } else {
                            "unknown"
                        }
                        .into(),
                        ..Row::default()
                    },
                    options,
                    &hash,
                    i + 1,
                )?);
            }
        }
    }
    // Duplicate coordinates must not become duplicate evidence under different IDs.
    let mut variants = BTreeSet::new();
    for f in &features {
        if let Some(FeatureValue::Mutation {
            chromosome,
            position,
            reference,
            alternate,
            ..
        }) = &f.value
            && !variants.insert((
                f.reference_build.clone(),
                chromosome.clone(),
                *position,
                reference.clone(),
                alternate.clone(),
            ))
        {
            return Err(ValidationError("duplicate variant observation").into());
        }
    }
    let features = FeatureSet {
        schema_version: 1,
        pipeline_version: PIPELINE.into(),
        sample_id: options.sample_id.clone(),
        patient_group_id: options.patient_group_id.clone(),
        data_class: options.data_class.clone(),
        features,
    };
    features.validate()?;
    Ok(Report {source_sha256:hash,accepted_records:features.features.len(),features,notices:vec!["No variant normalization, annotation inference or signature estimation is implied by import.".into(),"Observed variant records do not establish callable wild-type coverage elsewhere.".into()]})
}
