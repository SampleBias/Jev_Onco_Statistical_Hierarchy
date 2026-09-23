# Jev Onco Statistical Hierarchy — User Guide

## Welcome and scope

JOSH is a Rust molecular research workbench with a Ratatui interface. Start with a
sample and its molecular measurements. Patient forms are not required. Jev is the
origin classifier; Rust handles files, gene identifiers, QC and reference comparison.

The current release imports expression data, explores samples, curates a local
reference, computes numerical similarities and prepares a structured Jev request.
The molecular request is an OFFLINE PREVIEW. Molecular live inference, calibrated
cancer probabilities and a validated unknown/out-of-distribution detector are not
implemented. The legacy summarized-case workflow is separate.

The startup example contains invented measurements. Its provenance remains marked
Synthetic in the Sample/Dataset metadata and exports. The application title is
independent of the loaded dataset. A gene dictionary is not a cancer reference.

## Open and search this guide

Press g from any navigation screen, including while a local job is running.
Press F1 or Ctrl+g from any screen or text editor. Printable g remains available in
paths, sample names and search terms while editing. Forms keep their contents when
you open the guide. Closing the guide returns you to that exact form or screen.

- g, Esc or q: close the guide. These keys do not quit the underlying application.
- /: edit a grep-style regular expression; Enter applies it. Search is case insensitive.
- Ctrl+u: clear the search editor; Esc cancels editing without closing the guide.
- n / N: next / previous match, wrapping at the ends. Matches are highlighted.
- Enter: switch between matching lines and full context at the selected match.
- t: contents, showing section headings. Select with n/N and Enter to read a section.
- c: clear the filter and return to the full guide.
- Up/Down or j/k: scroll; PageUp/PageDown: one visible page; Home/End: beginning/end.

