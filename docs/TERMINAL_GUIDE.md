# CLI and terminal reference

JOSH 0.8.0 has one terminal entry point: `josh tui [INPUT]`. Run `./scripts/start`
from the repository to build and launch it. For the main Load → Analyze → Results
→ Markdown workflow, see [Quickstart](QUICKSTART.md) and [User Guide](USER_GUIDE.md).

## Shared terminal controls

| Key | Action outside text editors |
| --- | --- |
| F2 / F3 / F4 / F5 | Analysis / Data / Clinical / Cohort; preserve section state and jobs |
| l / o | Shared Load dialog for supported files/bundles |
| g | Searchable guide; F1/Ctrl+g also works inside editors |
| Esc | Close a dialog; it does not stop a job or leave a section |
| q / Ctrl+C | Quit the workspace; check recorded unsaved reviews and finish in-flight work |

Help captures its own keys; q/Esc closes help before returning to the application.
Close the shared Load dialog or quit confirmation before switching sections.
Paths are literal; there is no shell or environment expansion. In the shared
loader, type paths containing spaces without surrounding shell quotes. CLI shell
arguments do need normal quoting. The launcher uses the repository as its working
directory; a directly launched binary uses the current working directory.

The TUI needs interactive stdin/stdout. A large terminal, such as 120×40, gives
room for all panels; small layouts omit detail or ask for more space. Normal exit,
Ctrl+C, recoverable errors and Rust panic restore the terminal. A forcibly killed
process cannot run cleanup.

## Analysis and Data controls

Analysis (F2): l loads, a/c opens the confirmed one-call analysis dialog, v shows
Results, s exports Markdown by default, d runs the offline analytical demo, and e
shows the optional explanation budget. Tab/Right and Left change result/chart
views; t changes the explained class offline. x cancels future evaluations and
waits for any dispatched request. Inference-only reports need no explanation.
SVG/CSV charts require a completed explanation. See the
[Molecular Guide](data/MOLECULAR_GUIDE.md) for formats and resumption.

Data (F3): 1–7 selects Samples, Datasets, Analyze, Explore, Models, Reference and
Projects. i imports expression, p pastes a table, / searches genes, and [/] changes
samples. r loads a reference, a compares locally, and e exports an offline request.
s exports comparison evidence on Analyze when present, otherwise the manifest.
m attaches a usable comparison to Analysis, replacing its previous input/result.
See the [Expression Guide](data/EXPRESSION_GUIDE.md) and
[Reference Guide](data/REFERENCE_GUIDE.md). These Data operations make no provider call.

## Cohort controls and CLI

F5 opens Cohort Observatory. d loads an invented demo, 1–6 selects views,
arrows inspect confusion cells, n cycles normalization, b toggles an explicit broad
mapping, i toggles unweighted bounds, w toggles IPTW, and [/] pages survival groups.
l loads study/archive JSON; s exports SVG/Markdown/JSON. Use `josh study demo`,
`from-records`, `analyze` or `export` for the offline CLI and `josh schema cohort-study`
for the input contract. See the [Cohort Guide](data/COHORT_GUIDE.md) for limits.

## Clinical controls

Press F4, or load canonical case JSON/a case import bundle with shared Load:

```bash
./scripts/start tui fixtures/synthetic-case.json
```

Clinical starts with a bundled schema 3 synthetic case when no clinical input is
loaded. This remains available outside the repository. Its pages and controls are:

| Key | Action in Clinical |
| --- | --- |
| 1 / 2 / 3 / 4 | Evidence / Request / Results / Help |
| 5 | Import quality report |
| 6 / v | Visuals: scores, IHC, pathway, timeline, evidence counts |
| 7 | Source-linked NICE guidance; Up/Down selects a rule |
| 8 | Clinical context and review history |
| Left/Right | On Visuals: change chart; on Guidance: scroll rule details |
| a | On Guidance: record a reasoned review; 8 then s saves the complete case |
| Tab / Shift+Tab | Next/previous Clinical page |
| Up/Down / j/k | Scroll; on Guidance, select a rule |
| PageUp/PageDown / Home | Page or return to top; Guidance pages scroll rule details |
| b | Load a case import bundle and verify report/sidecars |
| [ / ] | Previous/next imported case; clear its previous result |
| r | Reload the current case file and clear its previous result |
| d | Offline uniform mock; always labeled, no API request |
| c | Confirm one synthetic live Jev call with y |
| s | Export Request, Results, Import, Visuals or Guidance; on Review, save the case |
| ? | Clinical shortcut/help page |

Clinical exports are JSON even if you enter another filename extension. Its case
results use ResultRecord, distinct from molecular InferenceRun. Request exports
omit local case identifiers and credentials; case/review exports retain clinical
context. No score assigns a stage. Guidance is a documented subset of local review
rules, not a complete guideline or a validated decision system. See the
[Clinical Review Guide](CLINICAL_REVIEW_GUIDE.md) for coverage and persistence.

Edit a standalone case JSON in your editor, then reload. A failed load preserves
the current case. Imported bundle cases are fingerprint-checked: make a standalone
copy before editing one. Save recorded reviews with 8 then s before replacing the
case. Quitting asks before discarding recorded unsaved reviews, even from another
section. Clinical results must be explicitly exported; they have no automatic run
archive. While a live call is pending, navigation works, case changes/new runs are
blocked, and quitting waits for completion. A dispatched call may be billed.

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
Clinical sends after c then y. CLI run/classify/explain are themselves explicit
send actions. The pinned model is jev-1.13.0; real-data eligibility remains open.
Local readiness checks do not authenticate the key. Requests are not automatically
retried. See the [0.7.0 validation report](reports/0.7.0-molecular-validation.md) for
the limited synthetic live check and its observed variation.

Development checks are `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets --locked -- -D warnings`, and
`cargo test --workspace --locked`. These verify software behavior, not cancer accuracy.
