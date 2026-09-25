# Build status — 2026-09-25

## Sample-centered workbench refactor

The current interface supersedes the four-section navigation described below.
It starts empty, with Data / Results, Open, a contextual next action and Menu.
Cohort studies is a secondary workspace. Session ownership now binds evidence,
results and expression preparation to one active input. Previous samples and
invalidated expression runs remain explicitly separate in session history.

Molecular tables discover IDs and support selected-sample CSV/TSV/MAF imports;
VCF remains the supported annotated single-sample subset. Failed/cancelled imports
preserve the active sample. Expression import has five basic fields plus Advanced
settings. Inference-only JSON exports now include their features for reopening.
Legacy archives remain readable; clinical cases retain read-only records and charts,
without a separate live classifier or new guideline-review workflow in the main TUI.
All molecular/cohort charts, CLI/API contracts and the pinned Jev model are retained.

Verification: **222 workspace tests passed**; workspace build, formatting, diff
checks and all-target clippy with warnings denied passed. A real PTY confirmed
sample loading, request-confirmation cancellation and terminal restoration without
provider calls. Scope and remaining boundaries are in the
[delivery report](reports/sample-workbench.md).
The following sections describe earlier increments, not current navigation.


## 0.8.0 — unified workbench UX

One shared action bar, navigation, focus model and form renderer now serve Analysis,
Expression, Clinical and Cohort. Main controls and form fields/buttons are clickable;
Tab/Shift-Tab moves focus and Enter activates it. Less-frequent actions are in Tools.
The new browser supports directories, supported files, recognized run/bundle folders,
filtering, hidden-file visibility, quoted/spaced paths, home-directory shorthand and
session-only recent paths. Reference releases use the same browser and loader.

Samples exposes all five synthetic molecular inputs directly from the binary,
using the real importer; no generated files or metadata forms are needed. Loading
does not call Jev. The fixed analytical chart demo is separate and asks before
replacing an input. Confirmation-only forms focus Cancel by default. Existing
validation, synthetic-only live inference, protected exports, job lifecycle and
cross-section unsaved-review safeguards remain in force.

Removed UI-only redundancy: repeated section headers/action bars, the placeholder
Models and Projects pages, and the duplicate Clinical Help tab in normal navigation.
Model identity remains in the common header; provenance stays with the data; shared
Help remains searchable. All working analytical methods, visualizations and CLI
workflows are retained. Tab now focuses controls rather than immediately switching
views; click a view or use Tab then Enter. F2–F5 and existing analysis shortcuts remain.

Verification includes bidirectional focus traversal, mouse/keyboard sample loading,
form submission/export, background-click isolation, failed-load state preservation,
asynchronous error visibility, form focus after section switches, demo replacement
confirmation, Unicode and tiny layouts. Actual Ratatui screens were rendered for
review at 120×40, 80×24 and 60×20, with the paired figure also checked at 160×50
and 60×55. No dependencies, model changes or live provider requests were introduced.

Final verification: **219 workspace tests passed**; locked workspace build,
formatting, diff checks and all-target clippy with warnings denied passed. The two
transport tests used localhost mock servers with sandbox approval. A real PTY smoke
check verified mouse-opened Samples, loading without paths, the one-request
confirmation, cancellation without sending, and terminal/mouse restoration on exit.

## 0.8.0 — discoverable OncoNPC explanation and research guide

Analysis adds a dedicated **OncoNPC [p]** view with linked category/feature rings,
signed contribution bubbles, feature selection, raw values and numerical
reconciliation. A clickable view strip and p shortcut make it accessible without
discovering the old Tab sequence. The paired figure uses compact chrome at 80×24,
stacks on tall narrow screens and falls back to a table on very small screens.
Existing Results, Ring, Scatter, Waterfall, Data, Clinical and Cohort views remain.

The published paper was already in Sources. Its new
[research guide](references/ONCONPC_GUIDE.md),
[current objective audit](assessment/ONCONPC_PARITY.md) and source register are
compiled into the searchable offline help. The audit supersedes the pre-build
assessment and identifies missing predictive validation, PRS, actionability and
treatment-analysis methods. The roadmap now points to that research direction.

Verification covers keyboard/mouse access through the shared workspace, preserved
view order and section state, resize down to 1×1, offline retargeting, unchanged
archives/exports, zero contributions, selection beyond the top ten and offline
reference search. Actual terminal buffers were rendered for visual review at
160×50, 120×40, 80×24 and 60×55. No dependencies, inference model changes, live
provider calls or real patient data were introduced.

