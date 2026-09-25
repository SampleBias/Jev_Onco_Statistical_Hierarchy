# Synthetic samples for live Jev testing

Five loadable inputs representing **three invented patients**, for testing the
actual import → Jev → results → optional explanation workflow. No patient data,
predictions, probabilities or attributions are bundled. All measurements and
laboratory descriptions are invented. Not for diagnosis or treatment decisions.

## Load and run

**Simplest route:** launch `./scripts/start`, click **Samples**, select an input,
then click **Open selected**. The five inputs are now bundled into the Rust app;
no paths, generated JSON or import form are required. Keyboard equivalent:
press `b`, use arrows to select, then Enter.

The prepared JSON files in this checkout are under `results/jev-synthetic/`.
From the repository, in the terminal where you exported `TYPESAFE_API_KEY`:

```bash
./scripts/start tui results/jev-synthetic/sample-001.json
```

1. In Analysis (F2), confirm `SYNTH-001` and **20 observations** appear. Alternatively,
   press `l` in an already-running app and open that JSON file.
2. Press `a`, review the dialog and choose **Send 1 request** for a **new run directory**.
   This sends one potentially billed Jev request. For example, use
   `results/jev-synthetic/run-001` if it does not already exist.
3. Press `v` to inspect rankings, evidence checks and abstention reasons. A
   successful inference saves the run and `report.md` automatically.
4. Optionally press `e`, review the additional-call budget, choose **Start explanation**, and wait
   for the explanation to complete. Then press `p` for the linked OncoNPC-inspired
   ring/scatter. Ring, Scatter and Waterfall remain available too.
5. Press `l` to load another sample. Keep each inference in its own new directory.

**Do not press `d` for these samples:** it replaces your loaded input with the
separate, fixed analytical demo. Open `sample-001.json`, not
`sample-001-request.json`; the latter is an offline request preview, not an input.

The app remains pinned to `jev-1.13.0`. Preparing this pack makes **no provider
calls**. Key presence does not prove authentication. A `.env` file alone is not
loaded by the app; launch from the shell where the key was exported.

## What each file tests

Names below are file basenames under `results/jev-synthetic/`. Scenario descriptions
are design notes, not verified ground truth or acceptance targets for a model score.

| Input | Observed / unavailable | Intended exercise |
| --- | --- | --- |
| `sample-001.json` | 18 / 2 | Lung-like pattern: KRAS G12C, TP53 R273H, SBS4-rich synthetic exposures, CNA, TTF-1/Napsin A and morphology |
| `sample-001-genomic.json` | 10 / 1 | Same sample, retaining genomics and demographics but removing all IHC and morphology |
| `sample-002.json` | 13 / 3 | Colorectal-like pattern: BRAF V600E, TP53 R273H, CNA, SATB2/CDX2/CK20 and morphology |
| `sample-002-genomic.json` | 6 / 2 | Same sample without IHC or morphology; tests how much the missing pathology changes the result |
| `sample-003.json` | 6 / 8 | Poorly differentiated carcinoma with weak, incomplete evidence; inspect uncertainty and abstention behavior |

The genomic controls retain the same sample and patient-group IDs as their full
counterparts. They are paired evidence comparisons, **not independent patients**;
never split a pair between training and test data. The full inputs include pathology
extensions and are not reproductions of the genomic-only OncoNPC feature pipeline.

For the first two cases, compare the full and genomic-only rankings and evidence
checks; do not require a particular confidence value. For the third, an unexplained
confident classification is worth investigating, but abstention is not guaranteed.
Three constructed cases cannot establish accuracy, calibration, treatment-response
prediction or clinical usefulness. No treatment or outcome labels are supplied.

## Realism and its limits

- These are **processed feature-level fixtures**, not FASTQ, VCF, complete genomes
  or outputs of actual laboratory assays. Assay names explicitly say `synthetic`.
  They are curated marker subsets, not whole-genome variant inventories.
- Variant alleles use verified GRCh38 coordinates. Somatic status, read counts,
  coverage, ages, sex, staining and CNA calls are invented. Coverage text describes
  simulated evidence; it is not a typed VAF measurement or proof of assay QC.
  Wording about reviewed CNA calls also describes a simulated process, not an
  actual laboratory review.
- Discrete CNA values are calls, not absolute copy counts: `-2` denotes deep loss,
  `0` neutral, and `+1` gain in these scenarios. An observed neutral call remains
  distinct from an unknown result.
- Unavailable or unperformed tests use `unknown` or `not_tested` with a null value.
  They are never converted to a negative result or a measured zero.
