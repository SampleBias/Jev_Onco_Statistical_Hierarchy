# Cohort Observatory implementation — September 24, 2026

This is an engineering report, not a cancer benchmark. The earlier
[OncoNPC assessment](../assessment/ONCONPC_PARITY.md) is retained as the pre-build
baseline. Jev's predictive equivalence remains unmeasured.

## Delivered

| Paper-inspired capability | Implementation |
| --- | --- |
| Classification panel A | Native Ratatui confusion heatmap, recall strip, cell inspector, counts/row/column normalization; retained unknown, other and failed outcomes |
| Predicted-type survival panel B | Native Braille Kaplan–Meier curves, censor marks, optional pointwise 95% bounds, number-at-risk tables and global log-rank |
| Treatment comparison panel C | Unadjusted and supplied-propensity IPTW curves; adjusted Cox association with confidence intervals; overlap, effective N and balance diagnostics |
| Cohort evaluation | Per-class/weighted/macro F1, top-1/top-3, patient-cluster bootstrap, threshold-specific F1/coverage and reliability diagnostics |
| Reproducible workflow | F5 section, shared loader, offline study CLI, versioned study contract, hashed input archive, SVG/Markdown/JSON exports and searchable guide |

All plots are calculated from the selected study. The demo has 528 invented
classification records and 180 invented outcome records; it is marked synthetic
throughout. No paper patient data, pre-trained cancer model, model download or live
Jev API call was used. Existing molecular inference, Data and Clinical workflows
remain separate. The banner change retains the two status lights without DNA/JOSH
decoration.

## Verification

- `cargo fmt --all -- --check` and workspace Clippy with warnings denied.
- Full `cargo test --workspace --locked --offline`, including pre-existing
  HTTP transport tests. Those tests need permission to bind localhost outside
  the sandbox; they use mock servers, not the live provider.
- New numerical tests cover hand-calculated Kaplan–Meier and Greenwood bounds,
  tied events/censoring, delayed entry, all-censored cohorts, hypergeometric
  log-rank variance, two/multi-group consistency and a closed-form Cox HR/information.
- Multivariable Cox with delayed entry and ties is also checked against an
  independent brute-force partial likelihood and numerical first/second derivatives,
  including model-based standard errors and permutation invariance.
- Failure checks cover separation, collinearity, duplicate patients, missing
  covariates, invalid propensity provenance/overlap, data leakage, unsupported labels,
  all failed predictions and unavailable estimates.
- Application checks cover deterministic analysis, probability mass preservation,
  all six views from 190×64 down to 1×1, archive tampering, shared navigation/hidden
  job completion, generated-schema validation, no-key CLI roundtrips and overwrite
  protection. SVG escaping and rendering were inspected with actual Ratatui buffers.

These are software checks, not external validation of the entire inference engine
against an independent statistical package on representative cohorts. Methods,
sample limits, confidence procedures and explicit unavailable cases are documented
in the [Cohort Guide](../data/COHORT_GUIDE.md).

## Remaining scientific and operational work

1. Freeze an approved genomic-only Jev protocol, preprocessing/assay adapter,
   taxonomy, corrected broad grouping, model/prompt version and patient partitions.
   Imported frozen predictions declare their source; the hash does not authenticate
   model provenance. No real-data live-call authorization was added.
2. Obtain an eligible representative labeled cohort and compare frozen predictions
   against OncoNPC/an appropriate baseline on the same held-out cases. Measure
   institution/panel/time shift, abstention, errors and repeated-call variability.
   Fit calibration only on separate calibration data.
3. Independently validate statistical estimates and clinically review outcomes,
   time origins, selection, censoring, exposure timing and concordance adjudication.
   The study accepts reviewed labels rather than deriving them from treatment records.
4. Add reviewed propensity fitting and weighted uncertainty/testing if needed.
   Current IPTW curves are descriptive; **weighted confidence intervals and weighted
   log-rank are unavailable**. Cox uses Breslow/model-based Wald inference, not robust
   weighted or clustered inference; no automatic PH test, competing risks,
   time-varying exposure or germline-risk validation is included.

The interface now supports much of the requested study workflow, but attractive
plots and working estimators do not show that Jev matches the paper's predictive
performance or improves treatment outcomes.
