# Phase 04 — Evaluation, calibration and abstention

Status: planned; no labeled cancer cohort has been evaluated. Lead: ML/statistics lead. Reviewers: clinical lead and independent QA. Estimate: 10–20 person-days after data is ready. Depends on: Phase 02 locked cohort/splits and Phase 03 frozen Jev contract. Controls: all scientific probability claims and advancement to pilot.

## Outcome

Determine whether Jev adds useful information about origin for the project's target cases, quantify uncertainty, and select a measured abstention policy. A valid outcome can be that Jev is unsuitable for origin prediction from the chosen inputs.

## Study design

Use patient-level development, calibration and final test partitions. Keep institutions/time periods/panels separated for external or shift testing where data allows. Prompts, evidence packing, taxonomy and thresholds are tuned only on development/calibration data as specified in a prewritten protocol. The final test partition stays sealed until those choices are frozen.

Known-primary metastatic tumors with origin information masked are an initial proxy task. They are not equivalent to real CUP. True CUP evaluation needs an independently adjudicated outcome when available, or explicitly reports concordance/coverage without pretending unresolved cases have known labels. Multiple samples from one patient stay in one partition.

## Work packages

| ID | Owner | Task and concrete output |
| --- | --- | --- |
| P04-01 | ML + clinical | Freeze protocol, class mapping, label adjudication, exclusions and measurable release criteria before test access |
| P04-02 | Backend/ML | Create `josh-eval`: batched case runs with immutable request/response artifacts, resume support and all failures in the denominator |
| P04-03 | ML | Compute top-1/top-3 accuracy, macro/weighted F1, per-class recall/precision, confusion matrix, coverage and selective error |
| P04-04 | Statistician | Evaluate raw probability reliability with multiclass Brier score, log loss, classwise reliability plots, ECE with declared bins, and patient-bootstrap confidence intervals |
| P04-05 | ML | Compare Jev against majority-class and empirical class-prior references computed from the development partition; freeze these references before final testing |
| P04-06 | ML + clinical | Perform modality ablations, missingness/negation tests, institution/panel shifts, rare/out-of-taxonomy cases and incorrect/injected report text |
| P04-07 | Statistician | If discrimination warrants it, fit a regularized calibration mapping on a dedicated calibration partition and compare raw/calibrated test results |
| P04-08 | ML | Select abstention thresholds using calibration/development data; report coverage-versus-risk and sensitivity for each important subgroup |
| P04-09 | Clinical + QA | Review blinded failure cases, including confident errors and no-match handling; document unsupported populations and evidence types |
| P04-10 | Technical lead | Package results, model card and a go/narrow/no-go recommendation; approve only evidence-supported product wording |

## Metrics and interpretation

Define metrics for both biological classes and operational abstention. `insufficient_evidence` is not a biological cancer label. Preserve its mass for audit; specify exactly how any cancer-only evaluation conditions on assignment. Do not remove no-match mass and silently renormalize it into a patient's reported cancer probabilities.

Report metrics on all eligible inputs and separately on accepted predictions, with coverage and failure rates beside accuracy. Low coverage can create superficially high accepted-case accuracy. Include per-class sample counts and uncertainty intervals; do not advertise a class with too little evidence. Demographic slice analysis must avoid exposing small groups or implying absent data has been validated.

Calibration candidates may include temperature scaling of logged probabilities or a regularized multinomial mapping; select a method compatible with sample size and zeros. Numerical clipping for log-loss must be documented and must not alter stored/displayed raw scores. Calibration cannot create discrimination and may fail under institutional or modality shift.

## Artifact contract

A calibration artifact binds provider/model, taxonomy order and version, prompt version/hash, preprocessing/evidence version, eligible modalities, development/calibration cohort manifests, fitted parameters, method and acceptance report. Runtime loading must reject incompatible artifacts. Keep raw and calibrated values separately. A calibration artifact is downstream software; it does not alter Jev weights.

## Acceptance criteria

- A protocol with clinical/statistical owners specifies numerical minimums for class performance, selective risk, coverage, calibration and subgroup uncertainty **before** the final test. These values are intentionally not invented by this plan.
- Splits are patient-disjoint and label leakage is audited, including indirect diagnostic statements in reports.
- Reports include full and abstained cases, errors, confidence intervals, class counts and external/shift evaluation where claimed.
- Proposed calibration improves or justifiably preserves held-out performance, with compatibility checks proven in tests.
- No production field is labeled a calibrated cancer probability until the relevant artifact and evidence are approved.
- The report states whether and for which modalities Jev is fit for a research pilot; negative results narrow or stop the classification claim.

## Handoff and risks

Deliver frozen manifests, evaluation code, reproducible metric tables/plots, raw prediction archive, failure analysis, optional calibration artifact, and model card. Main risks are proxy-cohort optimism, prompt leakage, small rare-class counts, and provider/prompt drift. A new provider version must be evaluated before replacing the pinned one.
