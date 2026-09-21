# Build status — 2026-09-21

## Implemented and verified

Three-crate Rust workspace: domain/contracts, Jev transport, and CLI/TUI/offline HTTP application. Includes a synthetic case, a pinned Jev request builder, strict provider-answer validation, raw probability ranking, research review/abstention, and reproducibility metadata. Terminal workflows include open/reload, request preview, demo/live actions, complete result tables, JSON exports and contextual help.

The current scope uses Jev as the sole classifier. Original classifier training/inference, model artifact recovery and explanation tooling are excluded from the delivery plan. None of those components was included in the Rust source or dependency manifests, so this scope update required documentation changes only.

| Check | Result |
| --- | --- |
| `cargo build --workspace --offline --locked` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --workspace --all-targets --offline --locked -- -D warnings` | Passed |
| `cargo test --workspace --offline --locked` | Passed: 32 tests |
| CLI `validate`, `prepare`, `demo` on synthetic case | Passed |
| CLI `classify` without credentials | Expected local error; no request sent |
| CLI integration tests | Pipes, standalone demo, JSON/text, file protection, credential masking, help/version and errors passed |
| TUI tests | Navigation, reload invalidation, exports, narrow terminals and live-request guards passed |
| Interactive PTY smoke test | Launch, tabs, mock results, help, resize, quit and terminal restoration passed |
| Markdown local-link check | No missing targets |
| Phase plan files | Eight separate Markdown files |

Environment: Linux, Rust/Cargo 1.98.1. Registry dependencies were already cached; the local build did not require a download. `Cargo.lock` records the resolved versions.

## Not yet verified or implemented

- `TYPESAFE_API_KEY` was absent; no live Jev prediction, provider latency or actual billed usage was measured.
- Tests validate software contracts and offline API behavior; the HTTP client still needs mock-server failure tests and a live synthetic smoke test in Phase 03.
- No real case records or GENIE cohort were imported. No model training, CUP evaluation or calibration was performed.
- No browser UI, database, user authentication, batch importer, queue, retries or production deployment exists yet.
- The origin taxonomy and gate thresholds are development examples, not clinically approved definitions or operating points.
- The runtime's synthetic-data declaration is not a deidentification detector.

The first live milestone is a synthetic contract smoke test. The first scientific milestone is a blinded, labeled cohort experiment with the evidence types the project will actually use. Both have owners and acceptance criteria in the [phase plan](PLAN.md).
