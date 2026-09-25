# Jev Onco Statistical Hierarchy (JOSH)

A Rust molecular-data workbench for cancer-of-unknown-primary research, with a
Ratatui terminal application and a CLI. Version **0.8.0** provides one workspace
for **Load data → Analyze with Jev → View results → Export Markdown**.

Jev is the hosted origin classifier. Live requests currently accept declared
**synthetic data only**. Local imports, reference comparison, saved-run review and
exports work without a key. No cancer-specific accuracy or clinical calibration
has been established; bundled analytical demos use invented measurements and scores.

## Start here

From the repository:

```bash
./scripts/start
```

The launcher checks for Cargo, Rust and a system C compiler, builds the binary,
then opens JOSH. The repository pins Rust **1.98.1**; a first build needs registry
access. The launcher does not install prerequisites. No Python, Node, local GPU,
database or model download is required. See the [quickstart](docs/QUICKSTART.md)
for setup and the complete walkthrough.

The easiest first run: **Try a sample → Open selected → Data → Run analysis**.
The five synthetic inputs are built in; loading them is local and does not call Jev.
Review the one-request confirmation before sending.

The primary workspace has **Data** and **Results**, an **Open** browser, one
contextual next action and **Menu**. Tab/Shift-Tab, Enter and mouse clicks work
throughout. Results' **View ▾** groups all existing charts, including paired
ring/scatter and waterfall. Explanations require a separate budget confirmation.

**Menu → Offline chart demonstration** provides explicitly invented scores without
a key. **Menu → Previous samples** restores earlier inputs and their results.
Failed or cancelled imports preserve the active sample. Imported tables now detect
sample IDs; multi-sample molecular tables offer a selector.

## One sample-centered workspace

- **Data:** evidence, provenance, missingness and readiness; expression is an
  optional input profile, not a separate app.
- **Results:** rankings, abstention, explanations and protected exports.
- **Workbench ▾ → Cohort studies (F5):** all existing classification, survival,
  treatment, calibration and study-export tools.
- **Legacy case files:** explicitly labelled read-only evidence, visualizations,
  guidance and existing reviews; no second clinical classifier in the main UI.

Jev remains pinned to **jev-1.13.0**, with synthetic-only live requests. Rust handles
validation, statistical methods and rendering. Loading never silently joins
patient records or transfers results between taxonomies.

Run `josh tui PATH` or use Open for supported files and bundles. F1 opens searchable
help even inside forms. q quits safely and waits for in-flight work. Session
history is temporary; keep saved run folders. See the [User Guide](docs/USER_GUIDE.md)
and [synthetic sample pack](fixtures/synthetic-jev/README.md).

## CLI workflows

After building, use `./target/debug/josh` from the checkout, or `josh` if installed
on PATH. The launcher also accepts CLI arguments:

```bash
./scripts/start doctor --format text
./scripts/start molecular check fixtures/molecular/synthetic-features.json --format text
./scripts/start molecular prepare fixtures/molecular/synthetic-features.json
```

These commands run locally. Readiness reports key presence and input eligibility;
it does not authenticate credentials. `prepare` shows the request without sending it.

For a live synthetic analysis, after configuring the key:

```bash
mkdir -p results
./scripts/start molecular run fixtures/molecular/synthetic-features.json --out-dir results/run-001
./scripts/start tui results/run-001
./scripts/start molecular export results/run-001 --output results/run-001-report.md
```

Choose new destinations. Reopening and exporting are offline. Keep the run folder:
an inference JSON file needs its `features.json` sidecar to reopen. Requests use
`jev-1.13.0` at `https://api.typesafe.ai/v1/systemone`; ambiguous failures are never
retried automatically. See the [molecular guide](docs/data/MOLECULAR_GUIDE.md) for
formats, budgeted explanations, resumption, cohort tooling and evaluation.

Expression datasets use `josh dataset`; references use `josh reference`.
Summarized cases use `josh validate`, `prepare`, `demo`, `classify` and `guidance`.
Case CSV/TSV/JSONL must first pass through `josh import`; load its output bundle in
the TUI. Each command has `--help`. The [terminal guide](docs/TERMINAL_GUIDE.md)
documents output formats, stdin support and exit codes.

