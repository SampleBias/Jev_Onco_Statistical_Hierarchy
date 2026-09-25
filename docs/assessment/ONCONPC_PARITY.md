# JOSH compared with OncoNPC — current objective audit

Assessed 2026-09-25 against JOSH 0.8.0 and the
[published article](https://pmc.ncbi.nlm.nih.gov/articles/PMC11484892/) with its
[publisher correction](https://www.nature.com/articles/s41591-023-02693-x).
This replaces the September 24 pre-build assessment. The
[cohort delivery report](../reports/cohort-observatory.md) records what changed.
The [research guide](../references/ONCONPC_GUIDE.md) explains the paper relationship,
terminal figure and prioritized development direction.

**We do not meet all of the paper's objectives.** Much of the research workflow
is implemented. Cancer-specific predictive equivalence and clinical utility remain
unmeasured. No new real-data experiment or live Jev call was performed for this audit.

“Implemented” below means software exists, not that the scientific objective is
validated. “Partial” identifies missing workflow or methods. “Missing” means the
required analysis/evidence is not supplied by the current application.

## Objective-by-objective assessment

| Research objective | Current JOSH evidence | Verdict and what remains |
| --- | --- | --- |
| Genomic origin classification across 22 cancer types | [Molecular core](../../crates/josh-core/src/molecular.rs), [importer](../../crates/josh-ingest/src/molecular.rs), [Jev transport](../../crates/josh-jev/src/lib.rs): mutation, CNA, signature and demographic features; typed taxonomy; unresolved outcomes | **Partial.** Jev inference works, but assay/preprocessing equivalence and representative cancer accuracy are unmeasured. Live inputs remain synthetic-only. Experimental parents are not the corrected paper grouping. |
| Held-out classification and confidence/coverage assessment | [Evaluation](../../crates/josh-features/src/evaluation.rs), [study statistics](../../crates/josh-features/src/study.rs), F5: confusion, precision/recall, macro/weighted F1, top-1/top-3, threshold F1/coverage and patient-bootstrap intervals | **Tooling implemented; objective unvalidated.** No representative frozen test results or same-patient comparator benchmark. Reliability/Brier/log loss do not fit a calibration model. |
| Generalization across centers, panels and populations; excluded cancer types | Partition checks, explicit unknown/other outcomes, failure accounting and archived model/prompt/input provenance | **Partial.** No dedicated center/panel/time/ancestry stratification report, external validation, validated OOD detector or measured subgroup performance. Low model confidence is not proof of an unseen cancer type. |
| Feature robustness and feature selection | [Attribution engine](../../crates/josh-explain/src/lib.rs), grouped perturbations, feature QC and signature fitting | **Missing study.** No frozen feature-ablation sweep with held-out performance and uncertainty. Perturbing an input for explanation is not a population-level robustness experiment. |
| Individual explanations and cohort-level biological patterns | Native exact/permutation Shapley, stability comparison, linked ring/scatter/waterfall, raw values and exports | **Individual tooling implemented; cohort objective partial.** No aggregate per-class attribution/carrier-rate study or independently verified biological concordance. Jev explanations and the paper's TreeSHAP use different models/semantics. |
| CUP assignment and independent pathology agreement | Molecular archives and a separate Clinical review workflow | **Partial.** No representative CUP cohort, adjudicated pathology comparison, eligible-patient selection workflow or validated sample-to-clinical linkage. Merely switching screens does not link patients. |
| Orthogonal germline polygenic-risk validation | No PRS input/analysis workflow | **Missing.** Requires appropriate germline data, reference weights, ancestry-aware processing, enrichment/control tests and independent validation. Somatic explanation scores cannot substitute for PRS. |
| Prognostic stratification and clinical-outcome comparison | [Survival engine](../../crates/josh-features/src/survival.rs): delayed-entry Kaplan–Meier, censoring, risk counts, unweighted intervals/log-rank and F5 type curves | **Partial.** No real outcome validation, published CKP-vs-CUP prognostic comparison or prognostic somatic-feature analysis. Endpoint/selection review and independent numerical validation remain necessary. |
| Treatment-concordance associations with adjustment | Reviewed concordance inputs, delayed entry, adjusted Cox, supplied-propensity IPTW curves and balance/overlap/effective-N diagnostics | **Partial, not method parity.** No cross-fitted propensity estimation, weighted uncertainty/log-rank or PH diagnostics. Cox permits eight numeric covariates, insufficient for the full published adjustment after categorical expansion without redesign. No reproduced real cohort effect. |
| Actionable alterations and genomically guided treatment opportunities | Variant/CNA observations and source-linked clinical context | **Missing.** No OncoKB-style, versioned cancer-specific variant/fusion/therapy matching, evidence-level contract, actionability cohort analysis or measured increase in eligible therapies. |
| Reproducible research delivery | Rust, Ratatui, Jev API, frozen archives/hashes, separate labels, request budgets, offline replay, JSON/CSV/SVG/Markdown and searchable references | **Engineering delivered.** Imported source declarations/hashes do not authenticate predictions. Real-data permissions, representative cohorts and reproducible external comparisons are still needed. |

Source: published Results, Methods and Extended Data, with implementation checked
in the linked Rust modules. The native
[cohort contract and method limits](../data/COHORT_GUIDE.md) specify current estimator
assumptions and unavailable analyses.

## Differences that materially affect comparison

- The paper's published numbers belong to its trained XGBoost classifier.
  They cannot be transferred to Jev by matching labels, drawing its figures, or
  substituting a raw score threshold. No JOSH cancer-accuracy percentage is justified.
- The paper uses stabilized IPTW and cross-fitted propensities. JOSH currently
  consumes supplied scores and uses unstabilized ATE weights. A constant groupwise
  weight factor can cancel in a group's KM ratio, but that does not establish
  equivalent diagnostics, uncertainty, tests or causal interpretation.
- Current Cox fitting uses Breslow ties and model-based Wald intervals. Its
  eight-covariate limit, numeric encoding requirements and lack of PH checks need
  explicit reconciliation with a reviewed replication protocol.
- The paper's treatment evidence is retrospective and subject to confounding;
  it is not an individually validated drug-response predictor. JOSH does not
  presently supply such a predictor either.
- JOSH's 512-feature and 32-KiB request limits require a reviewed panel adapter.
  Complete the input/aggregation protocol before comparison; do not silently
  remove features. The bounded live cohort runner permits 1,000 cases per manifest;
  the offline study evaluator supports up to 100,000 records.
- Shapley sampling error excludes provider variability and background uncertainty.
  The [small historical live check](../reports/0.7.0-molecular-validation.md)
  verified API behavior on invented data; it did not estimate cancer performance.

## Acceptance gates and recommended order

1. **Eligible, frozen benchmark:** reviewed molecular representation and corrected
   class mapping; disjoint patient partitions; documented model/prompt versions;
   representative known-primary test data and frozen comparator outputs.
2. **Measured inference quality:** locked thresholds, F1/coverage and intervals,
   separate calibration fit, center/panel/population/OOD tests, repeatability and
   prospectively specified modality/feature ablations. Report failures explicitly.
3. **Independent CUP evidence:** adjudicated pathology agreement, aggregate
   explanation analysis and germline-risk validation with appropriate controls.
4. **Outcome and actionability evidence:** validated survival estimators, reviewed
   cohort selection/time origins and adjustment, missing weighting methods, and
   versioned therapeutic evidence. Evaluate external replication before inferring
   clinical utility.

All four gates remain scientifically open. The current change makes the requested
individual explanation figure discoverable and improves the reference/assessment
workflow; it does not claim completion of these research milestones.
