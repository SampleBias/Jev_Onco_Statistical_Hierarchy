# CLI and terminal workbench

The `nexus` binary supports scripts and an interactive Rust TUI. Both use the same case validation, Jev request preparation and result interpretation. The first build is a research scaffold with synthetic examples; scientific evaluation and bulk data imports remain in the phase plan.

## Start

```bash
cargo build --locked --bin nexus
./target/debug/nexus doctor --format text
./target/debug/nexus tui
```

The TUI opens a bundled synthetic case even when launched outside the checkout. To load your own canonical case:

```bash
./target/debug/nexus tui --case fixtures/synthetic-case.json
```

A terminal of at least 48 columns by 12 rows is required; 100 by 35 or larger is recommended. Piped/noninteractive execution reports a clear error. The terminal is restored on normal exit, Ctrl-C, recoverable errors and Rust panic; forcibly killing the process cannot run cleanup.

## Workbench controls

| Key | Action |
| --- | --- |
| `1`, `2`, `3`, `4` | Evidence, Request, Results, Help |
| `Tab`, `Shift-Tab` | Next/previous view |
| `↑`/`↓`, `k`/`j` | Scroll |
| `PgUp`, `PgDn`, `Home` | Page or return to top |
| `o` | Enter a case JSON path and load it |
| `r` | Reload the current file and clear the previous result |
| `d` | Run an offline mock; no API request |
| `c` | Review confirmation for one live synthetic Jev call |
| `s` | Save the Request or Results view as complete JSON |
| `?` | Help |
| `Esc` | Close a dialog |
| `q`, `Ctrl-C` | Quit |

The Evidence view shows the specimen and individual observations. Request shows the actual provider payload with local case identifiers omitted. Results preserves the complete distribution, model version, review flags and request fingerprint. Mock scores are explicitly labeled and never presented as a cancer prediction.

Use a normal editor to change case JSON, then reload. Path dialogs accept literal relative or absolute paths; they do not expand `~`, environment variables or shell commands. A failed load preserves the current case. While a live request is pending, navigation works and case changes/new runs are disabled. Quitting cancels local waiting; the provider may already have received the request and incurred a charge.

## CLI commands

| Command | Purpose |
| --- | --- |
| `nexus example` | Print a complete synthetic case template |
| `nexus doctor` | Report local readiness and key presence without exposing credentials or contacting Jev |
| `nexus taxonomy` | List all development taxonomy outcomes |
| `nexus schema KIND` | Print `case`, `jev-request`, `jev-response`, `result`, `error` JSON Schema or `openapi` |
| `nexus validate CASE` | Validate JSON and report case ID/finding count |
| `nexus prepare CASE` | Print the exact request without sending it |
| `nexus demo [CASE]` | Offline mock; defaults to bundled example |
| `nexus classify CASE` | Send one synthetic case to Jev |
| `nexus replay CASE RESPONSE` | Interpret an unverified local provider fixture |
| `nexus tui [--case CASE]` | Open the terminal workbench |
| `nexus serve [--port 3000]` | Run offline loopback HTTP endpoints |

`--help` works on each command; `--version` reports the package version. JSON is the default output for scripts. `--format text` displays a readable summary/table for validation, taxonomy, results and diagnostics. Templates and request previews remain JSON in either mode. Diagnostics go to stderr and do not contaminate stdout. Successful commands exit 0, application errors exit 1, and invalid CLI arguments exit 2.

In 0.2.0, JSON-mode failures use a stable envelope on stderr, for example `{"error":{"code":"invalid_json","message":"input does not match the JSON schema"}}`. Scripts should inspect `error.code`; readable wording may change. Invalid arguments use `invalid_arguments` and never echo submitted values. Help/version remain readable text. See [contracts and compatibility](engineering/foundation.md).

To inspect import requirements before writing an importer, run `./target/debug/nexus schema case`. To inspect the offline routes, run `./target/debug/nexus schema openapi`. Result exports now include both case revision and provider request fingerprints, and explicitly mark unverified replay scores.

## Files, pipes and exports

```bash
./target/debug/nexus example --output /tmp/nexus-example.json
./target/debug/nexus validate /tmp/nexus-example.json --format text
./target/debug/nexus example | ./target/debug/nexus prepare -
./target/debug/nexus demo --format text
./target/debug/nexus demo --output /tmp/nexus-demo-result.json
```

Use `-` as a case path to read stdin. Replay can read one of its inputs from stdin, not both. TUI reserves stdin for keyboard controls.

`--output` creates a new file and refuses to overwrite any existing file. TUI exports have the same behavior and always use JSON. Parent folders must already exist. On Unix, newly created exports have owner-only permissions. Exports may contain submitted evidence; store them in a suitable location. Avoid committing working case files or results; root `data/` and `results/` folders are ignored.

## Live Jev

Set `TYPESAFE_API_KEY` in the launching process environment using your normal secret-management method. A `.env` file is not automatically loaded. Never pass keys as CLI arguments. Then use `nexus classify CASE` or `c` in the TUI. TUI asks `y` before sending. The CLI classify command is itself the explicit send action.

Live inference accepts declared synthetic cases only in this first release. That declaration does not detect identifiers or verify provenance. Missing keys and unsuitable cases fail locally. The adapter is pinned to `jev-1.13.0`; no live provider result has yet been verified for this project.

## Development checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

CLI integration tests cover pipes, standalone use, exports, key masking and exit behavior. TUI tests render through Ratatui's test backend and exercise navigation, reload invalidation, export and live-request guards. These tests verify the application workflow, not diagnostic accuracy.
