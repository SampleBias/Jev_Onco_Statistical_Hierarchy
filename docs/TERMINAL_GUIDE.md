# CLI and terminal reference

JOSH has one entry point: `josh tui [INPUT]`. Run `./scripts/start` to build
and launch. The [User Guide](USER_GUIDE.md) documents the sample-centered workflow.

## Shared terminal controls

| Key / control | Action outside editors |
| --- | --- |
| Workbench ▾ / F2 / F5 | Select single-sample workbench / cohort studies |
| Tab / Shift-Tab; Enter / Space | Move focus; activate the focused control |
| o / l / Open | Browse supported input or saved run |
| Ctrl+B | Choose a bundled synthetic input |
| 1 / 2 | Data / Results in the sample workspace |
| a / c | Contextual next action shown on the toolbar |
| v / p | Results summary / paired ring + scatter |
| m / Menu | Export, previous samples, advanced actions, help and quit |
| s | Export the active result, dataset or complete legacy case |
| e | Optional explanation confirmation, when a saved Jev run is eligible |
| d | Explicit offline demonstration; preserves the previous sample |
| x | Cancel future work / safely finish in-flight work |
| F1 / Ctrl+G | Searchable help, including inside forms |
| Esc | Close the top dialog without submitting |
| q / Ctrl+C | Quit safely; wait for in-flight work |

F3 now selects Data; F4 explains legacy-file loading rather than opening another
unrelated patient. Existing CLI commands remain compatible.

The sample workbench has Data and Results, not separate Analysis/Expression/Clinical
applications. Results' View selector retains Summary, paired ring/scatter, ring,
scatter and waterfall. With chart content focused, arrows cycle/select and t
retargets cached explanations. Raw scores remain uncalibrated.

Expression opens as a sample input. Data's selector exposes Overview, Quality,
Comparison, Genes and Reference. Menu offers sample selection, reference loading,
local comparison, confirmed expression-only preparation and gene search. Changing
evidence moves the old analysis to Earlier runs instead of displaying it as current.

In Studies, the selector retains all six views; 1–6 also switches views. Menu
contains normalization, confidence intervals, supplied-propensity weighting and
curve selection. Studies are offline and secondary to the sample workbench.

Legacy case files/bundles remain readable and exportable, including guidance,
review history and visualizations. New clinical classification and guideline
review are no longer exposed in the main TUI. Historical CLI/API support remains.

Browse accepts quoted/spaced paths, ~/ and session-local Recent. No shell expansion
is performed. Relative paths use the displayed directory. The TUI requires an
interactive terminal; 80×24 is supported, with additional detail at larger sizes.
Normal exit, Ctrl+C, recoverable errors and Rust panic restore the terminal.

## Molecular CLI commands

| Command | Purpose |
| --- | --- |
| josh molecular example / taxonomy | Print synthetic feature input / default experimental taxonomy |
| josh molecular import INPUT --input-format FORMAT --sample ID --patient-group ID --source-id ID --assay NAME | Import one molecular sample; --synthetic only for invented data |
| josh molecular check FEATURES | Report local readiness and blockers; no provider call |
| josh molecular prepare FEATURES | Validate and preview the exact request offline |
| josh molecular run FEATURES --out-dir NEW_DIR | One synthetic Jev request; archive and Markdown report |
| josh molecular demo --out-dir NEW_DIR | Offline analytical inference/explanation fixture |
| josh molecular plan RUN_DIR | Estimate explanation budget without sending |
| josh molecular explain RUN_DIR | Run/resume a budgeted explanation; may send billed requests |
| josh molecular inspect ARCHIVE_JSON | Verify and summarize an explanation or checkpoint |
| josh molecular export RUN_OR_ARCHIVE --output NEW.md | Verified Markdown report, also supports inference with features.json sidecar |

Use `josh molecular --help` for merge, expression/signature attachment, experimental
SBS processing, cohort, evaluation, repeatability and stability commands.
`josh dataset --help` lists expression operations; `josh reference --help` lists
reference build/inspect/compare/prepare. Their guides include worked examples.

