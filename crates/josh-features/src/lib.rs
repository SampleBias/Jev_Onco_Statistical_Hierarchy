//! Deterministic gene mapping and numerical processing; no network or classifier.
pub mod reference;

use josh_core::sample::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, thiserror::Error)]
#[error("invalid or unsupported HGNC mapping table")]
pub struct MappingError;

type Index = BTreeMap<String, Vec<GeneIdentity>>;

pub struct GeneMapper {
    ids: Index,
    symbols: Index,
    aliases: Index,
    ensembl: Index,
    entrez: Index,
}

impl GeneMapper {
    /// HGNC complete-set TSV. The caller records the exact file hash and release.
    pub fn from_tsv(bytes: &[u8]) -> Result<Self, MappingError> {
        if bytes.len() > 64 * 1024 * 1024 {
            return Err(MappingError);
        }
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b'\t')
            .from_reader(bytes);
        let headers = reader.headers().map_err(|_| MappingError)?.clone();
        let index = |name| headers.iter().position(|h| h == name).ok_or(MappingError);
        if headers.iter().collect::<BTreeSet<_>>().len() != headers.len() {
            return Err(MappingError);
        }
        let (id, symbol, status, ensembl, entrez) = (
            index("hgnc_id")?,
            index("symbol")?,
            index("status")?,
            index("ensembl_gene_id")?,
            index("entrez_id")?,
        );
        let alias = headers.iter().position(|h| h == "alias_symbol");
        let previous = headers.iter().position(|h| h == "prev_symbol");
        let mut mapper = Self {
            ids: Index::new(),
            symbols: Index::new(),
            aliases: Index::new(),
            ensembl: Index::new(),
            entrez: Index::new(),
        };
        for (n, record) in reader.records().enumerate() {
            if n >= 100_000 {
                return Err(MappingError);
            }
            let row = record.map_err(|_| MappingError)?;
            if row.iter().any(|v| v.len() > 65_536) {
                return Err(MappingError);
            }
            if &row[status] != "Approved" {
                continue;
            }
            if !row[id]
                .strip_prefix("HGNC:")
                .is_some_and(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
                || !valid_label(&row[symbol])
                || mapper.ids.contains_key(&row[id])
            {
                return Err(MappingError);
            }
            let gene = GeneIdentity {
                hgnc_id: row[id].into(),
                symbol: row[symbol].into(),
            };
            insert(&mut mapper.ids, &row[id], &gene);
            insert(&mut mapper.symbols, &row[symbol], &gene);
            insert(&mut mapper.ensembl, &row[ensembl], &gene);
            insert(&mut mapper.entrez, &row[entrez], &gene);
            for i in [alias, previous].into_iter().flatten() {
                for value in row[i].split('|') {
                    insert(&mut mapper.aliases, value, &gene);
                }
            }
        }
        if mapper.ids.is_empty() {
            return Err(MappingError);
        }
        Ok(mapper)
    }

    pub fn resolve(&self, original: &str, namespace: GeneNamespace) -> GeneMapping {
        // Preserve case and spelling. No speculative symbol uppercasing or fuzzy matches.
        let namespace = if namespace == GeneNamespace::Auto {
            if original.starts_with("HGNC:") {
                GeneNamespace::HgncId
            } else if original.starts_with("ENSG") {
                GeneNamespace::Ensembl
            } else if !original.is_empty() && original.bytes().all(|b| b.is_ascii_digit()) {
                GeneNamespace::Entrez
            } else {
                GeneNamespace::Symbol
            }
        } else {
            namespace
        };
        let mut status = MappingStatus::Exact;
        let hits = match namespace {
            GeneNamespace::HgncId => self.ids.get(original),
            GeneNamespace::Entrez => self.entrez.get(original),
            GeneNamespace::Symbol | GeneNamespace::Auto => {
                self.symbols.get(original).or_else(|| {
                    status = MappingStatus::Alias;
                    self.aliases.get(original)
                })
            }
            GeneNamespace::Ensembl => {
                let key = if let Some((base, suffix)) = original.split_once('.') {
                    if !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()) {
                        status = MappingStatus::VersionStripped;
                        base
                    } else {
                        original
                    }
                } else {
                    original
                };
                self.ensembl.get(key)
            }
        };
        let candidates = hits.cloned().unwrap_or_default();
        match candidates.len() {
            0 => GeneMapping {
                status: MappingStatus::Unmapped,
                gene: None,
                candidates,
            },
            1 => GeneMapping {
                status,
                gene: Some(candidates[0].clone()),
                candidates: vec![],
            },
            _ => GeneMapping {
                status: MappingStatus::Ambiguous,
                gene: None,
                candidates,
            },
        }
    }
}

fn insert(index: &mut Index, key: &str, gene: &GeneIdentity) {
    if !key.is_empty() {
        let hits = index.entry(key.into()).or_default();
        if !hits.contains(gene) {
            hits.push(gene.clone());
            hits.sort_by(|a, b| a.hgnc_id.cmp(&b.hgnc_id));
        }
    }
}

