# JOSH compared with OncoNPC

Assessed September 24, 2026 against the current 0.8.0 source and its recorded
validation. This is a code and evidence review, not a new cancer benchmark.
The requested reference is the [2022 version-1 preprint](https://www.medrxiv.org/content/10.1101/2022.12.22.22283696v1.full).
The [source register](../SOURCES.md) also records the published study and correction.

## Implementation update — September 24, 2026

The assessment below is the **pre-build baseline**, retained as a dated comparison.
Cohort Observatory now adds the three terminal plot types, patient-bootstrap
classification intervals, threshold F1, a survival contract, Kaplan–Meier/log-rank,
adjusted Cox and supplied-propensity IPTW diagnostics. See the
[implementation report](../reports/cohort-observatory.md) and
[Cohort Guide](../data/COHORT_GUIDE.md) for scope and limitations.
The baseline 3/10 cohort engineering rating is no longer a current-state rating.
Predictive equivalence remains **unmeasured**; no representative cancer benchmark
or clinical validation was performed by implementing these tools.

## Baseline assessment

JOSH can execute a molecular inference workflow through Jev and explain its
outputs. Equivalence to OncoNPC's predictive performance has not been measured.
Rust and Ratatui provide the application and graphics; changing the classifier
requires an independent assessment on known-origin tumors.

These ratings are engineering judgments, not accuracy estimates or percentages
of a measured project plan:

| Dimension | Rating | Basis |
| --- | --- | --- |
| Research application workflow | 7/10 | Import, validation, inference, archives, explanations, terminal charts and exports work; representative assay/cohort integration remains incomplete. |
| Cohort study functionality | 3/10 | Frozen-prediction scoring and a bounded runner exist; the screenshot's plots, survival data model and outcome analyses are missing. |
| Demonstrated predictive equivalence | Unmeasured | No real cancer benchmark is recorded; a numerical cancer-accuracy rating would be invented. |

The preprint reports weighted F1 of 0.784 on 7,289 held-out tumors. At a maximum
class probability threshold of 0.9, F1 increases to 0.942 while retaining 65.2%
of the test tumors. The latter is not 94.2% accuracy on all CUP patients.
[Source: version-1 results](https://www.medrxiv.org/content/10.1101/2022.12.22.22283696v1.full).

## What already exists, and where it falls short

| Capability | Evidence in JOSH | Remaining gap |
| --- | --- | --- |
| Molecular inputs | [Molecular importer](../../crates/josh-ingest/src/molecular.rs): canonical tables, annotated MAF/VCF subsets, mutation/CNA/signature/demographic features | No established equivalence to the paper's panel-specific preprocessing; reviewed coverage, annotation, missingness and cohort adapters are needed. |
| Cancer labels | [Molecular core](../../crates/josh-core/src/molecular.rs): 22 detailed experimental classes, plus insufficient-evidence and other-origin outcomes | Shared class names do not establish equivalent predictions; parent groups and extra outcomes need a frozen benchmark mapping. The legacy Clinical path has a different taxonomy. |
| Origin prediction | The same core prepares typed Jev questions, validates responses and applies abstention rules | No locally trained OncoNPC model or cancer-specific Jev validation; raw scores and thresholds are uncalibrated. |
| Signature processing | [SBS96/NNLS implementation](../../crates/josh-features/src/signatures.rs), imported signatures and indexed FASTA handling | External catalogue/genome assets, assay suitability and uncertainty remain unvalidated. NNLS fitting is not evidence of matching the original preprocessing. |
| Explanations | [Rust attribution engine](../../crates/josh-explain/src/lib.rs) and [terminal/SVG charts](../../crates/josh-app/src/molecular_charts.rs) | Explanations describe Jev outputs; biological correctness does not follow from their numerical reconciliation. Provider variation and background sensitivity need broader measurement. |
| Classification evaluation | [Evaluator](../../crates/josh-features/src/evaluation.rs): confusion counts, per-class precision/recall, macro/weighted F1, top-1/top-3, Brier, log loss, ECE/reliability and coverage/error | No real cohort results, confidence intervals, fitted calibration, or institution/panel/time validation. Threshold-specific weighted F1 is not currently reported. |
| Cohort execution | [Runner](../../crates/josh-app/src/cohort.rs): separate labels, patient partition checks, budgets and resumable archives | Live execution is synthetic-only and limited to 1,000 cases per manifest. Research-data use and larger-scale operation need an explicit supported workflow. |
| Clinical outcomes | Clinical review prompts and records exist | No survival endpoint/event/censoring schema, Kaplan–Meier estimator, Cox model, propensity weighting, treatment-concordance study or germline-risk validation. |

The original classifier learns from somatic mutations, copy-number changes,
mutation signatures, age and sex using XGBoost. Jev consumes a different feature
representation through a hosted model. The authors provide a preprocessing and
model repository, which is a useful external comparator but is not installed in
JOSH. [Source: original implementation](https://github.com/itmoon7/onconpc).

The recorded [JOSH validation](../reports/0.7.0-molecular-validation.md) used invented
inputs. A live synthetic case abstained, and repeated full-input scores varied
from 0.61 to 0.63. These observations establish API behavior and some variability;
they cannot estimate cancer accuracy. A successful build or explanation test has
the same limitation.

Operational gaps also matter: molecular requests permit at most 512 features and
32 KiB. A study-scale feature adapter must be checked against those limits;
silently dropping genes to make a request fit would change the experiment. Extra
IHC, histology or expression inputs should be evaluated as separate extensions
when comparing a genomic-only protocol.

## The three requested visual references

The supplied screenshot represents cohort evaluation. Current molecular charts
explain one sample. Both views are useful, but they answer different questions.

| Screenshot panel | Current status | Data and implementation needed |
| --- | --- | --- |
| A: confusion heatmap and recall strip | Confusion counts and recall exist offline; no cohort heatmap in Ratatui | Load an evaluation report; render all truth/prediction classes, sample counts and recall. Label count versus row/column normalization, expose failure/unknown/other outcomes, and show the exact class mapping and cohort. |
| B: survival by predicted type | Missing | Patient-level time origin, entry time where needed, event/censoring indicator, last follow-up and frozen predicted group; Kaplan–Meier curves with risk counts and uncertainty. |
| C: survival by treatment concordance | Missing | Reviewed treatment/concordance labels, treatment and sequencing dates, prognostic covariates and endpoints; adjusted analyses with assumptions and diagnostics. A simple pair of unadjusted lines does not reproduce this panel. |

The study used Cox adjustment and inverse-probability-weighted Kaplan–Meier
analyses for treatment comparisons. Its retrospective associations do not prove
that assigning treatment using JOSH improves survival.
[Source: study methods and discussion](https://www.medrxiv.org/content/10.1101/2022.12.22.22283696v1.full).

The preprint screenshot uses ten broad cancer groups; the published version uses
thirteen. The [publisher correction](https://www.nature.com/articles/s41591-023-02693-x)
changes group assignments and supplementary-data ordering. JOSH's experimental
parent labels should not silently stand in for either mapping. Paper curves can
be cited as external context, but JOSH result plots must derive from its own
explicitly identified cohort and archived predictions.

## Recommended order of work

1. Freeze a Jev evaluation protocol: feature processing, genomic-only input set,
   taxonomy and broad-group mapping, model/prompt versions, patient partitions,
   endpoints and score thresholds. Select a permitted labeled known-primary
   cohort; keep truth labels outside provider state and threshold tuning outside
   the test partition. Resolve research-data eligibility before any live requests.
2. Complete the existing evaluation view: confusion heatmap, per-class metrics,
   coverage versus error, reliability, threshold-specific F1 and patient-bootstrap
   intervals. Support saved reports offline and compare models on the same cases.
   Until representative data are available, synthetic plots verify rendering only.
3. Run a frozen benchmark, compare against an appropriate baseline, audit errors,
   and assess institution/panel shift, abstention and repeatability. Fit any
   calibration on a separate partition and evaluate it on the untouched test set.
4. Add patient-level outcome import and validated survival methods. Review time
   origins, censoring, delayed entry, repeated tumor samples, concordance labels
   and confounders before interpreting the two survival views.

The immediate milestone is a credible estimate of Jev's origin-prediction
performance. Additional graphs will make that evidence understandable; their
presence alone cannot establish predictive parity.
