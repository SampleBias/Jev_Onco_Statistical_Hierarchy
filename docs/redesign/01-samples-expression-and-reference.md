# Redesign Phase 1 — Samples, expression and reference comparison

Status: in progress. The first sample/expression workbench milestone is implemented
in 0.5.0; the full phase is not complete. Assessment baseline: JOSH 0.4.0 (`757692e`).

Delivered: separate Sample/Dataset/Assay/artifact contracts, v1–v3 legacy Case
attachment import, long/wide CSV/TSV expression parsing, configurable detection,
HGNC mapping, explicit identity/log2(x+1) transforms, QC, immutable bundles,
local reproduction, measurement exports, and Ratatui data navigation/import/paste.
See the [implemented behavior and limits](../data/EXPRESSION_GUIDE.md).

R1-01–05 and R1-10–12 have partial deliverables. Canonical standalone-sample import,
annotation joins, additional normalization, reference releases/comparisons,
EvidencePackage/AnalysisRun contracts, molecular Jev calls and scientific evaluation
remain open. R1-06–09 are not complete. The clinical compatibility module remains
in the existing core crate, but is not required by the expression pipeline.
Direction: [gap assessment](../assessment/DATA_FIRST_GAP_ASSESSMENT.md).
Leads: Rust backend engineer and bioinformatician. Reviewers: research/statistics
lead, data curator and terminal UX/QA engineer. Assign individual owners before
scheduling; no team availability or delivery duration has been established.

## Outcome

A researcher imports bulk human gene-expression data without creating a patient
profile, checks mapping/QC/provenance in Ratatui, compares one sample against a
versioned known-primary reference, and receives an archived structured Jev ranking
with evidence and uncertainty. Rust owns all local processing and orchestration.
Jev remains the sole origin classifier. The CLI uses the same services as the TUI.

## Implementation sequence

Deliver in order: **R1-01–03 → R1-04–06 → R1-07–09 → R1-10–12**.
Contract fixtures let UI work proceed alongside the corresponding service work.
Reference curation and provider transport testing can proceed while import is built.
Do not call the phase complete before a real compatible reference and held-out
evaluation exist; a synthetic demonstration is an intermediate engineering milestone.

| ID | Owner | Work package and concrete artifact |
| --- | --- | --- |
| R1-01 | Technical lead + backend | Define Sample, Dataset, Assay, ModalityArtifact, FeatureSet, QcReport, ReferenceRelease, EvidencePackage, AnalysisRun and AnalysisResult; generate versioned JSON Schemas |
| R1-02 | Backend | Add legacy v1–v3 Case adapter and migration report; preserve clinical records and old result/request versions; separate optional clinical code from new sample analysis |
| R1-03 | Data engineer | Define DatasetAdapter detection/parse/normalize/validate/conversion boundaries; implement CSV/TSV expression long/wide tables, canonical JSON, annotations and bounded paste/stdin ingestion |
| R1-04 | Bioinformatician | Implement pinned gene mapping and duplicate/ambiguity policy; emit mapping table and source-to-canonical lineage |
| R1-05 | Bioinformatician | Implement explicit expression scales/transforms, raw/derived artifacts, feature generation and per-sample QC; freeze compatibility rules |
| R1-06 | Curator + statistics lead | Acquire/curate eligible known-primary reference data, class mappings and group-separated partitions; publish immutable reference manifest and data dictionary |
| R1-07 | Bioinformatician + backend | Implement numerical reference comparisons, overlap and coverage checks, gene percentiles and explicit unsupported/OOD assessment status |
| R1-08 | Backend + ML lead | Prepare bounded structured evidence; adapt Jev question/result contract; construct evidence/conflict/missingness fields in Rust |
| R1-09 | Backend + QA | Harden shared Jev client with injected test transport, bounded retry policy, cancellation and job identity; archive a synthetic live contract check before authorized research calls |
| R1-10 | Backend | Persist stage states, immutable manifests and source/request/response artifacts; local replay and JSON/CSV/TSV exports |
| R1-11 | Terminal UX + backend | Ratatui Samples/Datasets/Analyze/Explore/Models/Reference/Projects, sample dashboard, import correction, gene search and source drill-down; matching CLI operations |
| R1-12 | QA + statistics lead | End-to-end/scale/failure testing, frozen expression evaluation, model/pipeline card, documentation and release evidence |

