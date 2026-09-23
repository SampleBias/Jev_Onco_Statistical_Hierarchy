# Clean data demo

A small, fully offline demo for testing import, missing-value handling, expression
QC, terminal graphics and export. All measurements and patient/sample IDs are
invented. The included six-gene HGNC dictionary contains real identifier mappings;
it supplies no cancer labels. No Jev key or network request is needed.

If you are reading `START_HERE.md` inside a generated pack, it is already prepared:
skip the generation commands and start with step 1.

Run commands from the repository root. First generate the ready-to-open files:

```bash
cargo run --locked -p josh-app --example prepare_demo -- --out-dir results/basic-demo
cargo build --locked --bin josh
```

The generator is Rust and uses the application's existing import, validation,
reproduction and analytical-demo services. Add `--offline` before `--` when Cargo
dependencies are cached. If the destination already exists, choose a fresh path,
such as `results/basic-demo-2`, and use that path in the commands below. Generation
never overwrites an existing destination. A completed pack has `demo-manifest.json`
with `status: ready`.

## 1. Open the graphics

```bash
./target/debug/josh tui --explanation results/basic-demo/charts/explanation.json
```

Use a terminal of about 140×45 cells to see the circular chart and signed scatter
side by side. Smaller terminals use tabs or a table with signed bars.

- Up/Down selects a feature and its original measurement.
- Tab changes views; the third view is the waterfall.
- t changes the explained class using cached distributions.
- g opens the searchable guide.
- s saves a **new** `.svg`, `.csv` or `.json` file.
- q exits.

Expect **ANALYTICAL DEMO / NO CANCER PREDICTION**, target NSCLC, raw score **0.405**,
baseline **0.120**, signed contributions totaling **+0.285**, 128 evaluations,
and an additivity residual near zero. The status remains `abstained`.
The seven labels include SBS4, SBS24, KRAS, MDM2, FAT1, CREBBP and age.

These charts use the application's fixed `INVENTED-01` analytical fixture. Its
seven measurements match the complete input example, but it has a distinct sample
ID and fixture provenance. Opening or editing a molecular input does **not** make
these demo charts a prediction for that input. Pressing d in a molecular screen
loads the fixed demo, rather than classifying the open sample.

## 2. Inspect clean molecular input

```bash
./target/debug/josh tui --molecular results/basic-demo/molecular/complete.json
./target/debug/josh molecular prepare results/basic-demo/molecular/complete.json
```

Expect sample `DEMO-MOL-01`, seven observed features and seven attribution groups:
two signature projection scores, one invented coordinate variant, three CNA calls
and age. Request preparation returns `sends_to_provider: false`. It sends nothing.
The KRAS coordinates are deliberately invented for format testing and are not an
annotated real variant. Signature values are not fitted COSMIC exposures.

To exercise the importer yourself:

```bash
./target/debug/josh molecular import results/basic-demo/inputs/molecular-complete.tsv \
  --input-format tsv --sample DEMO-MOL-01 --patient-group DEMO-PAT-01 \
  --source-id basic-demo-complete-v1 --assay invented-panel --synthetic \
  --output results/basic-demo/reimported-complete.json
```

The imported values have a real SHA-256 of the local TSV and original row numbers.
No patient names, credentials or inferred cancer labels are included.

## 3. Check zero, negative, unknown and not-tested values

```bash
./target/debug/josh tui --molecular results/basic-demo/molecular/missing.json
./target/debug/josh molecular prepare results/basic-demo/molecular/missing.json
```

Expect `DEMO-MOL-02`, six features and **three observed groups**:

| Observation | Expected representation |
| --- | --- |
| MDM2 CNA | Measured neutral call **0**, still observed |
| KIT IHC | Observed categorical **negative** |
| Age | **>89** years, preserving the bound |
| KRAS | Unknown, `value: null` |
| SBS4 | Not tested, `value: null` |
| Recorded sex | Unknown, `value: null` |

This is valid partial input. Missing values must stay missing; they must not become
neutral CNA calls or negative assay results. c and e in the TUI are live Jev
actions and are not required for this offline walkthrough.

## 4. Explore expression data

```bash
./target/debug/josh tui --dataset results/basic-demo/expression
./target/debug/josh dataset verify results/basic-demo/expression --reproduce
./target/debug/josh dataset explore results/basic-demo/expression \
  --sample DEMO-EXPR-01 --gene TP53
```

Expect two samples (`DEMO-EXPR-01` and `DEMO-EXPR-02`), six mapped genes per sample,
no blocked QC, and successful local reproduction. The raw TP53 value for the first
sample is **8.43 TPM**; the transformed value is log2(8.43 + 1). Use `[`/`]` to
switch samples and `/` to search genes in the workbench. No cancer reference is
attached, so reference compatibility remains unassessed.

## 5. Verify and export the explanation

```bash
./target/debug/josh molecular inspect results/basic-demo/charts/explanation.json --format text
./target/debug/josh molecular export results/basic-demo/charts/explanation.json \
  --kind svg --output results/basic-demo/my-chart.svg
./target/debug/josh molecular export results/basic-demo/charts/explanation.json \
  --kind csv --output results/basic-demo/my-contributions.csv
```

The generated `charts/explanation.svg` and `charts/contributions.csv` are already
available. New exports should show the same data. Repeating an export to the same
filename must fail while preserving that file.

## Files in the generated pack

| Path | Purpose |
| --- | --- |
| `inputs/` | Editable molecular TSVs, expression matrix, dictionary and dictionary provenance |
| `molecular/complete.json`, `molecular/missing.json` | Validated molecular inputs ready for Ratatui |
| `molecular/*-request.json` | Offline request previews with identifiers excluded from provider state |
| `expression/` | Reproducible expression dataset bundle |
| `expression-verification.json` | Result of local reproduction |
| `charts/explanation.json` | Complete analytical archive for offline replay |
| `charts/explanation.svg`, `charts/contributions.csv` | Viewable figure and numeric export |
| `demo-manifest.json` | Completion marker, counts, hashes and numerical expectations |

Generated packs live under ignored `results/`. Keep the source fixtures and Rust
generator to reproduce them. The demo tests software behavior, not cancer accuracy.
