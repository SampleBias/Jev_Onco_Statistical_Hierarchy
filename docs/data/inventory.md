# Data inventory

Recorded 2026-09-21. Owner: data engineer with data steward and clinical curator. **No real patient cohort has been imported.** Counts below refer to local records unless explicitly marked as planning inputs.

| Asset / candidate | Version and access | Modalities and labels | Local count / rights status | Next step |
| --- | --- | --- | --- | --- |
| Bundled synthetic case | `fixtures/synthetic-case.json`, case schema 1 | Authored structured observations; no independent cancer-origin truth | 1 example; generated for this project, no patient source | Expand synthetic import/error fixtures in Phase 02 |
| Structured import fixtures | `fixtures/import/`, versioned with application 0.3.0 | Seven tabular observations across three authored cases/two groups; two renamed JSONL copies of the original example | Synthetic only; table labels are invented software-test inputs | Used by importer, schema, CLI and TUI regression tests; not an evaluation cohort |
| Contract-test responses | Rust test fixtures, pinned model and demo taxonomy | Artificial probability distributions; not scientific predictions | Generated in tests, not a cohort | Retain separately from measured Jev predictions |
| AACR Project GENIE | Candidate only; release not selected or downloaded | Candidate genomic/clinical evidence; exact fields and usable origin labels must be inspected for the selected release | 0 records; access and external-processing terms not reviewed | Follow the [official access page](https://aacrprojectgenie.org/data/), record release, files, checksums, permissions and assay metadata |
| Institution-curated pathology/IHC cases | Not supplied; prospective retrospective-data agreement | Candidate pre-cutoff histology/IHC/clinical evidence and independently adjudicated primary origin | 0 records; institution and rights unknown | Product/clinical lead identifies an authorized source and reference standard |

The upstream code repository is not a dataset. Its absent data files and trained artifacts are not prerequisites for this Jev implementation. Published performance from another classifier is not a label or a Jev benchmark.

## Required manifest before cohort use

Record asset ID, exact release, source URL/custodian, acquisition time, file checksum, modalities, institution/patient/specimen linkage policy, row and patient counts, source-label ontology/version, reviewer mapping version, evidence cutoff, permitted uses, permission for external inference, retention limits and responsible reviewer. Store records under ignored controlled data paths; do not commit real records to GitHub.

Keep ground truth and cohort/split membership in evaluation sidecars that cannot enter Jev state. Patient-linked records must stay in one split; duplicate specimens and repeated records need explicit handling. Missing assay coverage is not a negative molecular finding. The importer must produce counts of included, excluded, unmapped and invalid records before inference.

Release-specific counts, cohort feasibility and label balance are unknown until an authorized source is inspected. A target sample size is not an observed dataset size. See [feasibility protocol](../evaluation/feasibility-protocol.md).