- Sample 001 has invented SBS1/SBS4/SBS5 fractions of 0.14/0.62/0.24, summing to
  one, with a hypothetical 3,200-SNV whole-genome context. No spectrum was generated,
  no catalogue was downloaded and no signature fitting was performed. The
  `synthetic-exposures-not-fitted` method records that limitation explicitly.
- Sample 001 groups the two reported mutations and three signature exposures as
  `sequence-profile`, with declared dependencies. Explanations perturb them jointly
  instead of claiming independent per-gene/per-signature effects. Demographics and
  selected related IHC findings are grouped too; attribution results depend on this
  grouping and the app's masking baseline. They are not causal explanations.
- Input filenames and identifiers are neutral. Expected origins appear only in this
  README, not in feature values or provider state. The importer records the actual
  TSV SHA-256 and source line for each feature. Local source IDs, hashes and patient
  IDs are excluded from provider state; feature descriptions and assay metadata
  are included. Synthetic wording is retained, so this is not a blinded evaluation.

An inference alone does not produce measured feature attributions. The optional
`e` action makes additional live calls. The current guided configuration uses 16
forward/reverse permutation pairs, capped at 512 evaluations, 1,000,000 estimated
input tokens and 600 seconds. Review its budget dialog; caching can reduce calls,
but completion within these limits is not guaranteed. No explanation calls have
been made for this pack.

## Rebuild the JSON and request previews offline

The TSVs here are the reproducible source. `results/` is ignored by Git, so use
these commands after a fresh checkout. Choose a **new** output directory; existing
files are protected. The block uses Bash and needs no API key or provider access
after building the application.

```bash
cargo build --locked --bin josh
(
  set -euo pipefail
  pack_dir=results/jev-synthetic-rebuilt
  mkdir -p results
  mkdir "$pack_dir"
  for name in sample-001 sample-001-genomic sample-002 sample-002-genomic sample-003; do
    case "$name" in
      sample-001*) number=001 ;;
      sample-002*) number=002 ;;
      sample-003) number=003 ;;
    esac
    ./target/debug/josh molecular import "fixtures/synthetic-jev/$name.tsv" \
      --input-format tsv --sample "SYNTH-$number" \
      --patient-group "SYNTH-P$number" \
      --source-id "synthetic-jev-v1-$name" --assay synthetic-assay-v1 \
      --synthetic --output "$pack_dir/$name.json"
    ./target/debug/josh molecular prepare "$pack_dir/$name.json" \
      --output "$pack_dir/$name-request.json"
  done
)
```

Do not add a global reference-build override: relevant genomic rows already specify
GRCh38, whereas IHC and demographics correctly have no genome build.

Validated at creation: all five imports and offline request preparations succeed;
missingness, source hashes, dependency references, paired subsets and signature
sum were checked. Compact request sizes are 7,844 / 5,582 / 6,325 / 4,323 / 5,440
bytes in the table's order, below the app's 32,768-byte limit. All five inputs were
also opened in the actual terminal UI without sending a request. These checks
establish format/workflow compatibility, not prediction validity.

## Sources used for plausibility

These sources support marker/annotation choices, not the invented measurements or
any prediction for these files:

- ClinVar genomic annotations: [KRAS G12C](https://www.ncbi.nlm.nih.gov/clinvar/variation/12578/),
  [TP53 R273H](https://www.ncbi.nlm.nih.gov/clinvar/RCV000115738/) and
  [BRAF V600E](https://www.ncbi.nlm.nih.gov/clinvar/variation/13961/).
  Germline classifications in annotation records are not evidence for the simulated
  somatic status of these cases.
- COSMIC's [SBS4 description](https://cancer.sanger.ac.uk/signatures/sbs/sbs4/)
  supports its tobacco-associated context, not the chosen synthetic exposure.
- IASLC's [diagnostic immunohistochemistry recommendations](https://education.iaslc.org/AssetListing/Special-Article-Best-Practices-Recommendations-for-Diagnostic-Immunohistochemistry-in-Lung-Cancer-3316/PIIS1556086418335147-22263)
  inform the lung-like IHC panel; markers require interpretation in context.
- A [primary SATB2/CDX2/CK20 study](https://pubmed.ncbi.nlm.nih.gov/36819801/)
  informs the colorectal-like panel; it does not make that panel a guaranteed origin label.

See the [OncoNPC research guide](../../docs/references/ONCONPC_GUIDE.md) for the
application's research direction and remaining evidence gaps.
