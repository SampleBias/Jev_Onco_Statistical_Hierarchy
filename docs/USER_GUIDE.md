# Jev Onco Statistical Hierarchy — User Guide

## Start and scope

Launch with `./scripts/start` or `josh tui [INPUT]`. JOSH is a Rust terminal
research workbench, using pinned Jev `jev-1.13.0`. Single-sample analysis is the
primary workflow; cohort research is secondary.

The app starts empty, not with unrelated demo patients. Choose **Open data…**
to browse a file or saved run, or **Try a sample** for one of five built-in
invented inputs. These contain evidence, not prewritten predictions.

Live requests remain synthetic-only. Cancer accuracy, clinical calibration and
validated out-of-distribution detection are not established. This UX change does
not establish parity with the OncoNPC paper.

## One sample: Data and Results

**Data** shows the selected input, evidence, missingness, provenance and readiness.
**Results** shows that input's rankings, abstention, separate evidence judgments and
optional explanations. Raw model scores are not calibrated patient probabilities.

The toolbar contains Data, Results, Open, one contextual next action, and Menu.
The next action is explicit: Run analysis, Explain result, Export report, or an
expression preparation step. Loading does not send data to Jev.

**Workbench ▾** switches between the sample workbench and **Cohort studies**.
Switching workspace preserves the active sample and any running task.
**Menu → Previous samples** restores a previously opened sample with its own
evidence and results. This history is session-local, not a persistent project;
retain original inputs and saved run folders.

## Load and validate

Open supports directories, filtering, hidden-file visibility, Recent, and bundled
samples. Use arrows and Enter, click Open selected, or double-click an entry.
The optional Path field accepts spaces, surrounding quotes and ~/; relative paths
use the displayed folder. No shell commands or environment variables are evaluated.
Ctrl+L focuses Path; Ctrl+U clears it.

| Input | What happens |
| --- | --- |
| Molecular feature JSON | Validated locally, then shown in Data |
| Molecular CSV/TSV, supported MAF/VCF | Detected sample IDs and row counts; review metadata before loading |
| Saved run folder or inference JSON with features.json beside it | Verified Results, with explanation if present |
| Self-contained explanation/checkpoint JSON | Verified archived input and results |
| Expression CSV/TSV | Basic import settings, with Advanced settings available |
| Dataset folder or dataset.json | Locally verified expression data |
| Canonical clinical case or case bundle | Explicit read-only legacy compatibility view |
| Cohort study/report JSON | Cohort studies |
| Expression reference release | Attached only to an already opened expression dataset |

Molecular tables use a modality column. Expression gene columns named id are no
longer misrouted merely because of that name. Legacy case tables using case_id
still require the existing CLI case importer; see the [Import Guide](data/IMPORT_GUIDE.md).

For molecular tables, the first detected sample is preselected; **Choose sample**
opens a selector when several exist. Review the patient group (initially the sample
ID), source, fallback assay and reference build. Repeated samples from the same
patient need the same explicitly reviewed patient group. Guided CSV/TSV/MAF imports
select one sample while preserving the whole-file hash and source record references.
VCF remains restricted to the documented annotated single-sample subset.

Data class defaults to deidentified_research. Change it to synthetic only for
invented records. The app does not infer eligibility from a filename. Import
errors remain editable; cancellation preserves the previously active sample.
Unknown, not-tested, zero and measured-negative values stay distinct.

## Analyze with Jev

1. Review Data and resolve readiness blockers.
2. Choose **Run analysis**. Review the destination and the one-request notice.
3. Confirm **Send 1 request**. This sends the synthetic sample to api.typesafe.ai
   and may incur charges. No automatic retry or demonstration fallback is used.
4. Results open on success. Features, request, inference, run status and report.md
   are saved automatically in a new run folder.
5. Use **Menu → Export** or s for a new report. Existing files are protected.

Set TYPESAFE_API_KEY in the launching process environment. Keys are never displayed
or written into archives; .env is not loaded automatically. “Key set · unverified”
means presence only, not successful authentication. **Menu → Inspect exact Jev
request** shows the prepared payload locally without credentials or a provider call.

After an ambiguous failure, inspect run-status.json before choosing a new run.
**Cancel job** prevents future work where possible; an already-dispatched request
cannot be unsent. Quit waits for in-flight work and restores the terminal.

