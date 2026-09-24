# Load → analyze → results → Markdown

From the repository, run:

```bash
./scripts/start
```

The launcher builds the Rust binary and opens the single JOSH workspace. First-time builds
need registry access, the pinned Rust/Cargo 1.98.1 toolchain and a system C compiler.
No Python runtime, model download, database, Node server or local GPU is required.
The launcher checks for Cargo, Rust and the compiler; `rust-toolchain.toml` selects
the toolchain for rustup installations. It does not install system packages.

## One application, shared navigation

Use the persistent tabs or **F2 Analysis**, **F3 Data**, and **F4 Clinical**. Switching
sections preserves loaded data, unfinished forms, results and running jobs. **l/o**
opens the same Load dialog from navigation; **F1/Ctrl+g** opens help even inside
forms, and **g** opens it from navigation. **q** quits from navigation and
checks unsaved clinical reviews in every section, then finishes
in-flight work before restoring the terminal. Esc closes dialogs without leaving
the workspace. Close the shared Load dialog or quit confirmation before switching
sections. Save results you want to keep: section state is not a persistent project,
and Clinical results require explicit export.

Load molecular or clinical JSON, expression or molecular tables, saved run folders,
expression dataset bundles or case import bundles. JOSH selects the relevant
section from the file. Reference JSON uses **F3 then r**. Case CSV/TSV/JSONL must
first pass through `josh import`; load its resulting bundle. The command line uses
the same loader:

```bash
./scripts/start tui /path/to/file-or-bundle
```

After compatible expression/reference comparison in Data, **m** attaches that
sample to the shared Analysis section. F2 by itself switches sections without
replacing the current analysis. Clinical reviews retain their own case identity;
changing tabs does not silently join records to a different molecular sample.

## Try it without a key

1. In **Analysis (F2)**, press **d** for the bundled offline analytical demonstration.
2. Read **Results**: ranked outcomes, abstention reasons and evidence checks.
3. Press **Tab** to view explanation charts. **t** changes the target offline.
4. Click **Export** or press **s**, enter a new filename ending in `.md`, then Enter.

This demonstration uses invented coefficients and measurements. It is visibly
labeled and never calls Jev. Pressing d replaces the current sample with the demo;
it does not produce simulated cancer predictions for your loaded data.

## Load a file and analyze with Jev

1. Click **Load** or press **l** and enter a file path. Typed molecular JSON opens
   directly. Canonical CSV/TSV, the supported annotated MAF subset and annotated
   VCF subset open a settings form. The [molecular guide](data/MOLECULAR_GUIDE.md)
   documents the columns and encodings; arbitrary expression matrices use the
   Data section below.
2. In the table form, enter the exact sample ID, patient group, source ID, fallback
   assay and optional reference build. Tab moves fields; Ctrl+u clears a field;
   Enter imports. Use `synthetic` only for invented records; the initial data class
   is `deidentified_research`. Errors leave the form editable and preserve the
   previous sample. The import is strict and local.
3. Review observed/unavailable measurements and setup status. Unknown and not-tested
   values remain distinct from measured zero and measured negatives.
4. Click **Analyze** or press **a**. The directory dialog states that Enter sends
   **one** potentially billed request to Jev. Accept the suggested unique directory
   or choose a new path inside an existing parent directory.
5. Results open automatically. The run directory contains `features.json`,
   `request.json`, `inference.json`, `run-status.json` and **`report.md`**.
6. Click **Export** or press **s** for another Markdown copy. Existing files are
   protected. Replacing the Analysis input clears its old result; loading Data or
   Clinical records leaves Analysis intact.

Configure credentials in the shell that will launch JOSH, without putting the key
in shell history:

```bash
read -r -s -p 'TypeSafe API key: ' TYPESAFE_API_KEY
export TYPESAFE_API_KEY
printf '\n'
./scripts/start doctor --format text
./scripts/start
```

Keys stay in the process environment, are never included in a report or archive,
and are not automatically loaded from `.env`. Local readiness confirms that a key
is present, not that the provider has authenticated it. Missing credentials do not
block offline import, saved-run review or the analytical demonstration.

Live inference retains the project's **synthetic-only** restriction. Inspecting real
research data locally does not authorize sending it to the provider. Provider/data
eligibility and cancer validation remain separate prerequisites. Jev is the sole
hosted origin classifier; the app preserves raw scores and explicit abstention.

The shared loader treats CSV/TSV headers `modality` or `id` as molecular input.
For expression data with a gene column named `id`, use **F3 then i** explicitly.
Paths in the shared loader are literal; enter spaces without surrounding shell
quotes. The launcher resolves relative paths from the repository.

For a ready-made live input, load `fixtures/molecular/synthetic-features.json`.
For a table import, use `fixtures/basic-demo/molecular-complete.tsv`, sample
`DEMO-MOL-01`, patient group `DEMO-PAT-01`, source `demo-table`, assay
`invented-panel`, build `GRCh38`, and data class `synthetic`.

## Reopen or export a saved run

Load the run directory using l, or:

```bash
./scripts/start tui /path/to/run
./scripts/start molecular export /path/to/run --output /path/to/new-report.md
```

Both operations are offline. You can also load inference JSON with its matching
`features.json` sidecar, or a self-contained explanation/checkpoint JSON. Reports
work with inference alone, complete explanations, or incomplete explanation checkpoints. They verify hashes and
request/response consistency before rendering. A report contains all ranked
outcomes, decision notes, separate evidence judgments, observations and missingness,
explanation contributions when available, usage, model and processing versions,
and source fingerprints. Exporting never substitutes the input data for results.

## Optional explanations

After a saved live result, **e** offers masked-evidence Shapley explanations with
an explicit ceiling of 512 evaluations, 1,000,000 input tokens and 600 seconds.
These can incur additional charges. Oversized plans are directed to the CLI to
choose an appropriate budget. **x** cancels future explanation calls; in-flight
work may already be billed. Successful calls are checkpointed. The original
`report.md` stays intact; completion adds `explanation-report.md` and the chart
archive. Export with s always uses the currently selected result/target.

After a failed or interrupted inference, inspect `run-status.json` and the archived
request before trying again. An ambiguous call is never automatically retried or
replaced by demonstration data. An incomplete inference directory cannot be opened
as completed results. Use the molecular CLI for explicit explanation resumption.

## Expression data and other workflows

- Expression imports and reference comparison: **F3 Data** in the workspace.
  Its import form uses **Ctrl+D** to preview and **Ctrl+S/Ctrl+Enter** to import.
  The form has no synthetic toggle; for invented data intended for live Jev, use
  `josh dataset import --synthetic` and load the resulting bundle. See the
  [expression guide](data/EXPRESSION_GUIDE.md) for all required options.
- Open an expression bundle: `./scripts/start tui DIRECTORY`.
- Summarized cases and clinical review: **F4 Clinical** in the workspace.
- Check a molecular input locally: `./scripts/start molecular check FILE --format text`.
- Exact request preview: `./scripts/start molecular prepare FILE`.
- One-call CLI analysis: `./scripts/start molecular run FILE --out-dir NEW_DIRECTORY`.
- SVG chart export: `./scripts/start molecular export EXPLANATION --kind svg --output NEW.svg`.

Migration: `josh tui` is the only TUI entry point. The former per-interface flags
have been removed; replace `josh tui --<input-type> PATH` with `josh tui PATH`, or
use the shared Load action. `molecular export` defaults to Markdown; scripts
expecting SVG must pass `--kind svg`. Existing JSON contracts are unchanged.
