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

In **Analysis (F2)**:

1. **Load (`l`)** a molecular feature file, supported table or saved run. Tables
   open import settings; review the sample, assay and data declaration.
2. **Analyze (`a`)** opens a new run-directory dialog. Enter confirms one
   potentially billed synthetic Jev request. Set `TYPESAFE_API_KEY` in the launching
   shell first; `.env` files are not loaded automatically.
3. **Results (`v`)** shows raw rankings, abstention reasons and evidence checks.
   Successful inference saves its archive and `report.md` automatically.
4. **Export (`s`)** saves another Markdown report to a new `.md` file.
   An explanation is optional; `e` shows a budget before additional provider calls.

To try the complete review/export flow offline, press **d** in Analysis. It replaces
the current analysis with the bundled analytical demo. For importable molecular
tables and expression data, use the [basic data demo](fixtures/basic-demo/README.md):

```bash
cargo run --locked -p josh-app --example prepare_demo -- --out-dir results/basic-demo
```

## One terminal workspace

| Section | Purpose |
| --- | --- |
| **F2 Analysis** | Molecular inputs, Jev inference, saved results, explanations and Markdown reports |
| **F3 Data** | Expression import, gene mapping, QC, exploration and local reference comparison |
| **F4 Clinical** | Summarized cases, clinical context, source-linked NICE review and case exports |
| **F5 Cohort** | Paper-inspired heatmap, survival, treatment associations, calibration and study exports |

For the OncoNPC-inspired research dashboard, press **F5**, then **d**. The offline
demo is explicitly synthetic. **1–6** changes views, **i** shows survival confidence
bounds, **w** toggles supplied-propensity weighting, and **s** exports SVG, Markdown
or a reopenable JSON archive. Load frozen cohorts with **l**. See the
[Cohort Guide](docs/data/COHORT_GUIDE.md) for input contracts and statistical limits.
These capabilities do not establish Jev's cancer accuracy or paper parity.

Switching sections preserves their data, forms, results and jobs. Use the shared
**l/o** loader or `josh tui PATH` for supported files and bundles; **F1** opens help
even inside a form, and **g** opens it from navigation. **q** quits from navigation,
checks for unsaved clinical reviews and waits for in-flight work to finish.

The [user guide](docs/USER_GUIDE.md) documents input routing and section-specific
keys. For example, **a** sends a confirmed Jev analysis in Analysis, performs local
reference comparison in Data, and records a review on Clinical's Guidance page.
In Data, **r** loads reference JSON and **m** attaches a usable comparison to
Analysis. Clinical cases retain their own identity; switching sections does not
join them to molecular samples.

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
- [Expression import](docs/data/EXPRESSION_GUIDE.md) and [reference comparison](docs/data/REFERENCE_GUIDE.md)
- [Clinical review](docs/CLINICAL_REVIEW_GUIDE.md) and [case batch import](docs/data/IMPORT_GUIDE.md)
- [Build status](docs/BUILD_STATUS.md) and [0.7.0 molecular validation report](docs/reports/0.7.0-molecular-validation.md)
- [Architecture](docs/ARCHITECTURE.md), [generated contracts](contracts/README.md) and [compatibility rules](docs/engineering/foundation.md)
- [Intended use](docs/product/intended-use.md), [sources](docs/SOURCES.md) and [roadmap](docs/PLAN.md)

The in-app guide is compiled into the binary and searches the user, molecular,
expression, reference and terminal guides offline. Rebuild after changing those files.
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
