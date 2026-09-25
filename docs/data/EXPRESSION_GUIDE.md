# Expression datasets

Expression is an input profile in the single-sample workbench. Open a CSV/TSV or
dataset folder, then use Data for gene mapping, QC, provenance and exploration.
The [reference guide](REFERENCE_GUIDE.md) covers local comparison and confirmed
expression-only preparation for Jev. It does not silently merge with another
sample or a legacy clinical taxonomy.

## Try the sample workbench

Launch `./scripts/start`, then Open `fixtures/expression/synthetic-expression.tsv`.
The basic import form has five fields, with full options under Advanced settings.
Unlike older builds, launching the app or pressing F3 does not preload expression
demonstrations. For the full reference example, use the CLI tutorial below.

## Import an expression matrix

```bash
mkdir -p results
./target/debug/josh dataset detect fixtures/expression/synthetic-expression.tsv
./target/debug/josh dataset import fixtures/expression/synthetic-expression.tsv \
  --dataset-id expression-demo --units tpm --transform log2-one-plus \
  --gene-map fixtures/expression/hgnc-subset.tsv \
  --gene-map-release hgnc-six-gene-fixture-2026-09-22 \
  --platform invented-demonstration --synthetic \
  --out-dir results/expression-demo --format text
./target/debug/josh tui results/expression-demo
```

The destination and its parents are ordinary local paths; the parent must exist and
an existing destination is never overwritten. The six-gene dictionary is a test
fixture, not a complete annotation source. Use a complete HGNC TSV for real inputs
and record the release with `--gene-map-release`. The import snapshots and hashes
that exact dictionary alongside the source, making later mapping reproducible.
The public source and extraction hash are recorded in the
[fixture provenance](../../fixtures/expression/hgnc-subset.provenance.json).

Supported layouts:

```text
# Single sample: provide --sample-id; '#' comments are illustrative, not input rows.
gene    expression
TP53    8.43
KRT7    11.82

# Long table: sample IDs are explicit.
gene    sample_id    expression
TP53    CUP.001      8.43
KRT7    CUP.001      11.82

# Wide matrix: all non-gene columns are sample IDs.
gene    CUP.001     CUP.002
TP53    8.43        4.10
KRT7    11.82       1.20
```

Use actual tab or comma delimiters. CSV quoting is supported. `--delimiter csv|tsv`,
`--layout long|wide`, `--gene-column`, `--value-column`, and `--sample-column` override
inferred structure. A transposed sample-by-gene matrix must currently be converted
before import. Long tables reject extra columns; join external sample annotations
through a future adapter rather than silently interpreting them as expression.

Two-column data needs a sample identity, not a patient profile:

```bash
./target/debug/josh dataset import my-expression.tsv \
  --dataset-id study-001 --sample-id CUP.001 --units tpm \
  --out-dir results/study-001
```

`-` reads the expression input from stdin. The gene dictionary must be a file. Source
samples may contain dots, commas and spaces; IDs are preserved and never used as
output filenames. Nonblank IDs/metadata are bounded to 256 bytes and exclude control
characters. Missing patient groups are allowed. Legacy case migration preserves any
existing patient group but does not infer new relationships.

Dataset metadata flags are `--study`, `--accession`, `--citation`, `--platform` and
`--genome`. Organism is explicitly Homo sapiens in this first implementation. The
source name is the original basename; its contents are archived. Import defaults to
a research-data declaration; `--synthetic` is only for invented inputs. Neither flag
certifies deidentification or authorizes sending data to a provider.

## Units, mappings and QC

Declared units: `unknown`, `counts`, `tpm`, `fpkm`, `normalized`,
`microarray-intensity`, `log2`, `z-score`. Unit/scale metadata is never inferred from
numerical ranges. Storage and exploration support these declarations; this does not
establish cross-platform comparability or inference readiness.

`identity` preserves the parsed numeric value. `log2-one-plus` computes log2(x+1)
from original counts/TPM/FPKM values only. It is a numerical transform, not library
normalization, batch correction or cancer feature selection. Already logged values,
z-scores and unknown units cannot receive this transform. Raw and transformed values
are stored separately. Counts must be nonnegative integers no greater than 2^53−1 (exact f64 range); TPM/FPKM nonnegative;
all parsed measurements must be finite. Negative z-scores are allowed.

Blank, `NA`, `N/A`, `null` and `.` are missing; `0` is measured zero. Invalid numeric
values remain in `original_value`, with parsed/derived values null and QC blocked.
Records are never silently dropped to make a sample pass QC. Malformed table
structure is a fatal import failure.

The HGNC mapper accepts the complete-set TSV with `hgnc_id`, `symbol`, `status`,
`ensembl_gene_id` and `entrez_id`; alias/previous-symbol columns are optional. It uses
approved entries, retains original identifiers and records exact, alias,
version-stripped, ambiguous, unmapped or not-attempted outcomes. Ambiguous aliases
retain candidates; no first-match selection, fuzzy matching or automatic uppercasing
occurs. Multiple records mapping to the same gene block QC instead of being summed.
Use `--gene-ids auto|hgnc-id|symbol|ensembl|entrez` to declare the input namespace.

