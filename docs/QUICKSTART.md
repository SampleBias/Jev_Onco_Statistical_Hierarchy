# Quickstart

From the repository, run:

```bash
./scripts/start
```

The launcher builds the Rust terminal app with pinned Rust 1.98.1. Cargo and a
system C compiler must be installed; first builds need registry access. No Python,
Node, GPU, database or local model download is required.

## First sample

1. Choose **Try a sample**, select a bundled input, then **Open selected**.
2. Review **Data**: sample identity, evidence, missing measurements and readiness.
3. Choose **Run analysis**, review the new run directory, then confirm **Send 1 request**.
4. Read **Results**. Use **View ▾** for explanation charts when available.
5. Choose **Menu → Export**, or press s, to save another report.

The five bundled inputs are invented evidence, not prewritten predictions. Loading
is local and needs no key. Live analysis and optional explanations use Jev and may
incur charges. Jev stays pinned to jev-1.13.0.

For an entirely offline result and charts, choose **Menu → Offline chart
demonstration**. Its scores are explicitly invented. Existing samples remain in
**Menu → Previous samples**. No data is sent.

## Open your data

Choose **Open…** (o/l). Browse folders, filter filenames, or paste a path into Path.
Click or use arrows/Enter. Paths with spaces, quotes and ~/ work; shell expressions
are not evaluated.

Molecular JSON opens directly. Molecular CSV/TSV and supported MAF/VCF open a
review form with detected sample IDs. Choose a sample if several are present;
review the patient group, source, assay and data class. Patient group initially
equals sample ID: correct it for repeated patients. Data class defaults to
deidentified_research; select synthetic only for invented records.

Expression tables open basic import settings. **Advanced settings** retains
columns, mapping, platform, transform and dataset identity. Imported datasets have
one explicit sample selection, local reference comparison and confirmed preparation
of an expression-only inference profile. They do not silently merge with molecular
or legacy clinical records.

Loading a new sample preserves the previous session under **Menu → Previous
samples**. Cancelling or failing an import keeps the previous sample active.
Keep original inputs and saved run folders: session history is not persisted.

## Configure the key

In the shell that will launch JOSH:

```bash
read -r -s -p 'TypeSafe API key: ' TYPESAFE_API_KEY
export TYPESAFE_API_KEY
printf '\\n'
./scripts/start doctor --format text
./scripts/start
```

The key is not shown, saved in reports, or automatically loaded from .env.
“Key set · unverified” means a credential is present, not authenticated.

Live inference remains **synthetic-only**. Inspecting real data locally does not
authorize provider transmission. Accuracy and clinical calibration remain unvalidated.

## Results, charts and saved runs

Results has Summary, Paired ring + scatter, Feature ring, Feature scatter and
Waterfall in one **View ▾** selector. p opens the paired figure; v shows Summary.
Existing charts and numerical methods are preserved.

**Explain result** is optional and has a separate confirmation for additional
budgeted calls. An inference alone does not provide feature attributions. A sent
request cannot be unsent; completed explanation calls are checkpointed.

Successful inference automatically saves features.json, request.json,
inference.json, run-status.json and report.md. Open that folder to review offline:

```bash
./scripts/start tui /path/to/run
./scripts/start molecular export /path/to/run --output /path/to/new-report.md
```

TUI inference-only JSON exports are self-contained sample-run bundles. Older
inference JSON still needs its features.json sidecar. Explanation archives remain
compatible. Use the original run folder for additional live explanation work.
Existing files are never overwritten. Failed requests are not automatically retried.

## Navigation

**Data / Results** are the primary views. **Workbench ▾ → Cohort studies** opens
the secondary research workspace. Legacy clinical files open explicitly labelled,
read-only compatibility views; no unrelated clinical demo is automatically loaded.

Tab/Shift-Tab moves focus; Enter/Space activates. Main controls, forms, menus and
the file browser are clickable. Esc cancels a dialog. m opens Menu; F1 opens
searchable help even in forms; F2 returns to the sample workbench; F5 opens studies.
q quits outside editors and waits safely for in-flight work.

See the [User Guide](USER_GUIDE.md), [terminal/CLI reference](TERMINAL_GUIDE.md),
[molecular formats](data/MOLECULAR_GUIDE.md) and [expression formats](data/EXPRESSION_GUIDE.md).