## Contract decisions

- Required identity is a Sample ID independent of patient identity. Keep optional
  patient grouping for evaluation and separate assay/aliquot/timepoint identities.
- A Dataset carries source/study/accession, citation, access/use metadata, samples,
  annotation schema and feature summary. A sample may participate in multiple named
  cohorts without duplicating its source artifacts.
- Use immutable raw and derived artifacts referenced by checksum. Keep row/column
  or record locators, original identifiers/values, import timestamp and transformation
  DAG. Numeric artifacts have explicit dtype, shape, feature order, units and scale.
- Keep the old Case schema namespace intact. Introduce a distinct sample contract;
  do not reinterpret schema 3 as a molecular matrix. Unsupported legacy findings
  remain annotations with a migration warning, not invented numeric features.
- Represent modality availability as absent, present, failed-QC or unsupported,
  with independent analysis compatibility. Extra modality artifacts must not force
  a redesign later; each type has a schema version and validation boundary.
- Store query ground truth outside inference evidence. Reference class labels are
  legitimate reference metadata; mask query labels and revealing identifiers.

## Import and gene mapping requirements

Support a two-column single-sample expression file, long form with explicit sample
column, and wide gene-by-sample matrices. Offer orientation/column selection for
ambiguous or transposed data; do not silently guess units. Keep CSV quoting, input
bounds, duplicate handling, partial-import reports and protected output behavior.

Detection returns candidate format/modality, observations supporting that choice,
uncertainty and required user-supplied metadata. Species, genome, units, scale and
platform may remain unknown at import; block incompatible inference with a reason.
Users can revise the import configuration without modifying the source file.

