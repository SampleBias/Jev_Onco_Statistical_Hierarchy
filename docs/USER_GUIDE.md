# Jev Onco Statistical Hierarchy — User Guide

## Start and scope

JOSH 0.8.0 is one Rust terminal workspace for molecular research: load data,
analyze with Jev, inspect results and export Markdown. Clinical cases and expression
data have their own sections within the same session. Jev is the hosted origin
classifier; Rust handles import, QC, comparison, archives and explanations.

Run `./scripts/start` from the repository, or `josh tui` if installed on PATH.
The launcher checks Cargo, Rust and a C compiler, then builds with pinned Rust 1.98.1.
First builds need registry access; the launcher does not install prerequisites.
See [Quickstart](QUICKSTART.md) for setup. Examples using `josh` can also be run
as `./target/debug/josh` after building.

Local workflows need no key. Live requests accept declared synthetic data only.
Cancer-specific accuracy, calibration and validated out-of-distribution detection
are unavailable. The synthetic declaration does not verify data provenance.

## Guided analysis: load, analyze, results, Markdown

In Analysis (F2), use the action buttons or these keys:

1. l loads molecular feature JSON, a canonical molecular CSV/TSV, supported annotated
   MAF/VCF, or a saved run. Molecular tables open metadata settings: enter the exact
   sample ID, patient group, source ID, fallback assay and optional reference build.
   Tab/Shift+Tab selects fields, Ctrl+u clears, Enter imports. The initial data class
   is deidentified_research; use synthetic only for invented records.
2. Review observed/unavailable measurements and setup status. a opens a new run-directory
   dialog. Enter confirms one potentially billed synthetic Jev request. Its parent
   directory must exist; an existing run directory cannot be reused for a new inference.
3. v shows readable rankings, abstention reasons and separate evidence checks.
   Successful inference automatically saves features.json, request.json,
   inference.json, run-status.json and report.md in the chosen directory.
4. s exports another report to a new .md or .markdown file. No explanation is needed
   to read or export results. Up/Down and PageUp/PageDown scroll Results.

Use d in Analysis for the offline analytical demo. It replaces the current analysis
with the bundled invented sample and scores; it never predicts arbitrary loaded data.
Wait for completion, inspect Results, then s exports Markdown without a key.

Failed molecular imports preserve the current sample and keep settings editable.
Replacing the Analysis input clears its previous result. Loading Data or Clinical
records leaves Analysis intact. Existing exports and run folders are protected.
After a failed call, inspect run-status.json before trying again: ambiguous requests
are never automatically retried or replaced by demo results.

## Shared loading and workspace navigation

F2 Analysis, F3 Data, F4 Clinical and F5 Cohort select sections; clicking the persistent tabs
does the same. Section forms, results and jobs survive a switch. Close the shared
Load dialog or quit confirmation before switching. l/o opens Load from navigation;
`josh tui PATH` uses the same loader at startup.

| Input | Destination |
| --- | --- |
| Molecular FeatureSet JSON | Analysis input |
| Canonical molecular CSV/TSV or supported annotated .maf/.vcf | Analysis import settings |
| Run directory containing features.json and inference.json | Verified Analysis results, with explanation/checkpoint if present |
| Inference JSON beside features.json; self-contained explanation/checkpoint JSON | Verified Analysis results |
| Expression CSV/TSV | Data import settings |
| Dataset bundle directory or its dataset.json | Data |
| Canonical case JSON; case import bundle or its manifest.json | Clinical |
| Cohort study JSON; cohort report archive JSON | Cohort |

CSV/TSV routing uses headers: case_id requires CLI case import first; modality or id
selects molecular import; other headers select expression import. If your expression
gene column is named id, use F3 then i to choose expression import explicitly.
Reference releases use F3 then r; they are not shared-loader inputs. Case
CSV/TSV/JSONL must pass through `josh import` before loading the output bundle.

Paths in the shared loader are literal relative or absolute paths: do not add shell
quotes, ~ or environment variables. Quote paths containing spaces only in shell
commands. The launcher resolves relative paths from the repository; a directly
launched binary uses its working directory. The TUI reserves stdin for keys.

Loading is blocked while any section has a running job. Failed loads preserve
previous data. Changing Data's dataset, sample or reference clears its comparison.
Analysis changes only when you load a new Analysis input, press d there, or attach
a usable comparison with Data's m key. Clinical cases retain a separate identity;
switching sections never joins them to molecular samples.

F1/Ctrl+g opens shared help inside forms; g does so from navigation. q quits from
navigation, and Ctrl+C requests quit outside help. Recorded unsaved clinical reviews
are checked even from another section: return to F4, then 8 and s to save the case,
or confirm discarding them. Quit requests cancellation at safe boundaries and waits
for in-flight work. Export results you want to keep before quitting: section state
is not a persistent project, and clinical calls are not automatically archived.
Esc closes dialogs; it does not stop a job or leave a section.

## Paper-inspired cohort workspace

