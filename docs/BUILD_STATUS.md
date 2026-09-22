# Build status — 2026-09-21

## Implemented and verified

Application 0.3.0 adds the Phase 02 structured import milestone to the Phase 01 foundation. The four-crate workspace now includes `nexus-ingest`: local JSON/JSONL/CSV/TSV imports, schema 2 migration, source references, observation status, censored ages, separate labels, patient-group splits, integrity-checked bundles and per-record quality reports. CLI commands and a TUI bundle browser expose the workflow. Phase 00 clinical/data approvals remain pending.

The current scope uses Jev as the sole classifier. Original classifier training/inference, model artifact recovery and explanation tooling are excluded from the delivery plan. None of those components was included in the Rust source or dependency manifests, so this scope update required documentation changes only.

| Check | Result |
| --- | --- |
| `cargo build --workspace --offline --locked` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --workspace --all-targets --offline --locked -- -D warnings` | Passed |
| `cargo test --workspace --offline --locked` | Passed: 71 tests |
| CLI `validate`, `prepare`, `demo` on synthetic case | Passed |
| CLI `classify` without credentials | Expected local error; no request sent |
| CLI integration tests | Pipes, standalone demo, JSON/text, file protection, credential masking, help/version, structured errors, schemas, trailing JSON and exact byte budgets passed |
| TUI tests | Navigation, reload invalidation, exports, narrow terminals, failed imports and live-request guards passed |
| Interactive PTY smoke test | Passed: bundle launch, censored age, next/previous cases, import report, mock result, resize, quit and terminal restoration |
| Generated schemas/OpenAPI | Drift check, reference resolution, schema validation and actual offline API response validation passed |
| Dependency inventory | 276 workspace/registry packages recorded; offline drift check passed |
| Batch imports | Source hashes, migration, row/byte caps, duplicate/conflict rejection, label separation, patient holdout and protected export tested |
| TUI batch workflow | Case navigation, source/status/censored-age display, quality report export, changed-case detection and result invalidation tested |
| Markdown local-link check | No missing targets |
| Phase plan files | Eight separate Markdown files |

Environment: Linux, Rust/Cargo 1.98.1. CSV parser dependencies were fetched for this milestone; checks run offline after fetching. `Cargo.lock` records resolved versions. Supported OS and compatibility rules are in the [foundation handoff](engineering/foundation.md). The [Phase 01 clean GitHub run](https://github.com/SampleBias/Jev_Onco_Nexus/actions/runs/35667838559) passed. Current commit-level status is available in the [private repository's Rust checks](https://github.com/SampleBias/Jev_Onco_Nexus/actions/workflows/ci.yml).

## Not yet verified or implemented

- The user configured a key in their terminal; this build session does not inherit that environment. No live Jev prediction, provider latency or actual billed usage was measured here.
- Tests validate software contracts and offline API behavior; the HTTP client still needs mock-server failure tests and a live synthetic smoke test in Phase 03.
- No real case records or GENIE cohort were imported. No model training, CUP evaluation or calibration was performed.
- No raw GENIE release mapper, genomic coordinate/allele/CNA/coverage normalization, browser UI, database, user authentication, queue, retries or production deployment exists yet.
- Patient-group partitions are deterministic engineering artifacts. Real cohort linkage, ontology mapping, indirect leakage review, scientific split approval and cross-file reconciliation remain pending.
- The origin taxonomy and gate thresholds are development examples, not clinically approved definitions or operating points.
- The runtime's synthetic-data declaration is not a deidentification detector.

The first live milestone is a synthetic contract smoke test. The first scientific milestone is a blinded, labeled cohort experiment with the evidence types the project will actually use. Both have owners and acceptance criteria in the [phase plan](PLAN.md).