pub fn valid_transform(config: &ExpressionConfig) -> bool {
    config.transform == Transform::Identity
        || matches!(
            config.units,
            ExpressionUnit::Counts | ExpressionUnit::Tpm | ExpressionUnit::Fpkm
        )
}

/// Adds parsed/derived values and mapping without modifying original fields.
pub fn process(
    records: &mut [ExpressionRecord],
    config: &ExpressionConfig,
    mapper: Option<&GeneMapper>,
) -> QcReport {
    let mut qc = QcReport {
        status: QcStatus::Pass,
        total_records: records.len(),
        measured_values: 0,
        missing_values: 0,
        invalid_values: 0,
        zero_values: 0,
        mapped_records: 0,
        ambiguous_records: 0,
        unmapped_records: 0,
        duplicate_gene_records: 0,
        gene_id_mapping_rate: None,
        minimum: None,
        maximum: None,
        issues: vec![],
        reference_compatibility: "not_assessed_no_reference".into(),
    };
    let mut raw_ids = BTreeSet::new();
    let mut canonical = BTreeSet::new();
    for record in records {
        record.raw_expression = None;
        record.transformed_expression = None;
        record.mapping = mapper
            .map(|m| m.resolve(&record.original_gene_id, config.gene_namespace))
            .unwrap_or(GeneMapping {
                status: MappingStatus::NotAttempted,
                gene: None,
                candidates: vec![],
            });
        let raw_duplicate = !raw_ids.insert(record.original_gene_id.clone());
        let canonical_duplicate = record
            .mapping
            .gene
            .as_ref()
            .is_some_and(|g| !canonical.insert(g.hgnc_id.clone()));
        if raw_duplicate || canonical_duplicate {
            qc.duplicate_gene_records += 1;
            issue(
                &mut qc,
                "duplicate_gene_no_aggregation",
                Some(record.source_record),
            );
        }
        if record.mapping.gene.is_some() {
            qc.mapped_records += 1;
        }
        if record.mapping.status == MappingStatus::Ambiguous {
            qc.ambiguous_records += 1;
        }
        if record.mapping.status == MappingStatus::Unmapped {
            qc.unmapped_records += 1;
        }
        let value = record.original_value.trim();
        if value.is_empty()
            || matches!(
                value.to_ascii_lowercase().as_str(),
                "na" | "n/a" | "null" | "."
            )
        {
            qc.missing_values += 1;
            continue;
        }
        let parsed = value
            .parse::<f64>()
            .ok()
            .filter(|v| config.units.accepts(*v));
        if let Some(value) = parsed {
            record.raw_expression = Some(value);
            record.transformed_expression = if !valid_transform(config) {
                None
            } else {
                Some(match config.transform {
                    Transform::Identity => value,
                    Transform::Log2OnePlus => value.ln_1p() / std::f64::consts::LN_2,
                })
            };
            qc.measured_values += 1;
            qc.zero_values += usize::from(value == 0.0);
            qc.minimum = Some(qc.minimum.map_or(value, |v| v.min(value)));
            qc.maximum = Some(qc.maximum.map_or(value, |v| v.max(value)));
        } else {
            qc.invalid_values += 1;
            issue(
                &mut qc,
                "invalid_expression_for_declared_units",
                Some(record.source_record),
            );
        }
    }
    if mapper.is_none() {
        issue(&mut qc, "gene_mapping_not_supplied", None);
    } else if qc.total_records > 0 {
        qc.gene_id_mapping_rate = Some(qc.mapped_records as f64 / qc.total_records as f64);
    }
    if config.units == ExpressionUnit::Unknown {
        issue(&mut qc, "expression_units_unknown", None);
    }
    if config.platform.is_none() {
        issue(&mut qc, "platform_unknown", None);
    }
    if qc.missing_values > 0 {
        issue(&mut qc, "missing_measurements", None);
    }
    if qc.unmapped_records > 0 {
        issue(&mut qc, "unmapped_gene_identifiers", None);
    }
    if qc.ambiguous_records > 0 {
        issue(&mut qc, "ambiguous_gene_identifiers", None);
    }
    if !valid_transform(config) {
        issue(&mut qc, "incompatible_transform", None);
    }
    if qc.measured_values == 0 {
        issue(&mut qc, "no_measured_expression", None);
    }
    if qc.invalid_values > 0
        || qc.duplicate_gene_records > 0
        || qc.measured_values == 0
        || !valid_transform(config)
    {
        qc.status = QcStatus::Blocked;
    } else if !qc.issues.is_empty() {
        qc.status = QcStatus::Warning;
    }
    qc
}

fn issue(qc: &mut QcReport, code: &str, source_record: Option<u64>) {
    // Counts above remain complete; per-record detail is capped to keep manifests bounded.
    if qc.issues.len() < 1000 {
        qc.issues.push(QcIssue {
            code: code.into(),
            source_record,
        });
    }
}