F5 then d loads an explicitly synthetic classification/outcome study without an
API call. Six views show confusion, survival by predicted type, treatment
concordance, calibration and provenance. l loads frozen studies; s exports
SVG/Markdown/JSON. n switches normalization; i shows unweighted 95% survival bounds;
w selects supplied-propensity IPTW. See the [Cohort Guide](data/COHORT_GUIDE.md)
for schemas, methods and limits. This does not establish predictive equivalence.

## Molecular explanations and exports

In Data (F3), m attaches a usable expression comparison and its reference taxonomy
to Analysis, replacing the previous Analysis input/result. Without one, it reports
what is missing and preserves Analysis. F2 alone only switches sections.

In Analysis, e offers masked-evidence Shapley explanations for an archived live run.
The dialog shows ceilings of 512 evaluations, 1,000,000 input tokens and 600 seconds.
These are additional provider calls and may be billed. Oversized plans require
explicit CLI configuration. x cancels future evaluations; a dispatched request
cannot be unsent. Successful calls are checkpointed. Completion adds
explanation-report.md and explanation.json; the original report.md stays intact.
Use the molecular CLI for explicit resumption of interrupted explanations.

Analysis chart controls:

- Tab/Right cycles Results, circular/scatter and waterfall views; Left goes back.
- Up/Down on charts selects a feature/group and shows measurements and provenance.
- t changes the explained target offline using cached distributions.
- s exports the currently selected result/target to a new file.

The outer ring groups categories; inner sectors show absolute contribution size.
The scatter shows signed contributions in percentage points. Its X axis is feature
rank unless a compatible background supports a reference percentile. Raw values and
units stay in the inspector. A large negative contribution opposes the selected
class even when its circular sector is large. Narrow terminals use tables/bars.
Baseline plus signed contributions reconciles with the full raw score; sampling
uncertainty is not a clinical confidence interval.

| Export | Requirement and contents |
| --- | --- |
| .md / .markdown in Analysis | Verified inference; all rankings, reasons, evidence, measurements/missingness, usage and provenance; explanation contributions when available |
| .svg / .csv in Analysis | Completed explanation; charts or contribution table |
| .json in Analysis | Loaded explanation archive, otherwise the inference record |
| CLI molecular export | Defaults to Markdown regardless of filename; accepts run directories, inference with sidecar, or explanation/checkpoint JSON |
| CLI molecular export --kind svg/csv/json | Requires a completed explanation JSON file |

Markdown supports inference alone and incomplete explanation checkpoints. An
inference-only JSON export needs its matching features.json beside it to reopen;
keep the original run directory. Self-contained explanation archives include the
features. Hash and request/response checks detect inconsistency, not authenticity
or scientific validity. Full formats, budgets and examples are in the
[Molecular Guide](data/MOLECULAR_GUIDE.md), also included in searchable help.

## Open and search this guide

Press g from navigation, or F1/Ctrl+g inside a form. Opening help preserves form
contents and never submits a request. Jobs can finish while you read. Printable g
remains ordinary text in paths and editors.

- g, Esc or q outside the search editor closes help and returns to the application.
- / edits a case-insensitive regular expression; Enter applies it.
- Ctrl+u clears the search editor; Esc cancels editing without closing help.
- n/N selects next/previous matches, wrapping at the ends.
- Enter switches between matching lines and full context at the selected match.
- t lists section headings; n/N selects a heading and Enter opens its context.
- c clears the filter; Up/Down or j/k scrolls; PageUp/PageDown pages; Home/End jumps.

