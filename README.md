# Jev Onco Statistical Hierarchy (JOSH)

A Rust research prototype for evaluating **Jev as a cancer-of-unknown-primary (CUP) origin classifier**. Imported observations become structured case evidence; Jev ranks a versioned set of origins; Rust validates the answer and decides whether to abstain or send it for review.

Jev is the sole classifier in the rebuild. The development scope covers data preparation, Jev integration, evaluation/calibration and evidence review; the original classifier, its model artifacts and its explanation tooling are excluded.

**Current status (0.4.0):** adds themed Ratatui visualizations, schema 3 clinical context, a local source-linked subset of NICE CG104 review rules, and versioned review-note exports to the existing import/classifier workbench. Clinical signoff, raw GENIE mapping and live provider verification remain pending. No cancer-specific accuracy or calibration has been established. The demo is a uniform mock distribution, not a prediction. Live inference currently accepts synthetic cases only.

## Read first

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

The TUI opens a bundled synthetic clinical example. Press `6` for Visuals (left/right cycles score bars, IHC matrix, pathway, timeline and evidence counts), `7` for NICE Guidance, and `8` for clinical context/review history. Evidence, Request, Results, Help and Import remain on `1`–`5`. Press `d` for a labeled offline mock. Clinical stages are clinician-recorded, never inferred from model scores.

On Guidance, `a` records a reasoned local review; **8 then s** saves the complete case to a new file. `o` opens case JSON, `c` requests a confirmed live synthetic call, and `q` quits (asking before discarding unsaved reviews). Full controls and clinical limitations are in the [clinical review guide](docs/CLINICAL_REVIEW_GUIDE.md).

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

Next is [Phase 03](docs/phases/03-jev-classifier.md): provider transport tests, Jev reliability, request budgets and a live synthetic smoke test. [Phase 02](docs/phases/02-data-import-and-evidence.md) still needs raw GENIE mapping and an authorized reviewed cohort. [Phase 00](docs/phases/00-scope-and-feasibility.md) clinical/data decisions remain open. A small, labeled, correctly masked cohort is still needed to measure whether Jev can infer origin from this project's evidence.

The upstream repository was inspected at commit `ee8069cdaaf997721cb071dffe1cb243651a5f56`. Its source was not copied into this implementation. Its GPL-2.0 license and the provenance of any future reused data-processing code or datasets must remain tracked; no distribution license has been selected for this new scaffold yet.
