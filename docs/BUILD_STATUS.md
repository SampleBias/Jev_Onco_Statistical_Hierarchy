# Build status — 2026-09-22

## 0.6.0 — reference comparison, terminal design and searchable guide

The header reads Jev Onco Statistical Hierarchy / Molecular Data Workbench. Dataset
classification stays in provenance. Ratatui now renders themed panels, quality
cards, an observed-expression histogram, signed correlation bars and job activity.
The shared offline guide opens with g in navigation or F1/Ctrl+g inside editors,
with bounded regex search, highlights, match navigation and full-context viewing.
It preserves forms and works in the legacy interface as well.

Local reference build/inspect/compare/prepare commands now curate explicitly labeled
TPM/log2 datasets into fixed-gene class means. Comparison checks processing metadata,
gene dictionary, source/sample/group overlap, QC and gene coverage. Evidence exports
preserve all signed correlations and explicit incompatible/insufficient/undefined
states. Jev request previews use typed questions and omit query identifiers.
The [reference guide](data/REFERENCE_GUIDE.md) and [embedded user guide](USER_GUIDE.md)
record exact scope and operational limits.

Verification: 148 workspace tests passed, including numerical oracle checks, reference
rejection paths, JSON Schemas, reproducible comparisons, request identifier exclusion,
protected exports, guide keyboard/search behavior, legacy access, asynchronous jobs
and stale-result invalidation. Formatting and clippy with warnings denied passed;
the dependency inventory remains 277 packages (regex was already locked). A real
PTY check passed branding, guide search/context, help over an unfinished form,
reference loading/comparison, offline request export, resizing and terminal
restoration. The actual Ratatui buffer was also visually inspected. No provider
request or real cancer cohort was used.

Phase 1 remains in progress: curated/validated cancer references, scientific splits,
reference percentiles, live molecular provider response handling, full analysis-run
archives, evaluation, calibration and a validated OOD detector remain open. The
synthetic tutorial's Demonstration A/B labels are not cancer reference classes.

## 0.5.0 — first data-first redesign milestone

Implemented independent Sample/Dataset/Assay/artifact contracts; long/wide expression
CSV/TSV detection/import; HGNC symbol/ID, Ensembl and Entrez mapping; explicit
identity/log2(x+1) transforms; QC; immutable source/measurement bundles; local
reproduction; JSON/CSV/TSV measurement exports; and a default Ratatui data workbench
with import settings, detection preview, paste, sample navigation and gene search.
Legacy case schemas 1–3, provider payloads and the clinical interface remain available.
The [expression guide](data/EXPRESSION_GUIDE.md) records exact behavior and limits.

Verification: 134 workspace tests passed, including 34 new feature/import/CLI/TUI
checks; rustfmt and clippy with warnings denied passed; generated schema drift and
actual dataset-schema checks passed; dependency inventory records 277 packages.
A real PTY smoke test passed sample-first startup, exploration/search, import
settings, bracketed paste, resizing, quit and terminal/paste-mode restoration.
This verifies software behavior, not cancer classification.

A debug-build scale check used 20,000 real approved HGNC symbols with invented
measurements for four samples and the full public HGNC dictionary: all 80,000
records mapped, import took approximately 5.4 seconds with 160,368 KiB peak child
RSS, and local reproduction passed. This was a synthetic scale check on this Linux
machine, not a reference-cohort experiment or a general performance guarantee.

The full redesign Phase 1 remains in progress. No reference-cancer comparison,
molecular Jev inference, molecular OOD detector, calibration, real cancer cohort,
cohort inference, repository connector or PDF report was delivered by this milestone.
Clinical guidance is available through explicit legacy entry points; no records were
removed. No live Jev request was made during this implementation.

## Historical 0.4.0 verification


## Implemented and verified

Application 0.4.0 adds five themed Ratatui visualization views, schema 3 local clinical
context, a versioned/source-linked subset of NICE CG104 review rules, and reasoned
review records with current/stale fingerprints and protected persistence. CLI
`guidance`, `example --clinical`, guidance schema and offline `POST /v1/guidance`
expose the same workflow. The existing four-crate structured-import/classifier
foundation is retained. Phase 00 clinical/data approvals remain pending.
See [exact rule coverage and limitations](CLINICAL_REVIEW_GUIDE.md).

