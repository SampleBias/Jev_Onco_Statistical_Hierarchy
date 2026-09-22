# Importing evidence — application 0.3.0

Imports run locally and send nothing to Jev. They produce validated case files for the CLI/TUI, source references, a quality report, protected labels and patient-group partitions. The first supported tabular format is the project's normalized evidence table; a raw GENIE release needs a separate release-specific mapper.

## Run the synthetic example

```bash
cargo build --workspace --locked
mkdir -p results
./target/debug/nexus import fixtures/import/synthetic-findings.csv \
  --input-format csv --source-id synthetic-v1 \
  --out-dir results/import-001 --format text
./target/debug/nexus batch results/import-001 --format text
./target/debug/nexus tui --batch results/import-001
```

The output directory must be new, even if an existing directory is empty. Choose `import-002` for another run. Its parent must exist. Both `results/` and `data/` at the repository root are ignored by Git. New bundle directories use mode 0700 and files use 0600 on Unix.

In the TUI, use `[` and `]` to browse cases, `5` for import quality, and `b` to open another bundle. Evidence shows the original finding value, observation status, assay/units and source record. `d` runs a mock; `c` retains the existing confirmation for a single synthetic Jev request. Importing a batch never starts batch inference. A changed case fingerprint prevents loading that case; make a standalone copy if you need to edit imported evidence.

Single-case and JSONL migration use the same command:

```bash
./target/debug/nexus import fixtures/synthetic-case.json \
  --input-format json --source-id legacy-example --out-dir results/migrated-001
./target/debug/nexus import fixtures/import/synthetic-cases.jsonl \
  --input-format jsonl --source-id synthetic-jsonl --out-dir results/jsonl-001
```

