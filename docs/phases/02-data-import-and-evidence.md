# Phase 02 — Data import and evidence preparation

Status: structured import milestone implemented in 0.3.0; raw GENIE mapping, clinical/data review and real-cohort acceptance remain pending. Lead: data engineer/bioinformatician. Reviewers: clinical curator, backend and ML lead. Estimate: 8–15 person-days, excluding data-access waits. Depends on: Phase 00 data scope and Phase 01 contracts. Unlocks: synthetic cohort workflows and provider integration tests.

## Delivered milestone

`nexus-ingest` now imports canonical JSON/JSONL and normalized CSV/TSV findings, produces schema 2 cases with source references and observation status, and writes protected bundles with every rejected record reported. `nexus import` and `nexus batch` expose the workflow. The TUI opens bundles with `--batch`/`b`, browses cases with `[`/`]`, and shows a fifth Import quality view. Changed case fingerprints prevent loading, and case changes clear previous results.

See the [import guide](../data/IMPORT_GUIDE.md), [synthetic fixtures](../../fixtures/import/README.md), [generated contracts](../../contracts/README.md) and [build evidence](../BUILD_STATUS.md).

| Package | Implemented evidence | Remaining acceptance work |
| --- | --- | --- |
| P02-01 | Bounded source snapshots, JSONL/CSV/TSV record readers, whole-case rejection and reports | Release-specific raw source adapters |
| P02-02 | Schema 2 source references, status, units, assay, timepoint, reference build, local specimen metadata and censored age; schema 1 migration | Clinical field/assay review |
| P02-03 | Findings joined by case ID; string IDs, duplicate findings/cases/specimens, repeated metadata conflicts and multiple patient specimens checked | Separate GENIE patient/sample/mutation/CNA joins and orphan detection |
| P02-04 | Reference-build/assay metadata preserved | Genomic coordinates/alleles/CNA/coverage normalization and numeric feature methods are not implemented |
| P02-05 | Narrow lexical status aliases, explicit missing/not-tested/negative distinctions and contradiction rejection | Clinical interpretation, intensity/method mappings and terminology review |
| P02-06 | Tabular labels in a separate sidecar; canonical unknown fields rejected; explicit provider-state allowlist | Reviewed source ontology mapping and indirect prose-leakage audit |
| P02-07 | Deterministic patient-group partitions, institution holdout and unassigned missing groups | Actual cohort grouping across files, statistical review and frozen scientific splits |
| P02-08 | Stable ordering, case/request fingerprints, source/record/case/request byte caps, overflow rejection | Provider tokenization remains unverified and explicitly marked false |
| P02-09 | Local-only imports; existing synthetic-only live policy retained | Dataset rights, deidentification, external-processing and retention approval |
| P02-10 | Synthetic fixtures and import, CLI, TUI, schema and provenance tests | Quality review of an authorized real cohort |

This completes the first structured import implementation, not the entire scientific/data phase. Engineering for Phase 03 can now use these fixtures without waiting for real records.

## Outcome

Convert permitted source records into reproducible, compact case evidence without leaking the known origin. Keep an auditable chain from every finding to its source. “Port data” means importing records and preparing evidence, not training Jev weights.

## Import tracks

| Track | Input | Output | Initial disposition |
| --- | --- | --- | --- |
| A | Canonical JSON or JSONL | Validated case revisions | First batch importer |
| B | CSV of IHC/histology/clinical findings | Normalized observations with provenance and missingness | First research cohort path |
| C | GENIE mutation, patient, sample and CNA TSVs | Joined case records and explicit molecular observations | Genomic feasibility track |
| D | RNA matrices, raw VCF/FASTQ, slide images or PDF OCR | Modality-specific preprocessing before case evidence | Separate later scope; not a direct Jev input |

Use the [GENIE data access page](https://aacrprojectgenie.org/data/) for permitted release acquisition. Pin a release and record its actual schema rather than assuming the source project's hard-coded 5.0 filenames still apply. The original four conceptual files are `data_mutations_extended.txt`, `data_clinical_patient.txt`, `data_clinical_sample.txt`, and `data_CNA.txt`; confirm actual names and fields in the selected release.

## Work packages

| ID | Owner | Task and concrete output |
| --- | --- | --- |
| P02-01 | Data engineer | Create `nexus-ingest`, streaming JSONL/CSV/TSV readers and per-row validation reports; batch failures must not silently discard records |
| P02-02 | Curator + backend | Extend case schema with source reference, units, assay/panel, specimen/timepoint, reference build and observation status; provide migration from schema v1 |
| P02-03 | Data engineer | Implement patient-to-sample joins; detect duplicate IDs, orphan samples, multiple specimens and patient overlap; never join on a truncated or floating-point ID |
| P02-04 | Bioinformatician | Define genomic normalization: gene identifiers, reference build, alleles, variant annotations, CNA encoding and tested-gene coverage; compute numeric summaries in code |
| P02-05 | Clinical curator | Normalize IHC to positive/negative/equivocal/not-tested/unknown with relevant intensity and method; review abbreviations and conflicting observations |
| P02-06 | Data engineer | Separate labels from inference features, strip known-primary fields and diagnostic conclusions, and produce blinded case payloads plus a protected label table |
| P02-07 | ML lead | Freeze patient-group development/calibration/test partitions, institution holdout where feasible, and source-to-split manifests |
| P02-08 | Backend + curator | Implement deterministic evidence packing, byte/token budgeting and explicit overflow errors; retain every exclusion in an import report |
| P02-09 | Data steward | Document dataset rights, external processing permission, retention and deidentification review; enable authorized research inputs only after review |
| P02-10 | QA | Produce synthetic edge fixtures and data-quality summaries covering missingness, rejected rows, joins, coverage and class distribution |

## Evidence preparation rules

Unknown, not assayed and measured negative are distinct states. A gene absent from a panel is not wild type; missing CNA is not diploid. Age must preserve censoring such as “>89”, not discard a character. Genomic coordinates require a declared build. A biopsy in the liver does not establish a liver primary. Retain stage/site information only if available at the stated prediction time and not a disguised ground-truth label.

Do not ask Jev to infer sequencing signatures from raw matrices or perform arithmetic. If signature features become necessary, document the method, reference release and quality requirements, verify it against a trusted implementation, then provide meaningful annotations. Sparse-panel signature validity must be assessed by the bioinformatics reviewer. Reference assets are separate dependencies with their own provenance.

The first payload should include only observations relevant to the chosen questions. No automatic summarizing generative model is required. If one is later introduced for reports, extracted facts need source-span validation, explicit error evaluation and their own versioning.

## Acceptance criteria

- Reimporting identical sources produces the same case content, versions and checksums.
- Every accepted finding has an identifiable source; duplicates/orphans and rejected rows are reported.
- Known-origin fields and evaluation labels never appear in inference payloads; reviewers audit indirect leakage through report wording and institution/sample identifiers.
- Multiple specimens from one patient cannot cross development/calibration/test partitions.
- A modest labeled cohort can run end to end; its sample size and class coverage support only the claims stated in its report.
- Input terms permit the intended processing. Synthetic fixtures remain the fallback when access is pending.
- Missing, negative and unmeasured values remain distinguishable through the complete pipeline.

## Handoff and risks

Deliver versioned importers, schemas, synthetic fixtures, data dictionary, source manifests, blinded case files, protected labels, split manifests and a quality report. Main risks: label leakage, unrepresentative known-primary cohorts, incomplete assay coverage, and conflating different modalities. Address them before increasing cohort size or comparing scores.
