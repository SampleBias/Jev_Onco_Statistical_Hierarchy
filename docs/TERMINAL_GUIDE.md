# CLI and terminal workbench

The `josh` binary supports scripts and an interactive Rust TUI. Both use the same case validation, Jev request preparation and result interpretation. Version 0.4.0 adds visualizations and local NICE clinical review to the structured import workflow. Scientific evaluation and raw GENIE import remain in the phase plan.

## Data workbench — 0.5.0

`josh tui` now opens the sample-first expression workbench. Use `josh dataset --help`
for detection, import, inspection, gene exploration, exports and local reproduction.
The [expression guide](data/EXPRESSION_GUIDE.md) documents all new commands and keys.
No API key is needed for these local workflows.

The remainder of this page documents the compatible **legacy case interface**.
Launch it with `josh tui --legacy`, `--case FILE` or `--batch DIRECTORY`.

## Legacy start

```bash
cargo build --locked --bin josh
./target/debug/josh doctor --format text
./target/debug/josh tui --legacy
```

The TUI opens a bundled synthetic case even when launched outside the checkout. To load your own canonical case:

```bash
./target/debug/josh tui --case fixtures/synthetic-case.json
```

A terminal of at least 48 columns by 12 rows is required; 100 by 35 or larger is recommended. Piped/noninteractive execution reports a clear error. The terminal is restored on normal exit, Ctrl-C, recoverable errors and Rust panic; forcibly killing the process cannot run cleanup.

## Workbench controls

| Key | Action |
| --- | --- |
| `1`, `2`, `3`, `4` | Evidence, Request, Results, Help |
| `5` | Import quality report |
| `6`, `v` | Visuals: score bars, IHC matrix, pathway, timeline, evidence counts |
| `7` | Source-linked NICE guidance; up/down selects a rule |
| `8` | Clinical context and review history |
| `←`/`→` | On Visuals: change chart; on Guidance: scroll selected rule details |
| `a` | On Guidance: record a reasoned review; **8 then s** saves the complete case |
| `Tab`, `Shift-Tab` | Next/previous view |
| `↑`/`↓`, `k`/`j` | Scroll |
| `PgUp`, `PgDn`, `Home` | Page or return to top |
| `o` | Enter a case JSON path and load it |
| `b` | Open an import bundle directory and verify its report/sidecars |
| `[`, `]` | Previous/next imported case; verify its fingerprint and clear previous results |
| `r` | Reload the current file and clear the previous result |
| `d` | Run an offline mock; no API request |
| `c` | Review confirmation for one live synthetic Jev call |
| `s` | Export Request, Results, Import, Visuals or Guidance; on Review, save the complete case |
| `?` | Help |
| `Esc` | Close a dialog |
| `q`, `Ctrl-C` | Quit; asks before discarding unsaved reviews |

The Evidence view shows the specimen and individual observations. Request shows the actual provider payload with local case identifiers omitted. Results preserves the complete distribution, model version, review flags and request fingerprint. Mock scores are explicitly labeled and never presented as a cancer prediction.

The legacy TUI example uses schema 3 and invented clinical context. The coordinated
navy/teal/cyan/blue/violet views are best at 120×40 or larger. Unknown/not-tested
results remain distinct. No model score assigns a clinical stage.
See the [clinical review guide](CLINICAL_REVIEW_GUIDE.md) for schema migration,
rule coverage, review persistence and clinical limitations. On Guidance,
PgUp/PgDn scroll the selected rule's details instead of changing selection.

Use a normal editor to change case JSON, then reload. Path dialogs accept literal relative or absolute paths; they do not expand `~`, environment variables or shell commands. A failed load preserves the current case. While a live request is pending, navigation works and case changes/new runs are disabled. Quitting cancels local waiting; the provider may already have received the request and incurred a charge.

Imported bundle cases are fingerprint-checked. Make a standalone copy before editing one; modifying a bundle case in place causes its next load to fail. Open an import with `josh tui --batch DIRECTORY` or press `b` in the workbench. The Import view supports `s` to export the complete report.

## CLI commands