Search examples: gene.*map, TPM|FPKM, ^## , provenance, missing, Jev, reference.
Escape regex punctuation to search literally, for example log2\(.
An invalid pattern is shown as an error; the previous search is preserved. A search
with no matches shows an empty-results message. This guide is compiled into the
binary and works without internet access or an API key. Searches never run a shell.

## First five minutes

1. Launch `josh tui`. The built-in expression example opens locally.
2. On Samples, inspect measured genes, mapping coverage, QC and provenance.
3. Press 4 for Explore. Press /, type TP53 and Enter to filter original/canonical IDs.
4. Use [ and ] to move between samples. Each sample keeps its own data and QC.
5. Press i to import a file, p to paste a table or o to open an existing bundle.
6. Use g to search this guide whenever you need a command or explanation.

Launch an existing molecular dataset: `josh tui --dataset results/my-dataset`.
Quote paths containing spaces. In the local repository use `./target/debug/josh`
in place of `josh` if the binary has not been installed on PATH.

## Main navigation and keyboard

1 Samples: quality cards, observed expression histogram, sample and source metadata.
2 Datasets: dataset summary, detection settings, notices and data dictionary.
3 Analyze: pipeline status, reference comparison, gates and evidence export.
4 Explore: original/canonical genes, raw/transformed values and source record:column.
5 Models: pinned Jev version and current inference capabilities.
6 Reference: loaded release, source, classes, processing compatibility and limitations.
7 Projects: current local workspace. A project catalog is not implemented yet.

Tab / Shift+Tab move between views. Number keys select a view directly.
Up/Down, j/k, PageUp/PageDown and Home scroll content. [ / ] select a sample.
i imports; p opens the paste editor; o opens an existing dataset directory.
r opens a reference JSON; a compares the selected sample with that reference.
s exports the current analysis evidence on Analyze; elsewhere it exports the dataset
manifest. e exports the complete offline molecular Jev request plus its evidence.
x requests cancellation of a local job. q or Ctrl+C quits and restores the terminal.
? shows a shortcut reminder. g opens the complete guide. F1/Ctrl+g work inside forms.

Jobs run in the background. Opening help never submits a form, runs analysis or
sends data. A pending job may finish while you read. Sample-changing actions stay
locked during a job. If a bundle write has started it completes before exit; an
abandoned partial directory is never reported as a valid dataset.

## Import files and paste data

Accepted expression inputs are UTF-8 CSV/TSV with a header: two columns for one
sample, long form with sample_id, or a wide gene-by-sample matrix. Units are declared,
never guessed from value ranges. See the complete Expression Guide later in this
manual for configuration, CLI commands and size limits.

Press i. Tab/Shift+Tab or Up/Down changes the active field. Fill source path, a NEW
output directory and dataset ID. Set units, layout, column names and transform as
needed. For two columns supply a Sample ID; for a wide matrix leave it empty.
The HGNC TSV path and its release label must either both be supplied or both omitted.
The platform field should identify the actual assay/processing family, not a disease.
Press Ctrl+D to preview detection. Correct fields if needed. Ctrl+S imports.
Esc closes a form. Backspace deletes the last character. Paste paths using your
terminal's paste shortcut. F1 opens help without losing any fields.

Press p to paste a complete CSV/TSV table using bracketed paste. Include its header.
Enter continues to the import settings; it does not insert a new row. Paste newlines
from your clipboard. The paste limit is 1 MiB; use a file for larger matrices.
TUI paths cannot be '-' because the terminal owns stdin. The CLI accepts stdin.

Imports preserve raw bytes, mapping assets, original IDs/values, transformations and
QC. Existing output directories are protected. A blocked-QC import is still archived
for inspection. It is not approved for analysis just because it was saved.

## Interpret the dashboard and QC

Measured counts include valid parsed values; missing values are distinct from zero.
The mapping gauge is the fraction of ALL source records unambiguously mapped, including
records whose expression value is missing. It is not prediction confidence.
The histogram shows 16 equal-width bins over observed raw expression values; its
height represents counts of measured genes. Its endpoints show the raw range.
No plot animates or fabricates measurements. The spinner indicates an actual job.

Pass means these import checks passed. Warning means information needs attention.
Blocked means invalid values, duplicates or other fatal QC issues must be resolved
before reference comparison. A pass is not scientific validation or clinical suitability.
Unmapped/ambiguous identifiers remain inspectable; no arbitrary candidate is selected.

## Explore and inspect provenance

Press 4 or / to open gene exploration. Search matches original IDs, canonical symbols
and HGNC IDs, case-insensitively. Clear the search and press Enter to show all genes.
Rows are sorted by raw expression. Source record:column uses a parsed table-record
ordinal including the header, not necessarily a physical line number in quoted CSV.
Raw and transformed values are shown separately. Missing values show a dash.

The Sample and Dataset screens show source file hashes, mapping release, processing
configuration and import metadata. Export a sample to inspect every original value.
`josh dataset verify DIRECTORY --reproduce` checks that stored derived records and QC
can be rebuilt from archived input and mapping data. Hashes detect accidental changes;
they are not a signature or proof of correct source labels.

## Reference comparison and Jev request preview

Build a reference from labeled known-origin data with `josh reference build`.
Open it with r. Press a to compare the current sample. The Reference Guide below
contains the complete annotation format and worked CLI example.

The first comparison family accepts explicitly matched TPM → log2(x+1), human gene
mapping, platform, genome declaration and pinned dictionary. Other data can still be
imported and explored. Matching text metadata does not remove batch effects.

Read Pearson r as a signed similarity in [-1, 1], not a percentage. Negative values
are retained. Constants have undefined correlation. Gates can return incompatible,
insufficient_data or undefined_similarity instead of a ranked usable comparison.
No validated OOD cutoff or automatic diagnosis is inferred from correlation.

On Analyze, s archives evidence. e saves an OFFLINE typed Jev request to a new JSON
file. The request uses Choice and independent Noul questions, with unknown and
other_origin options. Query IDs, paths and patient identifiers are excluded from the
provider state. Nothing is sent by comparison, export or request preparation.

## Export, reproduce and CLI conventions

Global --format json (default) produces machine-readable output; --format text gives
a summary. --output NEW_FILE protects existing files. Parent directories must exist.

- `josh dataset --help`: detect, import, inspect, explore, verify, migrate-case, export.
- `josh reference --help`: build, inspect, compare, prepare.
- `josh schema sample`: canonical sample JSON Schema.
- `josh schema dataset`: dataset manifest JSON Schema.
- `josh schema reference`: reference release JSON Schema.
- `josh schema molecular-evidence`: comparison evidence JSON Schema.
- `josh doctor`: local key/setup status, without printing credentials or making a call.

Exit 0 means success, 1 a command error, 2 invalid CLI arguments and 3 blocked QC or
comparison gates. JSON errors carry a code and safe message. Full molecular exports
are JSON; measurement export also supports CSV/TSV. PDF reports remain planned.

Keep original dataset bundles, labels, gene dictionary, reference JSON and output
artifacts. Reuse the same binary/pipeline and pinned reference for a reproducible
comparison. A provider response cannot be reproduced locally until it has been
archived; this milestone has no live molecular result to replay.

## Jev credentials and legacy workflow

Offline import, exploration, comparison and help need no key. For the existing
synthetic-case live command set TYPESAFE_API_KEY in the launching shell. To avoid
putting a key into shell history in Bash:

read -r -s -p 'TypeSafe API key: ' TYPESAFE_API_KEY
export TYPESAFE_API_KEY
printf '\n'

`josh doctor` reports whether the variable is present without displaying it. A new
terminal may not inherit a variable set in another shell. Do not commit keys.

`josh tui --legacy`, --case FILE and --batch DIRECTORY open the legacy interface.
Its numbered Help view documents the summarized-case workflow, demo, request preview,
NICE review and visualization pages. g/F1/Ctrl+g also open this shared guide.
The legacy c key confirms a synthetic live Jev call; it sends evidence and may incur
charges. Molecular a/e operations are separate and remain offline. The Terminal Guide
included below documents every legacy shortcut and export behavior.

## Troubleshooting and limitations

No genes mapped: check namespace and use the complete HGNC dictionary for real input.
The six-gene fixture is only for demonstration. Ambiguous aliases are not guessed.
Unknown units: provide the actual source scale; ranges cannot establish units.
Blocked duplicates: resolve source duplication under a reviewed assay-specific policy;
JOSH does not silently sum duplicate genes.
Incompatible reference: inspect units, transform, platform, genome and dictionary hash.
Insufficient overlap: inspect missing/mapped genes and the reference's frozen thresholds.
Query overlap: use an independent source/sample; do not test a reference against itself.
Constant profile: no correlation can be calculated; inspect source values and processing.
No reference loaded: press r and select a reference JSON; a dataset directory is different.
Output exists: choose a new name. Imports and exports never silently overwrite files.
Search has no matches: press / and change regex, or c to clear; t lists sections.
Small terminal: enlarge for dashboard panels; every view and guide has a compact fallback.

Not implemented: real validated cancer reference distribution, cohort inference,
multimodal VCF/MAF/IHC integration, repository accession downloads, methylation/CNV,
live molecular Jev classification, clinical calibration, validated OOD, PDF exports,
and a persistent project catalog. The phase documents track these open deliverables.
