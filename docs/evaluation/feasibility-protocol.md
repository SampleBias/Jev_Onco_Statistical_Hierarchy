# Jev feasibility protocol — draft 0.1

Owner: ML/statistics lead with clinical reviewer. Status: study-design draft; no cohort, live measurement, calibration fit or clinical acceptance has occurred. Purpose: determine whether Jev can use the intended evidence to rank primary origins, before committing to a large application rollout.

## Experiment tracks

Run separate genomic-only, histology/IHC/clinical-only and combined tracks. Record available modalities, test coverage and missingness per case. A combined-track improvement cannot establish genomic-only performance. Freeze eligible population and the reviewed taxonomy before mapping labels; the current demo taxonomy is insufficient for a medical benchmark.

Use known-primary cases with independent reference labels hidden during inference. Record when each observation was available and censor later diagnostic conclusions. True CUP records may be reviewed descriptively; an unconfirmed model assignment is not a ground-truth label.

## Development and locked evaluation

1. Curate a small authorized development sample covering each eligible class and the no-match/missing/conflict cases. Determine feasible counts after the data inventory; no cohort size is presently known.
2. Group splits by patient and account for institution, assay and time. Keep all related records together. Save the manifest and checksum before any model run.
3. Use development data to revise deterministic preprocessing, taxonomy, prompts and tentative gates. Keep an independent calibration partition for Phase 04; do not fit calibration on the locked test set.
4. Have the statistician set per-class sample size and precision targets, and freeze the model/prompt/taxonomy/preprocessing/policy versions, metric definitions and acceptance criteria before opening test labels.
5. Execute the frozen run once, record failures, timeouts, usage and exclusions, then publish all track results. A further prompt change requires a new experiment and a new held-out evaluation strategy.

## Measures and draft gates

Engineering gates: all accepted responses pass exact model/label/question checks, probabilities sum to one within 1e-6, input budgets are enforced, and mock/replay results cannot pass as authenticated provider output. Any failure blocks the corresponding engineering release.

Scientific reports include top-1 accuracy, top-3 coverage, macro/per-class recall, log loss and multiclass Brier score, with confidence intervals and denominators. Report selective accuracy versus abstention coverage, unresolved/out-of-vocabulary outcomes, missingness subgroups and failure rates. Specify clipping only for metric computation when zero probabilities occur; never silently alter the exported distribution.

Draft feasibility criterion: the lower bound of a prespecified 95% confidence interval for paired top-1 improvement over a development-estimated class-frequency reference should exceed zero. This is a screening proposal, not a medical deployment threshold. The statistician must choose the interval method and power/sample size, and the clinical lead must set acceptable per-class error and review coverage targets before test access. These items are still open; no clinical quantitative gate is claimed complete.

Calibration must be fitted on separate data, saved as a version-bound artifact, and assessed on held-out data. Until that evidence exists, the UI exposes raw Jev scores and null calibrated probabilities. Reliability of Choice distributions and provider confidence must be measured separately.

## Synthetic provider smoke record

Status: pending credentials/access. Local contract tests do not establish live provider compatibility. Phase 03 records model returned, answer types, complete option distribution, usage, latency, request hash and sanitized failure category for one approved synthetic request. Never commit a key or an unreviewed provider body. External real-data requests additionally require the data/provider decision record.

## Exit decision

Proceed to a larger study only after independent review of the measured results and data permissions. If a track lacks useful signal, revise its evidence representation using development data, narrow the product scope or stop that track. Building a larger UI cannot resolve a negative scientific result.
