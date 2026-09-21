# Jev Onco Nexus

A Rust research prototype for evaluating **Jev as a cancer-of-unknown-primary (CUP) origin classifier**. Imported observations become structured case evidence; Jev ranks a versioned set of origins; Rust validates the answer and decides whether to abstain or send it for review.

Jev is the sole classifier in the rebuild. The development scope covers data preparation, Jev integration, evaluation/calibration and evidence review; the original classifier, its model artifacts and its explanation tooling are excluded.

**Current status:** working CLI and interactive TUI, offline HTTP API, synthetic fixture, Jev REST adapter, and developer roadmap. No cancer-specific accuracy or calibration has been established. The demo is a uniform mock distribution, not a prediction. Live inference currently accepts synthetic cases only.

## Read first

- [Developer roadmap and phase index](docs/PLAN.md)
- [CLI and TUI user guide](docs/TERMINAL_GUIDE.md)
- [Open_Nexus source audit and existing ML inventory](docs/assessment/OPEN_NEXUS.md)
- [Jev capabilities and suitability assessment](docs/assessment/JEV.md)
- [Architecture and data contracts](docs/ARCHITECTURE.md)
- [Build status and verified limitations](docs/BUILD_STATUS.md)
- [Evidence and official sources](docs/SOURCES.md)

## Run

Use a current stable Rust toolchain with Cargo, rustfmt, clippy, and a system C compiler. Development was checked with Rust 1.98.1. Cargo needs registry access on a fresh machine; after dependencies are cached, the following also work with `--offline`.

```bash
cargo build --workspace --locked
cargo run --locked --bin nexus -- tui
cargo run --locked --bin nexus -- doctor --format text
cargo run --locked --bin nexus -- validate fixtures/synthetic-case.json
cargo run --locked --bin nexus -- prepare fixtures/synthetic-case.json
cargo run --locked --bin nexus -- demo fixtures/synthetic-case.json
```

`prepare` prints the exact request without sending it. `demo` always abstains and returns `source: mock`. The example is invented and has no ground-truth origin. Do not interpret its distribution as medical evidence.

The TUI opens a bundled example and provides Evidence, Request, Results and Help views. Press `o` to open case JSON, `d` for a demo, `c` for a live synthetic call, `s` to export JSON, and `q` to quit. `Tab` changes views; arrows or `j`/`k` scroll. Live calls require confirmation inside the TUI.

The CLI supports `--format text`, `--output NEW_FILE`, stdin via `-`, `--help`, and `--version`. `nexus example` prints a case template; `nexus demo` works with no input file. Exports never overwrite an existing file. See the [terminal guide](docs/TERMINAL_GUIDE.md) for all commands and keys.

For a live request, provide a TypeSafe key through the process environment or your secret manager, then run:

```bash
cargo run --locked --bin nexus -- classify fixtures/synthetic-case.json
```

This requires `TYPESAFE_API_KEY` and sends the synthetic findings to `https://api.typesafe.ai/v1/systemone`. The pinned model is `jev-1.13.0`. No live provider call has been verified in this workspace. A `.env` file is **not** automatically loaded. Do not put keys in JSON, source code, browser code, or command arguments.

## Local API

```bash
cargo run --locked --bin nexus -- serve --port 3000
curl http://127.0.0.1:3000/health
curl --fail-with-body http://127.0.0.1:3000/v1/demo \
  -H 'Content-Type: application/json' \
  --data-binary @fixtures/synthetic-case.json
```

`POST /v1/prepare` accepts the same case and returns a request preview. The server binds to loopback, has a 16 KiB request limit, and exposes no live classification route. It has no persistence or authentication yet; it is a local developer service.

## Result semantics

| Field | Meaning |
| --- | --- |
| `rankings[].raw_probability` | Jev's distribution over the supplied options, or an explicitly marked simulation |
| `rankings[].calibrated_probability` | Always `null` until a separately validated calibration artifact exists |
| `provider_confidence` | Provider's distribution-concentration statistic, distinct from class probability |
| `status` | `abstained` or `review_required`; never an autonomous diagnosis |
| `calibration_status` | Currently `not_validated_for_cup` |
| `source` | `jev`, `mock`, or `replay` |

The prototype taxonomy is illustrative: 12 broad origin groups plus `other_origin` and `insufficient_evidence`. It mixes broad sites and one lineage category and needs specialist review before cohort evaluation. It is not OncoNPC's validated label set.

## Quality checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Tests cover input limits, label leakage through undeclared fields, provider contracts, probability bounds, model pinning, abstention, mock labeling, request fingerprints, and preflight restrictions. They establish software behavior, not diagnostic performance.

## Next implementation milestone

Complete [Phase 00](docs/phases/00-scope-and-feasibility.md), then extend the scaffold through [Phase 01](docs/phases/01-rust-foundation.md) and [Phase 02](docs/phases/02-data-import-and-evidence.md). The most consequential next step is a small, labeled, correctly masked case cohort to measure whether Jev can infer origin from the evidence available to this project.

The upstream repository was inspected at commit `ee8069cdaaf997721cb071dffe1cb243651a5f56`. Its source was not copied into this implementation. Its GPL-2.0 license and the provenance of any future reused data-processing code or datasets must remain tracked; no distribution license has been selected for this new scaffold yet.