`-` reads stdin. Formats are explicit: `json` is one canonical case; `jsonl` is one complete canonical case per line; `csv` and `tsv` use the table below. JSONL blank lines are rejected records. CSV/TSV use the [Rust CSV parser](https://docs.rs/csv/1.4.0/csv/), supporting quoted delimiters and multiline values; its permissive quoting behavior and skipped blank lines apply. Input must be UTF-8.

## Table contract

Each data record represents one finding. Rows with the same `case_id` are joined into one case. Repeat the same case metadata and label on **every** row for that case; inconsistent metadata invalidates the whole case. Identifiers remain strings, preserving leading zeroes. IDs use 1–64 ASCII letters, digits, underscores or hyphens.

| Column | Required | Interpretation |
| --- | --- | --- |
| `case_id` | Yes | Local case key used to group observations |
| `data_class` | Yes | `synthetic` or `deidentified_research`; a declaration, not an identifier detector |
| `finding_id` | Yes | Unique within a case |
| `kind` | Yes | `histology`, `ihc`, `molecular`, `clinical` |
| `name` | Yes | Recorded observation/test name, nonblank, at most 128 UTF-8 bytes |
| `value` | Yes | Recorded result, nonblank, at most 1024 UTF-8 bytes; retained unchanged |
| `patient_id` | No | Local globally unique pseudonymous group for partitions; missing means unassigned |
| `sample_id` | No | Local specimen key; the same key in different cases invalidates those cases |
| `institution_id` | No | Local cohort site key; optional institution holdout uses this field |
| `age_years` | No | Exact integer 0–120 or an exclusive bound such as `>89`; `>=89`, decimals and malformed ages are rejected |
| `sex_at_birth` | No | `female`, `male`, `intersex`, `unknown`; empty is missing |
| `specimen_site` | No | Biopsy location; does not assert primary origin |
| `evidence_cutoff` | No | Local study cutoff declaration; source curation must actually enforce it |
| `status` | No | `observed`, `positive`, `negative`, `equivocal`, `not_tested`, `unknown` |
| `units` | No | Recorded measurement units; no automatic numeric conversion |
| `assay` | No | Recorded assay/panel or method; does not supply tested-gene coverage |
| `timepoint` | No | Relative observation timepoint included as qualifying evidence |
| `reference_build` | No | Recorded genomic reference build; coordinates/alleles are not normalized yet |
| `known_primary` | No | Separate evaluation label; exact current taxonomy key except `insufficient_evidence`; never a model input |

Unknown, duplicate or missing required headers reject the source. Column order is arbitrary; header names are exact. Empty optional cells become null. No automatic trimming or numeric coercion of identifiers occurs. Do not insert a final diagnosis into an allowed observation field: strict field validation cannot identify all indirect label leakage in prose. Clinical review must establish the evidence cutoff and reference labels.

When `status` is empty, narrow case-insensitive lexical aliases are recognized: `positive/pos/+`, `negative/neg/-`, `equivocal`, `not tested/not_tested/not assayed`, and `unknown`. Other nonblank values are `observed`. This is lexical normalization, not interpretation of a pathology report. Explicit status must agree with a recognized value. An unmeasured result stays `not_tested`; it is not converted to a negative. Blank required values are rejected. Conflicting findings with different IDs are preserved for review; contradictory status/value within one finding is rejected.

The demonstration `known_primary` values are authored test labels. They are not established truth for a medical evaluation. Real source diagnosis codes need a reviewed ontology mapping before use; unsupported labels are rejected rather than silently assigned to another class.

## Schema 2 and migration

Canonical schema 1 input remains supported. Import upgrades it to schema 2, adds source references and observation statuses, and leaves unknown patient/sample/cutoff metadata null. It does not invent a patient group from a case ID. Existing schema 2 documents retain their declared source references; the manifest separately fingerprints the input file. Such references do not independently authenticate an original source file.

Schema 2 requires `metadata` and `source`/`observation` on every finding. Local source references contain a caller-selected source ID, full source SHA-256, record ordinal and field locator. A record ordinal includes the CSV/TSV header as record 1; multiline CSV values therefore do not correspond to physical line numbers. JSONL ordinals are line numbers and a single JSON case is record 1. Preserve the original input under its source ID/checksum to make the chain auditable.

The provider state is built with an explicit allowlist. It excludes patient/sample/institution IDs, case IDs, data-class declarations, evidence cutoff metadata, source locators/checksums, labels and split assignments. Observed values, statuses, units, assay, timepoint and reference build are retained. Censored ages use `age_lower_bound_exclusive` with `age_years: null`. The request prompt is `cup-research-v0.2` for schema 2; the original schema 1 prompt and request digest are preserved. Result schema 2 records the case schema version; policy `unvalidated-research-gates-v0.2` adds abstention/preflight protection for cases with only unknown or not-tested findings.

## Outputs, failures and limits

| Artifact | Contents |
| --- | --- |
| `manifest.json` | Versioned source checksum, counts, every error/warning record, missingness, status/label/partition counts, case/request fingerprints and sidecar hashes |
| `cases/CASE_ID.json` | Compact canonical schema 2 case; accepted by `validate`, `prepare`, `demo`, `classify` and the TUI |
| `cases.jsonl` | All accepted cases, ordered by case ID; findings retain source order |
| `labels.json` | Accepted-case evaluation labels only; not loaded as evidence in the TUI |
| `splits.json` | Patient-group assignments, seed and holdout configuration |

The completion manifest is written last. A filesystem failure can leave an incomplete newly created directory; it has no valid completion manifest and must not be used as a successful run. No existing directory is overwritten or merged. Hashes detect accidental change relative to a manifest; they are not signatures or clinical source authentication.

The importer first snapshots the source into a private temporary file while hashing, then parses it record by record. This avoids a hash/parse race if the original path changes. Limits are 64 MiB per source, 50,000 data records, 2,000 assembled case groups, 16 KiB per JSON record/decoded tabular record, 64 findings per case, 16 KiB per compact canonical case and 64 KiB per prepared request. Temporary disk use is bounded by the source cap. CSV parsing can buffer a large malformed record up to that cap; it is not a constant-memory parser. Larger datasets require release-specific streaming/chunking work rather than raising bounds blindly.

Byte counts are deterministic. `token_budget_verified` remains false: a byte bound is not the provider's tokenizer. No evidence is silently truncated to fit a budget; the whole case is rejected. Source metadata can cause a migrated case to exceed the canonical budget.

Duplicate JSON case IDs, repeated finding IDs, inconsistent repeated case metadata/labels and duplicate specimen IDs across cases withdraw all affected known case IDs. A bad row that can be associated with a case also invalidates that whole case. Unparseable records with no recoverable ID are reported separately. `rejected_cases` counts identifiable invalid case IDs; `rejected_records` includes all excluded records, including records without a valid ID. Labels and partitions are emitted only for accepted cases.

Exit 0 means import completed without rejected records, including runs with quality warnings. Exit 3 means a partial or fully rejected import; the report and any accepted cases are still written. Exit 1 means a fatal options/source/IO error; exit 2 is invalid CLI syntax. JSON-mode reports go to stdout, sanitized fatal error envelopes to stderr. `--output` is disallowed for import because its report is already in the bundle. `nexus batch DIRECTORY` verifies the report and sidecar hashes; individual case fingerprints are checked when the TUI loads each case. Report inspection itself exits 0 even when inspecting a partial run.

## Partitions and remaining scientific work

`--split-seed study-v1` selects a deterministic SHA-256 partition of each patient ID into 60% development, 20% calibration and 20% test buckets. The exact algorithm is recorded in `splits.json`; it hashes UTF-8 `seed + NUL + patient_id`, reads the first eight digest bytes as a big-endian integer and takes modulo 10,000. No labels enter this calculation. All specimens with the same group stay together. Ensure IDs are globally unique across source institutions before combining files.

`--holdout-institution SITE-B` puts every patient with a specimen at that institution into test, including that patient's specimens from another institution. Missing patient groups stay `unassigned`. Buckets are not class-stratified and do not guarantee exact fractions in a small cohort. Changing a seed/holdout creates a different study split; a clinical/statistical reviewer still needs to approve and freeze a real cohort manifest before evaluation. Cross-file cohort reconciliation is not implemented.

Still pending: real dataset access/terms, raw GENIE patient/sample/mutation/CNA joins and orphan detection, genomic annotation/coverage normalization, reviewed clinical mappings, indirect leakage audit, approved cohort counts and locked scientific partitions. No accuracy/calibration claim follows from importing synthetic records.