The current scope uses Jev as the sole classifier. Original classifier training/inference, model artifact recovery and explanation tooling are excluded from the delivery plan. None of those components was included in the Rust source or dependency manifests, so this scope update required documentation changes only.

| Check | Result |
| --- | --- |
| `cargo build --workspace --offline --locked` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --workspace --all-targets --offline --locked -- -D warnings` | Passed |
| `cargo test --workspace --offline --locked` | Passed: 100 tests |
| CLI `validate`, `prepare`, `demo` on synthetic case | Passed |
| CLI `classify` without credentials | Expected local error; no request sent |
| CLI integration tests | Pipes, standalone demo, JSON/text, file protection, credential masking, help/version, structured errors, schemas, trailing JSON and exact byte budgets passed |
| TUI tests | Navigation, reload invalidation, exports, narrow terminals, failed imports and live-request guards passed |
| Interactive PTY smoke test | Passed: bundle launch, censored age, next/previous cases, import report, mock result, resize, quit and terminal restoration |
| Generated schemas/OpenAPI | Drift check, reference resolution, schema validation and actual offline API response validation passed |
| Dependency inventory | 276 workspace/registry packages recorded; offline drift check passed |
| Batch imports | Source hashes, migration, row/byte caps, duplicate/conflict rejection, label separation, patient holdout and protected export tested |
| TUI batch workflow | Case navigation, source/status/censored-age display, quality report export, changed-case detection and result invalidation tested |
| NICE clinical rules | 18 core tests covering scope, unknown/false semantics, conditional investigations, stage checks, benefit/MDT gates, withdrawn recommendations and review integrity |
| Clinical UI/API/imports | Rule detail scrolling, private provider state, schema 3 bundle round-trip, invalid clinical input, review persistence and unsaved/discard safeguards passed |
| Visualizations | All outcomes, fixed score scale, zero values, related RGB colors, distinct IHC statuses, missing-date handling and small-terminal rendering tested |
| 0.4.0 interactive PTY smoke | Synthetic launch, demo bars, IHC, pathway, timeline, guidance, review-dialog cancellation and terminal restoration passed; no provider request |
| Markdown local-link check | No missing targets |
| Phase plan files | Eight separate Markdown files |

Environment: Linux, Rust/Cargo 1.98.1. Existing locked platform-specific crates missing
from the cache were fetched for the full dependency inventory; no chart or clinical
dependency was added. Checks then ran offline. `Cargo.lock` records resolved versions.
Supported OS and compatibility rules are in the [foundation handoff](engineering/foundation.md).
These 0.4.0 verification results are local; no remote CI result is claimed here.
The earlier [Phase 01 GitHub run](https://github.com/SampleBias/Jev_Onco_Statistical_Hierarchy/actions/runs/35667838559)
remains historical evidence, not verification of this release.

## Not yet verified or implemented

- The user configured a key in their terminal; this build session does not inherit that environment. No live Jev prediction, provider latency or actual billed usage was measured here.
- Tests validate software contracts and offline API behavior; the HTTP client still needs mock-server failure tests and a live synthetic smoke test in Phase 03.
- No real case records or GENIE cohort were imported. No model training, CUP evaluation or calibration was performed.
- No raw GENIE release mapper, genomic coordinate/allele/CNA/coverage normalization, browser UI, database, user authentication, queue, retries or production deployment exists yet.
- Patient-group partitions are deterministic engineering artifacts. Real cohort linkage, ontology mapping, indirect leakage review, scientific split approval and cross-file reconciliation remain pending.
- The origin taxonomy and gate thresholds are development examples, not clinically approved definitions or operating points.
- Clinical rules are a documented diagnostic/review subset, not the entire guideline,
  a treatment engine or clinical validation. Independent CUP oncologist/pathologist
  signoff remains pending. The 2023 withdrawn genomic prohibitions are not active rules.
- Clinical assertions currently require standalone JSON editing. Review entries are
  locally versioned but self-reported, not authenticated or tamper-proof audit records.
- No real cohort/outcome data were invented to populate calibration or survival charts.
- The runtime's synthetic-data declaration is not a deidentification detector.

The first live milestone is a synthetic contract smoke test. The first scientific milestone is a blinded, labeled cohort experiment with the evidence types the project will actually use. Both have owners and acceptance criteria in the [phase plan](PLAN.md).
