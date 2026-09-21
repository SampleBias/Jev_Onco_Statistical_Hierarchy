# Phase 02 — Data import and evidence preparation

Status: planned; only canonical single-case JSON is currently implemented. Lead: data engineer/bioinformatician. Reviewers: clinical curator, backend and ML lead. Estimate: 8–15 person-days, excluding data-access waits. Depends on: Phase 00 data scope and Phase 01 contracts. Unlocks: reliable Jev experiments and evaluation.

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