Mapping handles HGNC symbols/IDs, Ensembl gene IDs and Entrez IDs using a pinned
mapping asset. Preserve Ensembl version suffixes in the original field; record any
version removal. Explicitly report unique mappings, aliases, withdrawn/ambiguous
entries, unmapped IDs and duplicate target genes. Do not choose the first ambiguous
match or silently sum duplicates. Use a reviewed aggregation policy per data type.
Start with a versioned [HGNC download](https://www.genenames.org/download/), verifying
the actual available fields before implementing its reader.

## Expression and QC requirements

Accept and describe counts, TPM, FPKM, normalized expression, log-transformed values,
z-scores and labeled microarray-derived values. Acceptance into storage is distinct
from eligibility for comparison. The first inference path should use one explicitly
matched processing family; additional paths need their own reference/evaluation.
Do not imply support for raw FASTQ alignment, raw microarray CEL processing or
single-cell interpretation from this bulk gene-expression milestone.

Raw values remain immutable. Record each transform and reference-fitted parameter;
preserve raw, normalized and selected/scaled feature artifacts. Reject non-finite
numbers; validate signs and ranges against the declared representation (negative
z-scores may be valid). Never auto-convert a partial panel into whole-transcriptome
TPM or treat a log transform as cross-platform harmonization.

QC includes total/measured genes, duplicate IDs, mapping success, zero/missing
values, unit validity, distribution summaries, feature overlap and reference
compatibility. Name the mapping-rate denominator. Distinguish gene-ID mapping rate
from sequencing alignment rate; the latter cannot be recovered from expression
values alone. Missing expected genes are relative to the selected feature set,
not a universal genome count. Thresholds are versioned and evaluated.

Tests must cover finite/non-finite and representation-valid values, missing versus
zero, duplicate genes, ambiguous mappings, reversed matrix orientation, quoted
sample IDs, versioned Ensembl identifiers, invalid unit declarations and transform
idempotence/accidental double application. Compare numerical outputs to independently
worked fixtures or trusted published methods, not tests that simply repeat the code.

## Reference and inference requirements

The reference release includes cohort/source version, known-primary label mapping,
sample selection, class counts, patient-group exclusions, species, platform,
annotation, units, transformations, selected features and numerical comparison
method. Reference assets stay outside normal source code; a reproducible acquisition
manifest accompanies them. A manually acquired compatible export is enough for this
phase; repository accession downloads belong to Phase 3.

Build summaries on the reference/development partition only. Exclude overlapping
patients/samples from held-out evaluation; fit scaling and feature selection without
query leakage. Validate a candidate reference comparison method on development data,
then freeze it. Report overlap, class support and missingness alongside similarity.
Unsupported platform or missing reference yields a useful QC/exploration result and
an explicit blocked analysis state, not a fabricated prediction.

The Jev EvidencePackage contains selected computed measurements, comparison method,
reference version, bounded evidence IDs, QC and modality availability. Full matrices
remain local. Runtime derives evidence/provenance arrays from stored artifacts;
Jev provides typed judgments. Preserve the raw response and distinguish Jev's
probabilities from local similarities in all interfaces.

Implement distinct reasons for insufficient features, failed QC, incompatible
reference, unavailable modality, unsupported origin and suspected OOD. OOD status
must be `not_assessed` until a method/reference and rejection evaluation exist.
Measure known-class rejection and withheld-class/shift behavior; do not equate a
provider confidence threshold with reference coverage.

Carry forward old Phase 03 transport work: timeout, auth failure, 429/retry-after,
malformed/oversized streaming body, model drift, cancellation and duplicate jobs.
Retries must be bounded and recorded; an ambiguous timeout must not silently trigger
unbounded additional charges. Do not remove the synthetic-only restriction merely
to make a demo run: replace it with dataset-specific external-processing eligibility
once the selected real source/provider terms are established.

## Terminal workflow and exports

1. Open/create a local project; import a file or paste expression data.
2. Review detected modality/orientation and supply missing configuration.
3. Inspect mapping, QC, provenance, dataset dictionary and sample list.
4. Explore top expressed genes, gene/family search where mapping supplies membership,
   distributions and reference percentiles without contacting Jev.
5. Select a compatible reference and sample; preview stages and structured evidence.
6. Run Jev and inspect rankings, evidence and explicit uncertainty.
7. Export full analysis and replay it from archived artifacts.

Retain useful score/IHC widgets and keyboard conventions. Replace the clinical
startup example and main NICE/review tabs. Show a processing pipeline and analysis
history in place of the primary clinical pathway/timeline. Clinical attachments
remain accessible through compatibility views when present.

Long processing runs asynchronously; users can inspect other views and cancel.
Render virtualized/limited table windows over large artifacts. Exports include full
JSON manifest/result and linked CSV/TSV tables; preserve all ranking outcomes.
PDF generation is Phase 2. All proposed CLI commands are to be specified with the
new service API; existing commands must not be advertised as supporting new inputs.

## Exit criteria

- All steps above work with a real, eligible expression input and frozen compatible
  reference. Synthetic fixtures additionally cover errors and boundaries.
- A 20,000-gene sample is represented without putting every gene into Case findings
  or the Jev state. Profile a representative multi-sample matrix and record memory,
  disk, elapsed time and limits on the supported hardware; do not claim scale from
  a two-gene fixture.
- Reference incompatibility, unknown units, low feature overlap and failed QC are
  visible before inference. Missing clinical/demographic data never blocks import.
- Every displayed evidence value resolves to source and transformation metadata.
- Local recomputation from archived inputs reproduces features/request hashes;
  saved-response replay works offline. A fresh provider call is a new analysis run.
- Held-out expression evaluation reports class counts, top-k accuracy, macro metrics,
  errors, abstention coverage and confidence intervals; calibration remains null
  unless separately justified. Define numerical success thresholds before testing.
- Ratatui input/navigation/resize/cancellation tests and CLI/schema integration checks
  pass. No clinical code is required for a sample's core pipeline.
- Update active architecture, intended use, guides and build status to reflect only
  delivered capabilities. Record unresolved scientific limitations explicitly.

Handoff: schemas and migration report; adapter/gene-map assets; QC and numerical
verification; frozen reference manifest; Jev contract/transport report; analysis
archive format; TUI/CLI workflows; evaluation/model card. Next: [Phase 2](02-multimodal-and-cohorts.md).
