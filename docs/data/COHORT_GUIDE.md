# Cohort Observatory — offline study analysis

## Open the paper-inspired workspace

Start `./scripts/start`, press **F5**, then **d** for the invented demonstration:
528 classification records across 22 cancer classes and 180 independent synthetic
outcome records. These are not Jev predictions or OncoNPC results. No network,
API key, Python, model download or external patient data is needed.

A true-color terminal with a Unicode font at around 190×64 cells gives the
three-panel overview room to breathe; smaller terminals have focused views.
Ratatui draws heatmaps, Braille curves, censor markers, risk tables and calibration
plots directly. Nothing is a screenshot of the paper.

| Key in F5 | Action |
| --- | --- |
| 1–6 / view selector | Overview, Confusion, Survival, Treatment, Calibration, Protocol |
| Arrows | Inspect a confusion cell; scroll textual views |
| n | Cycle counts, row-normalized recall, column-normalized precision |
| b | Detailed / explicitly mapped broad classification groups, if supplied |
| i | Pointwise 95% survival bounds on unweighted curves |
| w | Unadjusted / supplied-propensity IPTW treatment curves |
| [ / ] | Browse predicted-type curves, six groups per page |
| l / o | Shared loader: study input or exported study archive |
| s | Export to a new .svg, .md or .json path |
| x | Discard pending calculation when it finishes; keep previous study |
| F1 / g | Searchable offline guide |

Switching workspaces preserves the study and lets calculations finish. Failed loads
preserve the previous study. d explicitly replaces it with the demo after calculation.
Export before replacing a study or quitting; session state is not auto-saved.
Export errors remain visible and existing files are protected.

## CLI and reproducible exports

After building, substitute `./target/debug/josh` for `josh` if not on PATH.
Choose unused destination paths; their parent directories must exist.

```bash
josh study demo --output results/cohort-demo.json
josh tui results/cohort-demo.json
josh study analyze results/cohort-demo.json --bootstrap 1000 --seed 42 --format text
josh study export results/cohort-demo.json --kind svg --output results/cohort-figure.svg
josh study export results/cohort-demo.json --kind markdown --output results/cohort-report.md
josh study export results/cohort-demo.json --kind json --output results/cohort-archive.json
josh schema cohort-study
```

SVG is a fixed overview, not a capture of UI toggles: row-normalized detailed
confusion, predicted-type survival, **unadjusted** treatment curves with 95%
intervals, confidence/coverage and risk counts. Markdown/JSON include Cox and IPTW
diagnostics. JSON contains the complete study/report; reopening verifies the input
hash and recalculates results. Source declarations/hashes do not authenticate
imported predictions. Reopening with different bootstrap settings changes CIs.
The TUI uses 200 bootstrap replicates and seed 42; CLI allows 100–1,000 or zero
to disable. Other values make the interval explicitly unavailable.

## Import frozen predictions

Wrap the existing molecular runner's evaluation records in a study, without
inference or truth-label transmission (replace the illustrative input path):

```bash
josh study from-records results/run/evaluation-records.json \
  --study-id pilot-001 --protocol genomic-only-v1 \
  --prediction-source archived-evaluation --model jev-1.13.0 \
  --partition test --output results/pilot-study.json
```

Use `--synthetic` only for invented data; otherwise the wrapper declares
`deidentified_research`. Custom labels require `--taxonomy TAXONOMY.json`.
This never calls Jev or changes the synthetic-only live-data policy.

`contracts/cohort-study.schema.json` documents the structure. Runtime validation
also checks probability sums, labels, unique sample IDs and patient-level separation
of development/calibration/test partitions. Limits: 32 cancer classes, 100,000
prediction records. Success requires a complete probability vector including
unknown/other; failure requires an explicit failure and no probabilities.
Failures remain in accuracy/coverage denominators; scores are not renormalized.

`broad_groups` defaults to empty. A paper grouping needs a complete leaf-to-group
map and `broad_group_version`; sum group probabilities **before** choosing a winner.
Experimental parents are not automatically the preprint's ten-group or corrected
publication's thirteen-group map. Only the matrix switches to broad groups;
metrics and thresholds remain detailed-class analysis. Truth labels must stay
outside model inputs.

## Outcomes and statistical methods

`outcomes` is optional and separate from classification records: known-primary
evaluation and CUP outcomes can describe different populations. No automatic join
to Clinical cases/inference archives occurs. Document population and provenance,
and review treatment concordance externally.

Outcomes declare `endpoint`, `time_origin`, `time_unit` (days/months/years),
`source`, `adjustment_covariates`, `propensity_model` and `observations`.
Each observation has one unique patient, `predicted_class`, nonnegative `entry`,
`time > entry`, `event` (true for endpoint, false for right censoring),
`concordance` (concordant/discordant/empiric/unreviewed), optional `propensity` and
numeric `covariates`. Entry 3, time 12 means entry at 3 and exit at 12 from the
**same declared origin**, not 12 after entry.

- Kaplan–Meier uses `(entry, exit]` risk intervals, tied exits sharing a risk set,
  and pointwise 95% Greenwood log-log intervals. Survival 0/1 has degenerate bounds;
  median not reached is unavailable, not zero. No extrapolation past follow-up.
  Risk tables count patients before exits; t=0 includes baseline entrants.
  Limits: 20,000 patients / 32 groups.
- Global predicted-type and two-group concordance log-rank tests use unweighted
  hypergeometric tied-event covariance. Singular/no-event comparisons are unavailable.
  Limit: 2,000 patients; curves remain available above that limit.
- Cox uses concordant versus discordant exposure plus up to eight declared numeric
  baseline covariates, Breslow ties, centered/scaled fitting and model-based 95%
  Wald intervals in original units. Missing covariates, singularity, separation,
  unstable coefficients or nonconvergence make the fit unavailable. No automatic
  imputation, regularization, categorical encoding or PH test. Limit: 2,000 included
  patients; larger cohorts need an external inference workflow.
- IPTW requires supplied `P(concordant | baseline covariates)` and documented
  `propensity_model`. Unstabilized ATE weights are 1/p or 1/(1−p). Scores outside
  [0.01,0.99] disable weighting; no silent clipping/trimming. Diagnostics include
  effective N, weight extremes and pre/post-weighting SMDs using a fixed unweighted
  pooled SD. Missing covariates have unavailable balance. **Weighted CIs and weighted
  log-rank are not implemented.** Unweighted p-values cannot substitute for them.
  Weighted plots retain raw patient risk counts.
- Empiric/unreviewed records remain in type curves but are excluded from treatment
  comparisons. Endpoint, time origin, selection, independent censoring and exposure
  timing require expert review to avoid bias.
- Classification CIs use a patient-cluster percentile bootstrap: repeated samples
  travel together, classes are fixed, and failed predictions remain. More than
  10 million record×replicate operations makes the interval unavailable. Thresholds
  select biological winners; coverage divides by **all eligible records**.
  Reliability/Brier/log loss describe scores; they do not fit calibration.

These are research analyses, not clinical validation or proof of treatment benefit.
Tests use asymptotic approximations; sparse groups need review. No competing risks,
time-varying exposure, robust clustered Cox uncertainty, propensity fitting, weighted
inference, germline risk or external OncoNPC model is included. Treatment labels are
reviewed inputs, not automatically adjudicated from medication records.

See [Sources](../SOURCES.md), the [current objective assessment](../assessment/ONCONPC_PARITY.md)
and [implementation report](../reports/cohort-observatory.md). Freeze data eligibility,
preprocessing, labels, thresholds and validation splits before a representative
benchmark; do not tune on the held-out test set.