| Command | Purpose |
| --- | --- |
| `josh example` | Print a complete synthetic case template |
| `josh example --clinical` | Print the schema 3 synthetic clinical template |
| `josh guidance CASE` | Evaluate local NICE review rules without a provider call |
| `josh doctor` | Report local readiness and key presence without exposing credentials or contacting Jev |
| `josh taxonomy` | List all development taxonomy outcomes |
| `josh schema KIND` | Print `case`, `jev-request`, `jev-response`, `result`, `error` JSON Schema or `openapi` |
| `josh validate CASE` | Validate JSON and report case ID/finding count |
| `josh import INPUT --input-format FORMAT --source-id ID --out-dir NEW_DIR` | Import JSON/JSONL/CSV/TSV locally with quality reports; migrate schema 1 |
| `josh batch DIRECTORY` | Inspect an import bundle and verify report/sidecar integrity |
| `josh prepare CASE` | Print the exact request without sending it |
| `josh demo [CASE]` | Offline mock; defaults to bundled example |
| `josh classify CASE` | Send one synthetic case to Jev |
| `josh replay CASE RESPONSE` | Interpret an unverified local provider fixture |
| `josh tui [--case CASE] [--batch DIRECTORY]` | Open a standalone case or an import bundle; flags are mutually exclusive |
| `josh serve [--port 3000]` | Run offline loopback HTTP endpoints |

`--help` works on each command; `--version` reports the package version. JSON is the default output for scripts. `--format text` displays a readable summary/table for validation, taxonomy, results and diagnostics. Templates and request previews remain JSON in either mode. Diagnostics go to stderr and do not contaminate stdout. Successful commands exit 0, application errors exit 1, and invalid CLI arguments exit 2.

Import additionally exits 3 when records are rejected, while preserving the report and any accepted cases. `--output` is not accepted for import; its report is already written inside the new bundle. The [import guide](data/IMPORT_GUIDE.md) covers columns, status normalization, patient groups, labels and limits. `josh schema` also supports `import-report`, `labels` and `splits`.

`josh schema guidance` exports the guidance-report contract. `guidance CASE`
supports JSON/text and protected `--output`; `POST /v1/guidance` is its offline API
equivalent. Guidance and case exports contain local clinical context, unlike provider
request previews.

In 0.2.0, JSON-mode failures use a stable envelope on stderr, for example `{"error":{"code":"invalid_json","message":"input does not match the JSON schema"}}`. Scripts should inspect `error.code`; readable wording may change. Invalid arguments use `invalid_arguments` and never echo submitted values. Help/version remain readable text. See [contracts and compatibility](engineering/foundation.md).

To inspect import requirements before writing an importer, run `./target/debug/josh schema case`. To inspect the offline routes, run `./target/debug/josh schema openapi`. Result exports now include both case revision and provider request fingerprints, and explicitly mark unverified replay scores.

## Files, pipes and exports

```bash
./target/debug/josh example --output /tmp/josh-example.json
./target/debug/josh validate /tmp/josh-example.json --format text
./target/debug/josh example | ./target/debug/josh prepare -
./target/debug/josh demo --format text
./target/debug/josh demo --output /tmp/josh-demo-result.json
```

Use `-` as a case path to read stdin. Replay can read one of its inputs from stdin, not both. TUI reserves stdin for keyboard controls.

`--output` creates a new file and refuses to overwrite any existing file. TUI exports have the same behavior and always use JSON. Parent folders must already exist. On Unix, newly created exports have owner-only permissions. Exports may contain submitted evidence; store them in a suitable location. Avoid committing working case files or results; root `data/` and `results/` folders are ignored.

## Live Jev

Set `TYPESAFE_API_KEY` in the launching process environment using your normal secret-management method. A `.env` file is not automatically loaded. Never pass keys as CLI arguments. Then use `josh classify CASE` or `c` in the legacy TUI. TUI asks `y` before sending. The CLI classify command is itself the explicit send action.

Live inference accepts declared synthetic cases only in this first release. That declaration does not detect identifiers or verify provenance. Missing keys and unsuitable cases fail locally. The adapter is pinned to `jev-1.13.0`; no live provider result has yet been verified for this project.

## Development checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

CLI integration tests cover pipes, standalone use, exports, key masking and exit behavior. TUI tests render through Ratatui's test backend and exercise navigation, reload invalidation, export and live-request guards. These tests verify the application workflow, not diagnostic accuracy.