## Case and utility CLI commands

| Command | Purpose |
| --- | --- |
| josh tui [INPUT] | Open the one workspace and optionally load a supported input |
| josh doctor | Local key/model status; no authentication or provider request |
| josh example [--clinical] | Synthetic case template; --clinical includes schema 3 context |
| josh taxonomy | Clinical development outcomes; molecular taxonomy is a separate command |
| josh schema KIND | Print a generated contract; --help lists kinds, including openapi |
| josh validate CASE | Validate a canonical case |
| josh import INPUT --input-format FORMAT --source-id ID --out-dir NEW_DIR | Local case JSON/JSONL/CSV/TSV import and quality reports |
| josh batch DIRECTORY | Verify a case import bundle |
| josh prepare CASE | Exact case request preview; no send |
| josh demo [CASE] | Offline uniform mock; defaults to the bundled case |
| josh classify CASE | One synthetic case request to Jev |
| josh replay CASE RESPONSE | Interpret an unverified local provider response fixture |
| josh guidance CASE | Evaluate local clinical review rules offline |
| josh serve [--port 3000] | Offline loopback HTTP endpoints for summarized cases |

## Output formats, files and stdin

--help works on every command; --version reports the package version. Most commands
produce JSON by default; --format text uses a readable summary where available.
Templates, schemas and request previews remain JSON. Explicit exports use their
own selected format: molecular export defaults to Markdown, with --kind
markdown/json/csv/svg. CLI --kind svg/csv/json expects a completed explanation
JSON file. Filename extensions do not select CLI output formats.

```bash
./target/debug/josh molecular demo --out-dir /tmp/josh-guide-demo --format text
./target/debug/josh molecular export /tmp/josh-guide-demo --output /tmp/josh-guide-report.md
./target/debug/josh molecular export /tmp/josh-guide-demo/explanation.json \
  --kind svg --output /tmp/josh-guide-chart.svg
./target/debug/josh example | ./target/debug/josh prepare -
```

Choose new destinations on each run. --output creates a new file and refuses to
overwrite. TUI exports do likewise. Parent folders must exist; exports use owner-only
permissions on Unix. Case import, dataset import/migrate-case, tui and serve reject
--output; imports use --out-dir instead. Keep original run/dataset bundles, not just
rendered reports. Working data and results belong in the ignored data/ or results/
directories rather than source control.

Case-path commands accept - for stdin. Replay allows one stdin input, not both.
Case import and dataset detect/import also accept stdin; gene dictionaries require
files. Molecular commands use file paths rather than stdin. The TUI reserves stdin
for keyboard input.

Application JSON errors appear on stderr as an error envelope with code and message,
for example {"error":{"code":"invalid_json","message":"input does not match the JSON schema"}}.
Scripts should inspect error.code; wording may change. Help/version remain text.

| Exit | Meaning |
| --- | --- |
| 0 | Command completed; inspect result status/QC/readiness for its domain outcome |
| 1 | Application error |
| 2 | Invalid CLI arguments |
| 3 | Case import rejected records, expression import blocked QC, or reference comparison/preparation failed gates |

`molecular check` can exit 0 with ready:false. An abstained inference is a completed
result, not a CLI failure. Case imports with rejected rows retain accepted cases and
reports; blocked-QC expression imports retain data for inspection. See the
[Import Guide](data/IMPORT_GUIDE.md) and [compatibility rules](engineering/foundation.md).

## Live Jev and verification

Set TYPESAFE_API_KEY in the launching process environment; .env is not loaded and
keys must not be CLI arguments. Analysis sends after its run-directory confirmation;
The legacy TUI no longer sends clinical requests. CLI run/classify/explain are explicit
send actions. The pinned model is jev-1.13.0; real-data eligibility remains open.
Local readiness checks do not authenticate the key. Requests are not automatically
retried. See the [0.7.0 validation report](reports/0.7.0-molecular-validation.md) for
the limited synthetic live check and its observed variation.

Development checks are `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets --locked -- -D warnings`, and
`cargo test --workspace --locked`. These verify software behavior, not cancer accuracy.
