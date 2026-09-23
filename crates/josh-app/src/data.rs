//! Dataset CLI and shared services for the Ratatui data workbench.
use crate::workflows::{self, AppError};
use clap::{Args, Subcommand, ValueEnum};
use josh_core::sample::*;
use josh_ingest::{dataset, expression};
use serde_json::{Value, json};
use std::{
    fs::File,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Units {
    Unknown,
    Counts,
    Tpm,
    Fpkm,
    Normalized,
    MicroarrayIntensity,
    Log2,
    ZScore,
}
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Layout {
    Auto,
    Long,
    Wide,
}
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Separator {
    Auto,
    Csv,
    Tsv,
}
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum GeneIds {
    Auto,
    HgncId,
    Symbol,
    Ensembl,
    Entrez,
}
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Scaling {
    Identity,
    Log2OnePlus,
}

#[derive(Debug, Clone, Args)]
pub struct ExpressionArgs {
    #[arg(long, value_enum, default_value = "auto")]
    pub delimiter: Separator,
    #[arg(long, value_enum, default_value = "auto")]
    pub layout: Layout,
    #[arg(long, default_value = "gene")]
    pub gene_column: String,
    #[arg(long, default_value = "expression")]
    pub value_column: String,
    #[arg(long, default_value = "sample_id")]
    pub sample_column: String,
    /// Required for a single-sample two-column table.
    #[arg(long)]
    pub sample_id: Option<String>,
    #[arg(long, value_enum, default_value = "auto")]
    pub gene_ids: GeneIds,
    /// Declared source units; never inferred from numerical ranges.
    #[arg(long, value_enum, default_value = "unknown")]
    pub units: Units,
    /// Explicit numerical transform; raw values remain unchanged.
    #[arg(long, value_enum, default_value = "identity")]
    pub transform: Scaling,
    #[arg(long)]
    pub genome: Option<String>,
    #[arg(long)]
    pub platform: Option<String>,
}

impl ExpressionArgs {
    pub fn config(&self) -> ExpressionConfig {
        ExpressionConfig {
            delimiter: match self.delimiter {
                Separator::Auto => Delimiter::Auto,
                Separator::Csv => Delimiter::Csv,
                Separator::Tsv => Delimiter::Tsv,
            },
            layout: match self.layout {
                Layout::Auto => MatrixLayout::Auto,
                Layout::Long => MatrixLayout::Long,
                Layout::Wide => MatrixLayout::Wide,
            },
            gene_column: self.gene_column.clone(),
            value_column: self.value_column.clone(),
            sample_column: self.sample_column.clone(),
            single_sample_id: self.sample_id.clone(),
            gene_namespace: match self.gene_ids {
                GeneIds::Auto => GeneNamespace::Auto,
                GeneIds::HgncId => GeneNamespace::HgncId,
                GeneIds::Symbol => GeneNamespace::Symbol,
                GeneIds::Ensembl => GeneNamespace::Ensembl,
                GeneIds::Entrez => GeneNamespace::Entrez,
            },
            units: match self.units {
                Units::Unknown => ExpressionUnit::Unknown,
                Units::Counts => ExpressionUnit::Counts,
                Units::Tpm => ExpressionUnit::Tpm,
                Units::Fpkm => ExpressionUnit::Fpkm,
                Units::Normalized => ExpressionUnit::Normalized,
                Units::MicroarrayIntensity => ExpressionUnit::MicroarrayIntensity,
                Units::Log2 => ExpressionUnit::Log2,
                Units::ZScore => ExpressionUnit::ZScore,
            },
            transform: match self.transform {
                Scaling::Identity => Transform::Identity,
                Scaling::Log2OnePlus => Transform::Log2OnePlus,
            },
            organism: "Homo sapiens".into(),
            reference_genome: self.genome.clone(),
            platform: self.platform.clone(),
        }
    }
}

#[derive(Subcommand)]
pub enum DatasetCommand {
    /// Preview delimiter, matrix layout and columns; sends no data to Jev.
    Detect {
        input: PathBuf,
        #[command(flatten)]
        expression: ExpressionArgs,
    },
    /// Import gene-expression CSV/TSV into a new immutable dataset bundle.
    Import {
        /// File or '-' for stdin/pasted tabular data.
        input: PathBuf,
        #[command(flatten)]
        expression: ExpressionArgs,
        #[arg(long)]
        dataset_id: String,
        #[arg(long)]
        out_dir: PathBuf,
        #[arg(long, requires = "gene_map_release")]
        gene_map: Option<PathBuf>,
        #[arg(long, requires = "gene_map")]
        gene_map_release: Option<String>,
        #[arg(long)]
        study: Option<String>,
        #[arg(long)]
        accession: Option<String>,
        #[arg(long)]
        citation: Option<String>,
        /// Only use for invented data; this is a declaration, not deidentification.
        #[arg(long)]
        synthetic: bool,
    },
    /// Inspect all samples, QC, provenance and the generated data dictionary.
    Inspect { directory: PathBuf },
    /// Inspect numeric features without inference. Search original IDs or canonical genes.
    Explore {
        directory: PathBuf,
        #[arg(long)]
        sample: Option<String>,
        #[arg(long)]
        gene: Option<String>,
        #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u16).range(1..=1000))]
        limit: u16,
    },
    /// Check artifacts; optionally recompute mappings, values and QC from archived sources.
    Verify {
        directory: PathBuf,
        #[arg(long)]
        reproduce: bool,
    },
    /// Preserve a v1-v3 case byte-for-byte as a sample's legacy evidence attachment.
    MigrateCase {
        input: PathBuf,
        #[arg(long)]
        dataset_id: String,
        #[arg(long)]
        out_dir: PathBuf,
    },
    /// Export one sample's complete expression measurements, including source locators.
    Export {
        directory: PathBuf,
        #[arg(long)]
        sample: Option<String>,
        #[arg(long, value_enum, default_value = "json")]
        kind: ExportKind,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum ExportKind {
    Json,
    Csv,
    Tsv,
}

pub struct CommandResult {
    pub value: Value,
    pub human: String,
    pub raw: Option<String>,
    pub blocked: bool,
}

pub fn input(path: &Path) -> Result<Vec<u8>, AppError> {
    Ok(if path == Path::new("-") {
        dataset::bounded_read(std::io::stdin().lock(), expression::MAX_SOURCE)?
    } else {
        dataset::bounded_read(File::open(path)?, expression::MAX_SOURCE)?
    })
}

pub fn sample<'a>(manifest: &'a DatasetManifest, id: Option<&str>) -> Result<&'a Sample, AppError> {
    if let Some(id) = id {
        manifest
            .samples
            .iter()
            .find(|s| s.sample_id == id)
            .ok_or_else(|| {
                josh_core::errors::ErrorEnvelope::new(josh_core::errors::ErrorCode::SampleSelection)
                    .into()
            })
    } else if manifest.samples.len() == 1 {
        Ok(&manifest.samples[0])
    } else {
        Err(
            josh_core::errors::ErrorEnvelope::new(josh_core::errors::ErrorCode::SampleSelection)
                .into(),
        )
    }
}

pub fn summary(manifest: &DatasetManifest) -> String {
    let mut lines = vec![
        format!(
            "Dataset: {} | {} samples",
            manifest.dataset_id,
            manifest.samples.len()
        ),
        format!(
            "Source: {} | SHA-256: {}",
            manifest.source_name, manifest.source.artifact.sha256
        ),
        format!("Pipeline: {}", manifest.pipeline_version),
    ];
    if let Some(config) = &manifest.expression_config {
        lines.push(format!(
            "Expression: {:?} | {:?} | {:?}",
            config.units, config.layout, config.transform
        ));
    }
    for s in &manifest.samples {
        if let Some(qc) = s.assays.first().and_then(|a| a.qc.as_ref()) {
            lines.push(format!("{} | {:?} | {} records | {} measured | {} mapped | {} missing | {} invalid | {} duplicate", s.sample_id, qc.status, qc.total_records, qc.measured_values, qc.mapped_records, qc.missing_values, qc.invalid_values, qc.duplicate_gene_records));
        } else {
            lines.push(format!("{} | legacy annotations preserved", s.sample_id));
        }
    }
    lines.push(
        "Reference comparison: not implemented. No molecular prediction has been run.".into(),
    );
    lines.join("\n")
}

pub fn execute(command: DatasetCommand) -> Result<CommandResult, AppError> {
    use expression::DatasetAdapter;
    let mut raw = None;
    let mut blocked = false;
    let (value, human) = match command {
        DatasetCommand::Detect {
            input: path,
            expression,
        } => {
            let detection =
                expression::ExpressionTableAdapter.detect(&input(&path)?, &expression.config())?;
            (
                serde_json::to_value(&detection)?,
                format!(
                    "Candidate: expression | {:?} | {:?}\nColumns: {}\nUnits/platform/genome require explicit declarations.",
                    detection.delimiter,
                    detection.layout,
                    detection.columns.join(", ")
                ),
            )
        }
        DatasetCommand::Import {
            input: path,
            expression,
            dataset_id,
            out_dir,
            gene_map,
            gene_map_release,
            study,
            accession,
            citation,
            synthetic,
        } => {
            if gene_map.as_ref().is_some_and(|p| p == Path::new("-")) {
                return Err(dataset::DatasetError::Configuration.into());
            }
            let mapping = gene_map
                .map(|p| Ok::<_, AppError>((input(&p)?, gene_map_release.unwrap_or_default())))
                .transpose()?;
            let options = expression::ImportOptions {
                dataset_id,
                source_name: path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "stdin".into()),
                study,
                accession,
                citation,
                data_class: if synthetic {
                    josh_core::DataClass::Synthetic
                } else {
                    josh_core::DataClass::DeidentifiedResearch
                },
                config: expression.config(),
            };
            let data = expression::import(input(&path)?, &options, mapping)?;
            dataset::write(&out_dir, &data)?;
            blocked = data.manifest.samples.iter().any(|s| {
                s.assays
                    .iter()
                    .any(|a| a.qc.as_ref().is_some_and(|q| q.status == QcStatus::Blocked))
            });
            let human = format!("{}\nSaved: {}", summary(&data.manifest), out_dir.display());
            (serde_json::to_value(data.manifest)?, human)
        }
        DatasetCommand::Inspect { directory } => {
            let manifest = dataset::read(&directory)?;
            let human = summary(&manifest);
            (serde_json::to_value(manifest)?, human)
        }
        DatasetCommand::Explore {
            directory,
            sample: id,
            gene,
            limit,
        } => {
            let manifest = dataset::read(&directory)?;
            let selected = sample(&manifest, id.as_deref())?;
            let mut records = dataset::read_records(&directory, selected)?;
            if let Some(query) = gene {
                let query = query.to_ascii_lowercase();
                records.retain(|r| {
                    r.original_gene_id.to_ascii_lowercase().contains(&query)
                        || r.mapping.gene.as_ref().is_some_and(|g| {
                            g.symbol.to_ascii_lowercase().contains(&query)
                                || g.hgnc_id.to_ascii_lowercase().contains(&query)
                        })
                });
            }
            records.sort_by(|a, b| {
                b.raw_expression
                    .unwrap_or(f64::NEG_INFINITY)
                    .total_cmp(&a.raw_expression.unwrap_or(f64::NEG_INFINITY))
                    .then(a.original_gene_id.cmp(&b.original_gene_id))
            });
            let matched = records.len();
            records.truncate(usize::from(limit));
            let human = records
                .iter()
                .map(|r| {
                    format!(
                        "{}\t{}\t{:?}\t{:?}\trecord {} column {}",
                        r.original_gene_id,
                        r.mapping
                            .gene
                            .as_ref()
                            .map(|g| g.symbol.as_str())
                            .unwrap_or("unresolved"),
                        r.raw_expression,
                        r.transformed_expression,
                        r.source_record,
                        r.source_column
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            (
                json!({"sample_id":selected.sample_id,"matching_records":matched,"returned_records":records.len(),"records":records}),
                human,
            )
        }
        DatasetCommand::Verify {
            directory,
            reproduce,
        } => {
            let manifest = if reproduce {
                dataset::reproduce(&directory)?
            } else {
                dataset::read(&directory)?
            };
            (
                json!({"dataset_id":manifest.dataset_id,"artifacts_verified":true,"derived_data_reproduced":reproduce,"provider_called":false}),
                "Dataset artifacts verified; requested local replay completed.".into(),
            )
        }
        DatasetCommand::MigrateCase {
            input: path,
            dataset_id,
            out_dir,
        } => {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let manifest = dataset::import_legacy(input(&path)?, &dataset_id, &name, &out_dir)?;
            let human = summary(&manifest);
            (serde_json::to_value(manifest)?, human)
        }
        DatasetCommand::Export {
            directory,
            sample: id,
            kind,
        } => {
            let manifest = dataset::read(&directory)?;
            let selected = sample(&manifest, id.as_deref())?;
            let records = dataset::read_records(&directory, selected)?;
            if matches!(kind, ExportKind::Csv | ExportKind::Tsv) {
                raw = Some(tabular_export(&records, kind, &manifest, selected)?);
            }
            let value =
                json!({"dataset":manifest,"sample_id":selected.sample_id,"records":records});
            let human = workflows::pretty(&value)?;
            (value, human)
        }
    };
    Ok(CommandResult {
        value,
        human,
        raw,
        blocked,
    })
}

pub fn tabular_export(
    records: &[ExpressionRecord],
    kind: ExportKind,
    manifest: &DatasetManifest,
    sample: &Sample,
) -> Result<String, AppError> {
    let mut out = csv::WriterBuilder::new()
        .delimiter(if matches!(kind, ExportKind::Tsv) {
            b'\t'
        } else {
            b','
        })
        .from_writer(vec![]);
    out.write_record([
        "original_gene_id",
        "hgnc_id",
        "symbol",
        "mapping_status",
        "original_value",
        "raw_expression",
        "transformed_expression",
        "source_record",
        "source_column",
        "dataset_id",
        "sample_id",
        "source_sha256",
    ])?;
    for r in records {
        out.write_record([
            r.original_gene_id.clone(),
            r.mapping
                .gene
                .as_ref()
                .map(|g| g.hgnc_id.clone())
                .unwrap_or_default(),
            r.mapping
                .gene
                .as_ref()
                .map(|g| g.symbol.clone())
                .unwrap_or_default(),
            format!("{:?}", r.mapping.status),
            r.original_value.clone(),
            r.raw_expression.map(|v| v.to_string()).unwrap_or_default(),
            r.transformed_expression
                .map(|v| v.to_string())
                .unwrap_or_default(),
            r.source_record.to_string(),
            r.source_column.to_string(),
            manifest.dataset_id.clone(),
            sample.sample_id.clone(),
            manifest.source.artifact.sha256.clone(),
        ])?;
    }
    Ok(String::from_utf8(out.into_inner()?)?)
}