## Read and interpret results

Molecular Analysis uses an `InferenceRun`; Clinical uses a separate `ResultRecord`.
Their JSON fields and taxonomies differ:

| Workflow | Scores and provenance |
| --- | --- |
| Molecular | `response.answers.primary_site.probabilities`; `feature_sha256`, `request_sha256`, frozen `taxonomy`, full request/response |
| Clinical | `rankings[].raw_probability`; `rankings[].calibrated_probability` is `null`; `case_revision_sha256`, `request_sha256` |
| Both | `source` distinguishes Jev, mock and replay; `status` is `abstained` or `review_required`; calibration remains unvalidated |

Provider confidence is distinct from a class score. Neither is a calibrated cancer
probability. Provenance hashes detect inconsistency; they do not authenticate a
provider response or establish scientific validity.

The default molecular taxonomy has 22 detailed classes plus `insufficient_evidence`
and `other_origin`; expression comparisons carry their reference taxonomy.
Clinical's development taxonomy has 12 broad groups plus the two unresolved
outcomes. These are research contracts, not validated cancer classifiers.

## Documentation

- [Quickstart](docs/QUICKSTART.md) — prerequisites and Load → Analyze → Results → Markdown
- [User guide](docs/USER_GUIDE.md) — shared navigation, inputs, keys, reports and troubleshooting
- [Terminal and CLI reference](docs/TERMINAL_GUIDE.md) — commands, clinical controls and output conventions
- [Molecular formats and explanations](docs/data/MOLECULAR_GUIDE.md)
- [OncoNPC research guide and priorities](docs/references/ONCONPC_GUIDE.md) and
  [objective-by-objective assessment](docs/assessment/ONCONPC_PARITY.md)
- [Expression import](docs/data/EXPRESSION_GUIDE.md) and [reference comparison](docs/data/REFERENCE_GUIDE.md)
- [Clinical review](docs/CLINICAL_REVIEW_GUIDE.md) and [case batch import](docs/data/IMPORT_GUIDE.md)
- [Build status](docs/BUILD_STATUS.md) and [0.7.0 molecular validation report](docs/reports/0.7.0-molecular-validation.md)
- [Architecture](docs/ARCHITECTURE.md), [generated contracts](contracts/README.md) and [compatibility rules](docs/engineering/foundation.md)
- [Intended use](docs/product/intended-use.md), [sources](docs/SOURCES.md) and [roadmap](docs/PLAN.md)

The in-app guide is compiled into the binary and searches the user, molecular,
expression, reference, cohort and terminal guides, research assessment and source
register offline. Press F1, then / and `OncoNPC`. Rebuild after changing those files.
Earlier [gap assessments](docs/assessment/DATA_FIRST_GAP_ASSESSMENT.md) and phase
plans describe their dated baselines; use the build status for delivered capabilities.

## Local API and development checks

`josh serve --port 3000` exposes loopback-only `/health`, `/v1/demo`, `/v1/prepare`
and `/v1/guidance`. The POST routes accept summarized cases, with a 16 KiB request
limit. There is no live classification or molecular-analysis HTTP route, persistence
or authentication. See `josh schema openapi` for this local developer API.

```bash
cargo build --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Cargo also supports `--offline` once dependencies are cached. Tests verify software
contracts and workflows, not diagnostic performance. Remaining scientific work
includes curated cancer references, real-data provider eligibility, independent
cohort evaluation, representative repeatability studies, calibration and validated
out-of-distribution detection. A limited live synthetic check was recorded for
0.7.0; it is not a cancer-validation study.

The upstream repository was inspected at commit
`ee8069cdaaf997721cb071dffe1cb243651a5f56`; its source was not copied into this
implementation. See the [source audit](docs/assessment/OPEN_NEXUS.md). Its GPL-2.0
license and the provenance of future reused code or datasets must remain tracked;
no distribution license has been selected for this scaffold.