## Explanations and visualizations

Results has one **View ▾** selector: Summary, Paired ring + scatter, Feature ring,
Feature scatter and Waterfall. No molecular visualization was removed. p opens
the paired figure; v returns to Summary. With chart content focused, Left/Right
cycles views; Up/Down selects evidence and t changes the target offline.

**Explain result** opens a separate budget confirmation for additional Jev calls:
up to 512 evaluations, 1,000,000 input tokens and 600 seconds with 16 paired
permutations. Confirmation-only dialogs initially focus Cancel. Successful calls
are checkpointed; CLI resumption remains available. An inference alone does not
provide measured attributions, and no missing chart values are fabricated.

Use **Menu → Offline chart demonstration** for invented scores and coefficients
without a key. Opening it preserves the previous sample in session history.
It never predicts an arbitrary loaded sample.

The ring represents absolute contribution magnitude, while scatter/waterfall show
signed contributions. Feature-rank and compatible-background percentile axes are
labelled distinctly. Attribution sampling error is not clinical uncertainty.
See the [Molecular Guide](data/MOLECULAR_GUIDE.md) and
[OncoNPC research guide](references/ONCONPC_GUIDE.md).

## Expression is an input, not a separate app

Open an expression table or existing dataset. Basic import fields cover source,
a suggested new output folder, optional two-column sample ID, units and data class.
**Advanced settings** retains layout, columns, gene mapping, platform, transform
and dataset identity. Ctrl+D can preview detected structure; no settings are guessed
from biological expectations. Unknown units remain unknown.

Data's selector provides Overview, Quality, Comparison, Genes and Reference.
**Menu → Select sample in dataset** changes the explicit active sample.
The contextual actions guide **Load reference → Compare locally → Prepare analysis**.
Preparing an expression-only profile asks for confirmation; it uses the selected
sample and the reference's frozen taxonomy. It never silently combines molecular
and clinical probabilities. Run analysis is a separate, confirmed step.

Changing the dataset sample, comparison or reference invalidates its prepared
analysis. That earlier analysis moves to **Menu → Earlier runs**, with its original
input, rather than appearing as a current prediction. See the
[Expression Guide](data/EXPRESSION_GUIDE.md) and [Reference Guide](data/REFERENCE_GUIDE.md)
for scientific compatibility requirements.

## Cohort studies and legacy records

Choose **Workbench ▾ → Cohort studies**, or F5. Open a frozen study or explicitly
choose Try study demo. The view selector retains Overview, Confusion, Survival,
Treatment, Calibration and Protocol. Menu contains normalization, survival bounds,
supplied-propensity weighting and curve navigation. Exports support SVG, Markdown
and reopenable JSON. No provider call is hidden in these local analyses.
See the [Cohort Guide](data/COHORT_GUIDE.md) for method limitations.

Opening a clinical case shows **LEGACY**, with its own identity and contract.
Evidence, request inspection, IHC/pathway/timeline/evidence visualizations, existing
guidance and review history remain available. Export record saves a complete case.
New clinical classification and guideline review are not offered in the main TUI;
their historical CLI/API contracts remain compatible. Clinical and molecular
taxonomies are not silently mapped or linked by matching display names.

## Keyboard, mouse and help

Tab/Shift-Tab moves visible focus; Enter or Space activates a control. Main actions,
selectors, menus, browser and forms also accept mouse clicks. Escape closes the top
dialog; opening help never submits a form. Ctrl+U clears a field. Ctrl+Enter submits
an import form; inspect confirmation notices before submitting any live action.

o/l opens data; m opens Menu; Ctrl+B opens sample examples; F2 returns to the
workbench; F5 opens studies; 1/2 selects Data/Results in the sample workspace.
F3 is a compatibility shortcut for Data; F4 explains how to open legacy records
instead of creating an unrelated demonstration case. q quits outside editors.

F1/Ctrl+G opens searchable offline help, including inside forms. / searches a
case-insensitive regex; t shows headings; n/N steps through matches; Enter toggles
matching lines/full context; c clears the filter; Esc or q closes help. Search
never executes code. References and the current objective audit are bundled;
rebuild after changing documentation. Try searching OncoNPC, provenance or units.
