# Jev Onco Statistical Hierarchy (JOSH)

A Rust molecular-data workbench for cancer-of-unknown-primary research, using **Ratatui** and a CLI. JOSH is moving toward Sample → Data → Features → Reference comparison → Jev inference → Evidence and uncertainty.

**Current status (0.5.0):** the first data-first milestone adds independent samples, expression CSV/TSV import, HGNC gene mapping, explicit transformations, molecular import QC, source archives and a new default Ratatui workbench. See the [expression guide](docs/data/EXPRESSION_GUIDE.md). Reference-cancer comparisons and Jev inference on these expression features are still pending; Phase 1 is not complete.

Jev remains the sole origin classifier. The existing summarized-case Jev CLI and legacy TUI remain available with synthetic-only live requests. No cancer-specific accuracy or calibration has been established. XGBoost, original weights and SHAP remain excluded.

## Read first

- [Data-first redesign gap assessment](docs/assessment/DATA_FIRST_GAP_ASSESSMENT.md) — current code versus the design team's molecular workbench specification; proposed changes and three developer phases, retaining Rust, Jev and Ratatui
- [Developer roadmap and phase index](docs/PLAN.md)
- [CLI and TUI user guide](docs/TERMINAL_GUIDE.md)
- [Visualizations, NICE rule coverage and clinical review guide](docs/CLINICAL_REVIEW_GUIDE.md)
- [Batch import formats and migration guide](docs/data/IMPORT_GUIDE.md)
- [Open_Nexus source audit and existing ML inventory](docs/assessment/OPEN_NEXUS.md)
- [Jev capabilities and suitability assessment](docs/assessment/JEV.md)
- [Architecture and data contracts](docs/ARCHITECTURE.md)
- [Build status and verified limitations](docs/BUILD_STATUS.md)
- [Intended use and open review decisions](docs/product/intended-use.md)
- [Foundation handoff and compatibility rules](docs/engineering/foundation.md)
- [Generated JSON Schemas and OpenAPI](contracts/README.md)
- [Evidence and official sources](docs/SOURCES.md)

## Run

Use the pinned Rust 1.98.1 toolchain with Cargo, rustfmt, clippy, and a system C compiler on Linux. Cargo needs registry access on a fresh machine; after dependencies are cached, the following also work with `--offline`.

```bash
cargo build --workspace --locked
cargo run --locked --bin josh -- tui
cargo run --locked --bin josh -- doctor --format text
cargo run --locked --bin josh -- validate fixtures/synthetic-case.json
cargo run --locked --bin josh -- prepare fixtures/synthetic-case.json
cargo run --locked --bin josh -- demo fixtures/synthetic-case.json
```

`prepare` prints the exact request without sending it. `demo` always abstains and returns `source: mock`. The example is invented and has no ground-truth origin. Do not interpret its distribution as medical evidence.

The TUI opens two invented expression samples. Its primary views are Samples, Datasets, Analyze, Explore, Models, Reference and Projects. Press `i` to import, `p` to paste a table, `o` to open a dataset, `[`/`]` to change samples and `/` to search genes. No provider request occurs in these data workflows.

```bash
mkdir -p results
./target/debug/josh dataset import fixtures/expression/synthetic-expression.tsv \
  --dataset-id expression-demo --units tpm --transform log2-one-plus \
  --gene-map fixtures/expression/hgnc-subset.tsv --gene-map-release fixture-v1 \
  --platform invented-demonstration --synthetic --out-dir results/expression-demo
./target/debug/josh tui --dataset results/expression-demo
./target/debug/josh dataset verify results/expression-demo --reproduce
```

The original case/clinical interface is available with `josh tui --legacy`, `--case FILE` or `--batch DIRECTORY`. Its source-linked NICE guidance, clinical review records and score/IHC charts remain compatible. See the [clinical review guide](docs/CLINICAL_REVIEW_GUIDE.md).

The CLI supports `--format text`, `--output NEW_FILE`, stdin via `-`, `--help`, and `--version`. `josh example` prints a case template; `josh demo` works with no input file. Exports never overwrite an existing file. See the [terminal guide](docs/TERMINAL_GUIDE.md) for all commands and keys.