Jev's cancer-prediction performance and treatment utility remain unmeasured.

## 0.8.0 — guided Jev analysis and Markdown reports

The single terminal application presents Load → Analyze → Results → Export actions,
with mouse and keyboard operation. Direct molecular JSON and saved runs open
locally; canonical tables and annotated MAF/VCF subsets use editable import
settings. Local checks explain missing credentials, unavailable observations and
the synthetic-only provider boundary. The launcher `./scripts/start` checks build
prerequisites and starts the pinned Rust application.

One-request inference automatically archives inputs, the exact request, validated
inference, an attempt-status record and `report.md`. Inference-only runs reopen
offline and display readable rankings, abstention reasons and separate evidence
checks. Optional bounded explanations add `explanation-report.md` while retaining
the original report. Markdown exports verify run consistency and contain evidence,
missingness, full rankings, explanation details when present, usage and provenance.
Failed imports preserve prior data; replacing Analysis input invalidates its previous
result, while loading Data/Clinical records preserves Analysis;
exports protect existing files. Provider failures never substitute demo results or
trigger automatic retries. Interrupted attempts retain an explicit uncertain state.

One event loop now owns the Analysis, Data/reference and Clinical sections, shared
Load/help navigation, pending jobs and application-wide unsaved-review checks.
There are no nested molecular applications or alternate terminal runners.
F2/F3/F4 switch sections without dropping data, forms or results; compatible
expression comparisons attach to the existing Analysis section.

Migration: `josh tui [PATH]` is the only terminal entry. The former per-interface
flags were removed; Load detects molecular/clinical JSON, expression/molecular
tables and saved bundles. `molecular export` defaults to Markdown; use `--kind svg`
for the former default. JSON contracts are unchanged.
See [quickstart](QUICKSTART.md) for credentials, complete steps and supported inputs.

Verification: **182 workspace tests passed**, including shared import → injected
provider → archive → reload → Markdown tests, table-form errors, stale-result
invalidation, failed/uncertain attempts, credential redaction, tampering and Markdown
escaping, cross-section form/result retention, hidden-job completion, automatic
file routing and unsaved clinical review protection across sections.
Formatting, clippy with warnings denied, schema drift, locked workspace
build and the 278-package dependency inventory passed. A real PTY smoke check passed
one terminal initialization, F2/F3/F4 navigation, mouse Load, expression import form
retention, saved results, Markdown export, resize, guide and terminal/mouse
restoration. The two HTTP transport tests required localhost
socket access outside the filesystem/network sandbox; they used local mock servers.

No live provider request or real cancer data was used for this phase. Configured-key
checks do not authenticate credentials. Live use still requires TYPESAFE_API_KEY and
synthetic inputs; real-data eligibility, external cohort validation and calibration
remain open. The earlier 0.7.0 live check below is a separate historical result.

## 0.7.0 — molecular Jev inference and native Shapley graphics

Implemented the approved OncoNPC-inspired workflow in Rust, Jev and Ratatui:
typed mutation/CNA/SBS/demographic/IHC/histology/expression features; strict
CSV/TSV, MAF and VCF subsets; frozen taxonomy-aware Jev runs; grouped exact and
permutation Shapley; compatible development backgrounds; budgets, cancellation,
uncertain-attempt accounting and atomic resume checkpoints; circular/scatter/
waterfall graphics; offline class switching; and protected SVG/CSV/JSON exports.
Native experimental SBS96/NNLS and synthetic cohort/evaluation tooling are also
available. The existing expression and legacy workflows remain usable.

The [molecular guide](data/MOLECULAR_GUIDE.md) gives commands and supported formats.
The [engineering/scientific report](reports/0.7.0-molecular-validation.md) records
verification and limitations, including the live synthetic run: 14 valid coalition
evaluations within 16 attempts, zero additivity residual, and correct abstention.
Four repeated-input requests measured full-input score variation of 0.61–0.63.
No real cancer data, cancer performance study or clinical calibration was used.

Verification: **172 workspace tests passed**; formatting, clippy with warnings
denied, generated-schema drift and the 278-package dependency inventory passed.
Real PTY interaction and exported SVG inspection also passed. The tests include
zero/tied chart contributions, visible remainder totals, XML escaping, uncertain
request recovery and reported token usage exceeding its reservation.

The earlier sections below are historical release snapshots. Their statements
about missing molecular inference, attribution or provider verification describe
those earlier releases and are superseded by the 0.7.0 report.

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
