# Ten synthetic CUP workups

Collection `synthetic-cup-v2`, prepared 2026-09-26. Open **Open data → Samples**
in the TUI. These are ten distinct invented patients, with 227 feature rows in
total. Loading uses the normal TSV importer and makes no provider request.
No patient record, published patient example, or model prediction was copied.

These cases model the information available during an unresolved metastatic
workup. Several suggest a lineage without proving an anatomical primary. Case
002 has an incomplete investigation; case 003 awaits more tissue. They span
provisional and unresolved CUP presentations, not ten confirmed diagnoses.

## Coverage

| Case | Presentation and evidence | What to inspect |
| --- | --- | --- |
| 001 | Nodal adenocarcinoma; KRAS/TP53/STK11 counts, TTF-1/Napsin A | Pulmonary pattern despite no localized primary; EGFR count zero is scoped to tested exons |
| 002 | Hepatic adenocarcinoma; APC/KRAS/TP53 counts, SATB2/CDX2/CK20 | Intestinal pattern; incomplete colonoscopy and incomplete MMR panel |
| 003 | Scant acid-decalcified bone core; weak keratins, tumor-only TP53 | Failed controls, low tumor fraction and uncertainty; no confident origin from TP53 alone |
| 004 | Axillary nodes; ER/PR/GATA3/mammaglobin, PIK3CA count | Breast pattern with unrevealing imaging; shared GATA3 needs context |
| 005 | Peritoneal high-grade carcinoma; PAX8/WT1/p53 | Gynecologic differentiation does not prove ovarian rather than tubal/peritoneal/uterine location |
| 006 | Nodes and sclerotic bone lesions; NKX3.1/PSAP, weak PSA | Prostate pattern; focal weak staining is not the same as a missing result |
| 007 | Non-decalcified soft-tissue metastasis; clear cells, PAX8/CAIX/CD10 | Renal pattern; related VHL/PBRM1 counts and losses share an attribution group |
| 008 | Mucinous hepatic adenocarcinoma; CK7/CK19, KRAS/TP53/SMAD4 | Pancreatic/biliary overlap; liver biopsy alone cannot establish CHOL |
| 009 | Two specimens with competing intestinal and pulmonary panels | Flag conflict; possible distinct primaries, no assumed common clonality |
| 010 | Trabecular nodal carcinoma; Arginase-1/HepPar-1/canalicular CEA | Hepatocellular differentiation falls outside the frozen taxonomy; inspect other_origin |

These are design expectations, not ground truth. `scenarios.json` is a separate
local review manifest and is never loaded into the model's evidence. Case 005
deliberately has multiple plausible assignment outcomes. Cases 003, 008, 009 and
010 exercise insufficiency, overlap, conflict and taxonomy coverage respectively.
Do not demand predetermined numerical scores or call scenario agreement accuracy.

## Data representation

- TSVs contain the app's processed feature schema, not raw sequencing data or
  complete panel inventories. Counts are explicit reported nonsynonymous variant
  counts per named gene, with tested scope, depth and detection limits. No HGVS
  allele, pathogenicity annotation, germline exclusion or VAF is inferred from a
  count. The low-quality case explicitly has unresolved tumor-only findings.
- CNA calls use the existing ordinal -2/-1/0/+1/+2 encoding. Measured zero is
  distinct from a failed measurement. Tumor fractions, staining percentages,
  coverage and all molecular findings are invented, biologically motivated values;
  they are not fitted population distributions or output from actual assays.
- `unknown` means uninterpretable/unavailable, `not_tested` means not performed;
  both carry no value. Low coverage never becomes a negative call. Unlisted genes
  were not supplied and cannot be assumed wild type.
- RNA and SBS assessment are explicitly not performed. A handful of panel
  mutations does not justify fabricating fitted signatures, expression vectors,
  HRD, MSI, or treatment-response estimates. Partial MMR staining in 002 does not
  establish global MMR status. BRCA counts in 005 do not establish HRD status.
- Specimen, workup and quality context are bounded categorical rows in the existing
  histology modality. This preserves the application's schema; these contextual
  rows are not additional microscopy measurements. Sample 009 names specimens
  in the stain names/coverage and supplies sequencing for A only.
- Correlated IHC panels are grouped. PTEN mutation/CNA and the VHL/PBRM1 cluster
  are also grouped. These are documented attribution choices, not a claim to
  capture every biological dependence or to produce causal effects.
- Source byte hashes and row numbers are recorded by the importer. Neutral sample
  identifiers, patient groups, source paths and hashes remain local.

## Export and compare without a key

```bash
cargo run --locked -p josh-app --example prepare_cup_samples -- results/cup-realistic-v2-v5
```

Use a new directory. It exports all ten full inputs and paired genomic-only and
pathology/context-only views: 30 input files and 60 frozen v3/v5 request previews.
Open the **input** JSON, not the `*-request.json` preview, in the app.
`inventory.json` records hashes, byte sizes and the scope. Views share patient IDs;
they are not 30 independent patients and must never cross evaluation partitions.

Five coherent in-taxonomy scenario labels (001, 002, 004, 006, 007) also receive
development-only manifests for the existing comparison command. Their statistics
are pipeline smoke tests, not evidence of medical discrimination or calibration.
All ten cases retain requests for qualitative review, including the five excluded
from those metrics. Compare full versus genomic versus pathology views without
assuming more evidence always raises confidence.

The former five picker entries are preserved under `fixtures/synthetic-jev/` and
`samples::load_legacy`, for reproducing the historical v2/v3 preparation example.
They are not additional patients in the current picker.

## Reference basis

The [OncoNPC study](https://www.nature.com/articles/s41591-023-02482-6), its
[correction](https://www.nature.com/articles/s41591-023-02693-x) and the authors'
[implementation](https://github.com/itmoon7/onconpc) guide the genomic feature
families, frozen taxonomy and separation of evaluation labels. OncoNPC is a
trained XGBoost classifier; this suite does not reproduce its trained weights,
full feature matrix, signature preprocessing, cohort frequencies or performance.

The [SEOM–GECOD diagnostic guideline, Table 3](https://pmc.ncbi.nlm.nih.gov/articles/PMC8986666/)
guides lineage panels and their overlap; its workup discussion guides the bounded
clinical context. This 2021 guideline is used as a documented source of diagnostic
patterns, not as current treatment guidance. The molecular and IHC combinations
here are author-constructed scenarios requiring pathology review before use as
an evaluation benchmark. No therapies, survival outcomes or benefits are simulated.

See [how Jev is used](../../docs/references/JEV_WORKFLOW.md) for the request design,
evaluation procedure and limitations.