```bash
./target/debug/josh example --clinical --output /tmp/josh-clinical.json
./target/debug/josh guidance /tmp/josh-clinical.json --format text
```

Guidance runs offline without a key. It preserves unknowns, records contextual investigation
and MDT prompts, and reflects NICE's withdrawn 2023 gene-expression restrictions.
It is a documented subset, not a complete guideline or a clinically validated decision system.

To try batch evidence import and browse its three synthetic cases:

```bash
mkdir -p results
./target/debug/josh import fixtures/import/synthetic-findings.csv \
  --input-format csv --source-id synthetic-v1 --out-dir results/import-001 --format text
./target/debug/josh tui --batch results/import-001
```

Use `[`/`]` to browse cases and `5` for quality. Import runs locally, preserves labels in a separate file and reports rejected records. Choose a new output directory for each run; exit code 3 means some or all records were rejected. See the [import guide](docs/data/IMPORT_GUIDE.md) for all formats and limits.

For a live request, provide a TypeSafe key through the process environment or your secret manager, then run:

```bash
cargo run --locked --bin josh -- classify fixtures/synthetic-case.json
```

This requires `TYPESAFE_API_KEY` and sends the synthetic findings to `https://api.typesafe.ai/v1/systemone`. The pinned model is `jev-1.13.0`. No live provider call has been verified in this workspace. A `.env` file is **not** automatically loaded. Do not put keys in JSON, source code, browser code, or command arguments.

## Local API

```bash
cargo run --locked --bin josh -- serve --port 3000
curl http://127.0.0.1:3000/health
curl --fail-with-body http://127.0.0.1:3000/v1/demo \
  -H 'Content-Type: application/json' \
  --data-binary @fixtures/synthetic-case.json
```

`POST /v1/prepare` accepts the same case and returns a request preview; `POST /v1/guidance` returns the local clinical review report. The server binds to loopback, has a 16 KiB request limit, and exposes no live classification route. It has no persistence or authentication yet; it is a local developer service.

## Result semantics

| Field | Meaning |
| --- | --- |
| `rankings[].raw_probability` | Jev's distribution over the supplied options, or an explicitly marked simulation |
| `rankings[].calibrated_probability` | Always `null` until a separately validated calibration artifact exists |
| `provider_confidence` | Provider's distribution-concentration statistic, distinct from class probability |
| `status` | `abstained` or `review_required`; never an autonomous diagnosis |
| `calibration_status` | Currently `not_validated_for_cup` |
| `source` | `jev`, `mock`, or `replay` |
| `case_revision_sha256` / `request_sha256` | Versioned fingerprints of the full case and provider request; these do not authenticate replayed responses |

The prototype taxonomy is illustrative: 12 broad origin groups plus `other_origin` and `insufficient_evidence`. It mixes broad sites and one lineage category and needs specialist review before cohort evaluation. It is not OncoNPC's validated label set.

## Quality checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Tests cover input limits, label leakage through undeclared fields, provider contracts, probability bounds, model pinning, abstention, mock labeling, request fingerprints, and preflight restrictions. They establish software behavior, not diagnostic performance.

## Next implementation milestone

[Redesign Phase 1](docs/redesign/01-samples-expression-and-reference.md) is in progress. The sample/expression/mapping/QC milestone is implemented; next are a compatible known-primary reference, numerical comparisons, a structured molecular Jev evidence package and frozen evaluation. Provider transport hardening and a live synthetic contract check also remain required. The new data workbench does not yet return tissue-of-origin predictions.

[Phase 2](docs/redesign/02-multimodal-and-cohorts.md) adds variants, structured IHC and cohorts; [Phase 3](docs/redesign/03-modalities-and-connectors.md) adds secondary modalities and repository connectors. The [assessment](docs/assessment/DATA_FIRST_GAP_ASSESSMENT.md) records the 0.4.0 baseline and all requirement gaps.

The upstream repository was inspected at commit `ee8069cdaaf997721cb071dffe1cb243651a5f56`. Its source was not copied into this implementation. Its GPL-2.0 license and the provenance of any future reused data-processing code or datasets must remain tracked; no distribution license has been selected for this new scaffold yet.