QC records measured/missing/invalid/zero values, mapping counts, duplicate records,
raw range and issue codes. Mapping rate is unambiguously mapped records divided by
all source records, including missing measurements; it is not sequencing alignment
rate. Per-record issue details are capped at 1,000 per sample; summary counts remain
complete. Import QC records reference compatibility as `not_assessed_no_reference`;
subsequent reference comparison has its own evidence and gates. Import QC is not
updated to represent that separate comparison.

## Inspect, export and reproduce

```bash
./target/debug/josh dataset inspect results/expression-demo --format text
./target/debug/josh dataset explore results/expression-demo \
  --sample DEMO-EXPR-01 --gene TP53
./target/debug/josh dataset export results/expression-demo \
  --sample DEMO-EXPR-01 --kind tsv --output results/expression-demo-01.tsv
./target/debug/josh dataset verify results/expression-demo --reproduce
```

`explore` searches original/canonical identifiers and sorts matching records by raw
expression. `--limit` defaults to 50 and is bounded to 1–1,000; JSON reports both
matching and returned counts. Selecting a sample is mandatory for multi-sample
exports/exploration. `export --kind json` includes the dataset manifest and every
record for the selected sample. CSV/TSV include raw/original/transformed values,
mapping status, sample/dataset IDs, source hash and record/column locators. These are
measurement exports, not tissue-of-origin result reports.

A bundle contains:

| Path | Contents |
| --- | --- |
| `dataset.json` | Completion manifest, samples/assays, configuration, provenance, QC and dictionary |
| `source/input` | Exact original source bytes |
| `source/gene-map.tsv` | Exact selected dictionary, when supplied |
| `samples/000000.jsonl` | Full raw/derived measurements and mappings for one sample; numeric filenames avoid ID/path collisions |

Bundles use owner-only directories/files on Unix. The completion manifest is written
last; an IO failure may leave an incomplete directory that cannot be loaded as a
valid dataset. Artifact hashes detect change relative to the manifest, not malicious
replacement of the whole bundle. Load rejects symlinked artifacts and escaping paths.
`verify --reproduce` additionally recomputes sample measurements/mapping/QC from the
archived source, dictionary and configuration. Import timestamps are intentionally
excluded from deterministic derived comparisons. No provider call is involved.

Exit 0 means the operation succeeded, including QC warnings. Expression import exits
3 when any sample has blocked QC, while preserving its complete data and report.
Fatal errors exit 1; CLI syntax errors exit 2. `--output` cannot be combined with
import/migration because their output is the new bundle directory.

## Ratatui controls

Expression uses the same Data / Results shell as molecular input. Data's selector
offers Overview, Quality, Comparison, Genes and Reference. Menu offers sample
selection, reference loading, local comparison, profile preparation and gene search.
The contextual next action guides Load reference → Compare locally → Prepare analysis.
Preparation has a confirmation; Jev inference is a separate confirmed action.

Tab/Shift-Tab focuses controls and Enter activates them; controls are clickable.
Basic import settings cover source, suggested output folder, two-column sample ID,
units and data class. Advanced settings retains all column, mapping, platform and
transform options. Only explicitly invented records should be declared synthetic.
Ctrl+D detects columns; Ctrl+S or Ctrl+Enter imports. Escape cancels without replacing
the active sample. / searches genes; [/] changes selected samples when idle.
Use Menu to export dataset/comparison evidence or inspect the prepared request.

Changing sample or comparison/reference invalidates the prepared analysis and moves
it to Menu → Earlier runs with its original input. The app never shows that result
as a prediction for the newly selected sample. Failed loads preserve existing data.

## Legacy compatibility and current limits

```bash
./target/debug/josh dataset migrate-case fixtures/clinical-case.json \
  --dataset-id legacy-example --out-dir results/legacy-example
```

Case schemas 1–3 remain unchanged. Migration stores the original bytes, including
clinical context/reviews, as a `legacy_annotations` assay. It uses the original
sample ID where present, otherwise the case ID, and never invents expression values.
Legacy provider requests and fingerprints remain unchanged.

Limits: 64 MiB source and dictionary, 512 samples, 100,000 records per sample,
1,000,000 measurement cells per import, 128 bytes per numeric source cell, 256 MiB
per derived artifact, 64 KiB per derived JSONL record, 32 MiB manifest, and 1 MiB TUI
paste. These are bounded in-memory operations, not a constant-memory streaming
matrix engine. Larger cohorts need further chunking/storage work.

Remaining expression work includes sample-annotation joins, richer transforms/feature
selection, curated/validated cancer references, reference percentiles and gene-family
enrichment. Molecular inference archives, supported VCF/MAF and structured IHC imports,
synthetic cohort inference and evaluation tooling are available through the
[Molecular Guide](MOLECULAR_GUIDE.md). PDF reports and dataset HTTP routes are not
implemented. See [Build Status](../BUILD_STATUS.md) for current and historical scope.