Try gene.*map, TPM|FPKM, provenance, missing or Jev. Escape regex punctuation for
literal searches, such as log2\(. Invalid patterns show an error and preserve the
previous search. The user, molecular, expression, reference and terminal guides
are compiled into the binary. Rebuild after editing them. Help needs no internet
or key, and searches never execute shell commands.

## Data views and keyboard

Press F3. Its bundled example has two invented expression profiles and six gene
mappings, with synthetic provenance. A gene dictionary is not a cancer reference.

| Key/view | Purpose |
| --- | --- |
| 1 Samples | Measurements, QC, histogram and provenance |
| 2 Datasets | Manifest, configuration, notices and dictionary |
| 3 Analyze | Reference comparison, gates and evidence |
| 4 Explore | Original/canonical genes, raw/transformed values and source locators |
| 5 Models | Jev model and inference capabilities |
| 6 Reference | Release, classes, compatibility and limitations |
| 7 Projects | Current workspace summary; no persistent project catalog |

Tab/Shift+Tab changes views. Up/Down, j/k, PageUp/PageDown and Home scroll.
[/] selects samples. / searches original IDs, canonical symbols and HGNC IDs;
an empty search restores all rows. Rows are sorted by raw expression.

- i configures an expression-file import; p opens the paste editor; l/o uses shared Load.
- r opens reference JSON; a compares the selected sample locally.
- s exports comparison evidence on Analyze when present, otherwise the dataset manifest.
- e exports an offline molecular Jev request with evidence; it sends nothing.
- m attaches a usable comparison to Analysis for a separate, confirmed Jev run.
- x requests local job cancellation; ? shows a shortcut reminder.

Data's sample-changing actions stay locked during its jobs. Cancellation waits for
a safe processing boundary. A bundle write already started finishes before exit.

## Expression import, QC and provenance

Expression input is UTF-8 CSV/TSV with a header: two columns for one sample, long
form with sample_id, or a wide gene-by-sample matrix. Declare units from the source;
JOSH does not infer them from value ranges. A transposed matrix needs conversion.

In Data's import form, Tab/Shift+Tab or Up/Down selects fields. Set the source, a NEW
output directory and dataset ID, then units, layout, column names and transform.
Supply Sample ID for two-column data; leave it empty for wide matrices. HGNC path
and release must be supplied together. Platform identifies the processing family.
Ctrl+D previews detection; Ctrl+S or Ctrl+Enter imports. Backspace deletes text;
Esc closes the form. These controls differ from the molecular import form.

The expression form declares imported data deidentified_research and has no synthetic
toggle. For invented expression data intended for live Jev, use the dataset CLI
with --synthetic and load the resulting bundle. Use the CLI for metadata absent
from the form, including genome and study.

p accepts bracketed-paste CSV/TSV including its header. Paste multiline text from
the clipboard; Enter continues to settings. The paste limit is 1 MiB; use files for
larger inputs. Dataset CLI input supports stdin; TUI inputs require file paths.

Imports retain source bytes, dictionary, original identifiers/values, transformations
and QC. Blank/NA values are missing; zero remains measured zero. Invalid values and
duplicate mapped genes block QC instead of being silently dropped or summed.
Ambiguous mappings retain candidates. Mapping rate counts unambiguously mapped
records divided by all records, including those with missing expression.
The histogram counts observed raw measurements; it is not prediction confidence.

Blocked-QC imports are archived for inspection but cannot enter reference comparison.
A pass means import checks passed, not scientific validation. Use `josh dataset export`
for complete measurement records; TUI manifest export does not contain those records.
`josh dataset verify DIRECTORY --reproduce` recomputes records and QC from archived
source, dictionary and settings. Keep the original bundle for reproducibility.
See the [Expression Guide](data/EXPRESSION_GUIDE.md) for formats, commands and limits.

## Reference comparison and interpretation

Build with `josh reference build` using independently curated known-origin labels.
In Data, r loads the release and a compares the current sample. The initial comparison
family requires human TPM → log2(x+1), matching platform/genome declarations and
an exact pinned gene dictionary. Other scales remain importable and explorable.
Matching metadata does not remove batch effects.

Pearson r is signed similarity in [-1,1], not a percentage or cancer probability.
Negative values remain visible; constant profiles have undefined correlation.
Compatibility, overlap, QC and identity gates can block comparison/request preparation.
No validated OOD threshold or diagnosis is derived from correlation. The
[Reference Guide](data/REFERENCE_GUIDE.md) documents curation and interpretation.

## Credentials, CLI conventions and troubleshooting

Set TYPESAFE_API_KEY in the shell that will launch JOSH. In Bash, to avoid recording
the key in command history:

```bash
read -r -s -p 'TypeSafe API key: ' TYPESAFE_API_KEY
export TYPESAFE_API_KEY
printf '\n'
```

.env is not loaded automatically. Restart JOSH after changing its launching
environment. `josh doctor` reports key presence without printing it or contacting
Jev. `josh molecular check FILE` also reports input blockers; ready does not mean
that credentials have been authenticated.

Most CLI commands default to JSON; --format text gives summaries where supported.
Explicit exports use their selected format; molecular export defaults to Markdown.
--output NEW_FILE protects existing files, and parent folders must already exist.
Exit 0 means command success, 1 an application error, 2 invalid arguments. Case
import uses 3 for rejected records, expression import for blocked QC, and reference
comparison/preparation for gate failures. A readiness check can exit 0 with
ready:false: inspect its blockers. See [Terminal Guide](TERMINAL_GUIDE.md) for details.

Clinical (F4) starts with a bundled synthetic case. c opens live-call confirmation;
y sends it. On Guidance, a records a review; 8 then s saves the complete case.
Clinical shortcuts and export formats are documented in the Terminal Guide.

| Symptom | Check |
| --- | --- |
| Missing key | Export it in the launching shell, restart JOSH, run doctor |
| Research data blocks live analysis | Synthetic-only eligibility is enforced; local inspection remains available |
| No genes mapped | Check namespace and dictionary; the six-gene fixture is only a demo |
| Reference incompatible | Check units, transform, platform, genome and dictionary hash |
| Insufficient overlap / constant profile | Inspect missing/mapped values, QC and frozen reference thresholds |
| Reference won't load through l | Use F3 then r for reference JSON |
| Output exists | Choose a new filename/directory |
| Interrupted inference | Inspect run-status.json and request.json; no automatic retry |
| Help search has no matches | Change the regex with /, clear with c, or list contents with t |
| Small terminal | Enlarge for full panels; compact views omit detail |

Not implemented: curated validated cancer references, repository accession downloads,
methylation processing, clinical calibration, validated OOD, PDF reports and a persistent
project catalog. Typed molecular import, synthetic inference, explanations and cohort
metric tooling are available. The [build status](BUILD_STATUS.md) separates current
capabilities from historical milestones and scientific work still required.
