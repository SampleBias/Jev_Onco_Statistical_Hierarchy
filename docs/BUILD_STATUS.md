# Build status — 2026-09-21

## Implemented and verified

Application 0.2.0 includes the Phase 01 foundation: separate domain/prompt/taxonomy/policy modules, generated JSON Schemas/OpenAPI, consistent CLI/API errors, case/request fingerprints, pinned builds and a dependency inventory. The three-crate Rust workspace retains the Jev transport and CLI/TUI/offline HTTP workflows. Phase 00 intended-use, taxonomy, inventory, feasibility and decision drafts are delivered; clinical/data approvals remain pending.

The current scope uses Jev as the sole classifier. Original classifier training/inference, model artifact recovery and explanation tooling are excluded from the delivery plan. None of those components was included in the Rust source or dependency manifests, so this scope update required documentation changes only.

| Check | Result |
| --- | --- |
| `cargo build --workspace --offline --locked` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --workspace --all-targets --offline --locked -- -D warnings` | Passed |
| `cargo test --workspace --offline --locked` | Passed: 45 tests |
| CLI `validate`, `prepare`, `demo` on synthetic case | Passed |
| CLI `classify` without credentials | Expected local error; no request sent |
| CLI integration tests | Pipes, standalone demo, JSON/text, file protection, credential masking, help/version, structured errors, schemas, trailing JSON and exact byte budgets passed |
| TUI tests | Navigation, reload invalidation, exports, narrow terminals, failed imports and live-request guards passed |
| Interactive PTY smoke test | 0.2.0 case launch, mock run, navigation, resize, quit and exact terminal-setting restoration passed |
| Generated schemas/OpenAPI | Drift check, reference resolution, schema validation and actual offline API response validation passed |
| Dependency inventory | 273 workspace/registry packages recorded; offline drift check passed |
| Markdown local-link check | No missing targets |
| Phase plan files | Eight separate Markdown files |

Environment: Linux, Rust/Cargo 1.98.1. The pinned toolchain components and schema-generation/test dependencies were downloaded for this milestone; checks run offline after fetching. `Cargo.lock` records resolved versions. Supported OS and compatibility rules are in the [foundation handoff](engineering/foundation.md). The initial [GitHub clean CI run](https://github.com/SampleBias/Jev_Onco_Nexus/actions/runs/35661300018) passed; the 0.2.0 run is recorded after pushing this milestone.

## Not yet verified or implemented

- `TYPESAFE_API_KEY` was absent; no live Jev prediction, provider latency or actual billed usage was measured.
- Tests validate software contracts and offline API behavior; the HTTP client still needs mock-server failure tests and a live synthetic smoke test in Phase 03.
- No real case records or GENIE cohort were imported. No model training, CUP evaluation or calibration was performed.
- No browser UI, database, user authentication, batch importer, queue, retries or production deployment exists yet.
- The origin taxonomy and gate thresholds are development examples, not clinically approved definitions or operating points.
- The runtime's synthetic-data declaration is not a deidentification detector.

The first live milestone is a synthetic contract smoke test. The first scientific milestone is a blinded, labeled cohort experiment with the evidence types the project will actually use. Both have owners and acceptance criteria in the [phase plan](PLAN.md).
