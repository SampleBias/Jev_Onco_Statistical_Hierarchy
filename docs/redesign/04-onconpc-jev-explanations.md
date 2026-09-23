# Proposal: OncoNPC-inspired molecular inference and explanations

Status: **approved by the user 2026-09-23; integrated engineering implementation in 0.7.0**.
Prepared against JOSH 0.6.0, commit `a9c669c`. This document preserves the approved
scope. Current commands and operational limits are in the
[molecular guide](../data/MOLECULAR_GUIDE.md); verified results and remaining
scientific work are recorded in [build status](../BUILD_STATUS.md).

## Intended result

Extend the existing Rust application into a molecular tissue-of-origin research
workbench using Jev for classification and Ratatui for interactive explanations.
The main workflow becomes:

`Sample → molecular inputs → QC/features → Jev prediction → explanation → review/export`

Reference comparison remains available within feature preparation. Genomic-only
analysis must not require expression data or an expression reference. Existing
expression, clinical guidance and legacy workflows remain usable.

The supplied image is Extended Data Figure 4 in the [OncoNPC paper](https://pmc.ncbi.nlm.nih.gov/articles/PMC11484892/).
It combines feature-category rings with a signed attribution scatterplot. The
paper's inputs include somatic mutations, copy-number alterations, mutation
signatures, age and sex; its explanations use TreeExplainer for its XGBoost model.
These establish the workflow and visual reference for this proposal. Jev requires
its own attribution calculations and performance evaluation.

All new application logic, numerical calculations, graphics and exports will be
Rust. Jev continues through its hosted API; this proposal does not imply local
Rust implementation of Jev weights. TypeSafe documents domain adaptation through
state and questions, rather than customer fine-tuning.
[Jev models](https://docs.typesafe.ai/models)

## Findings from the current implementation

| Capability | Current evidence | Proposed change |
| --- | --- | --- |
| Expression datasets and QC | Implemented in `josh-ingest`, `josh-features`, and the workbench | Reuse the sample, artifact, mapping and QC infrastructure |
| Reference comparison | `josh-app/src/reference.rs` builds class means and Pearson evidence | Preserve compatibility checks; make this one supported evidence route |
| Molecular classification | Workbench exports an offline request preview | Complete request execution, molecular response interpretation and run archives |
| Provider integration | `josh-jev/src/lib.rs` accepts legacy `Case`, synthetic only | Introduce a shared typed run/transport boundary with bounded jobs |
| Taxonomy | Molecular preview uses reference classes plus `unknown`; legacy policy validates fixed classes plus `insufficient_evidence` | Bind validation and abstention to the exact versioned request taxonomy |
| Mutations | Generic molecular findings can hold summaries; canonical molecular adapters are planned | Add typed variant observations, gene summaries and assay coverage |
| Copy number | Enum exists; processing is deferred to redesign Phase 3 | Bring gene-level CNA import and QC into the next molecular milestone |
| SBS signatures | No typed signature modality or processing pipeline | Add signature import, identity/version/QC and later native derivation |
| Explanations | No attribution engine; README and original plan exclude SHAP | Propose explicit scope revision permitting Jev model-agnostic attribution |
| Graphics | Scores, IHC, timeline, evidence charts, expression histograms and correlations exist | Add linked circular, scatter, waterfall and feature-detail views |
| Scientific validation | No established JOSH cancer accuracy or calibration | Add frozen cohort evaluation and explanation stability measurements |

Code references: [molecular preview](../../crates/josh-app/src/reference.rs),
[legacy response policy](../../crates/josh-core/src/policy.rs),
[Jev adapter](../../crates/josh-jev/src/lib.rs),
[workbench](../../crates/josh-app/src/workbench.rs),
[existing charts](../../crates/josh-app/src/tui/visuals.rs).

## Proposed scope decisions

1. Jev remains the sole tissue-of-origin classifier; XGBoost and original model
   weights remain excluded.
2. Amend decision D002 and the broad SHAP exclusion to permit a new Rust
   model-agnostic explanation engine for Jev. Do not import Python SHAP or use a
   replacement tree model to generate explanations for Jev.
3. Promote mutations, CNA and imported SBS signatures alongside existing
   expression support. Make age and sex optional, typed evidence.
4. Add a versioned experimental taxonomy based on the paper's 22 cancer labels,
   with explicit broad-group mappings and operational unknown/other outcomes.
   Labels such as NSCLC need distinct IDs; the existing broad `lung` option is
   not silently relabeled NSCLC. Existing taxonomy versions remain readable.
5. Freeze the available classes before a run and across all its perturbations.
   When displaying broader groups, sum mapped leaf probabilities; do not mix
   overlapping broad and detailed options in the same Choice distribution.
6. Preserve raw Jev distributions and display their calibration status. The
   example's 0.98 is not a target value, fixture assertion or JOSH accuracy claim.

After approval, update [decisions](../product/decisions.md), [roadmap](../PLAN.md),
[README](../../README.md), relevant redesign phases, and build status together.
Historical decisions remain documented as superseded where appropriate.

## Milestone A: contracts and a complete molecular Jev run

Create versioned `FeatureSet`, `InferenceRun`, `TaxonomyDefinition`,
`ExplanationConfig`, and `ExplanationResult` contracts. Each feature records an
ID, modality, typed value, units, observation/coverage status, source records,
processing version and dependencies on other features. Distinguish missing,
not tested, measured negative and measured zero.

Move reusable molecular request preparation out of UI orchestration. Make result
validation accept the exact taxonomy/question specification used by the request.
Resolve the `unknown`/`insufficient_evidence` mismatch through explicit version
mapping, preserving existing saved results. Probability vectors must include all
requested options and pass the current finite-value and sum checks.

Keep origin ranking, sufficiency and conflict as separate questions. The API
documents a Choice distribution and independent questions sharing one state;
different masked states therefore require separate evaluations.
[Jev API](https://docs.typesafe.ai/api)

Refactor the transport to reuse a client and support a mock server, bounded
concurrency, cancellation, timeouts and rate-limit handling. Account for uncertain
completion on timeouts before retrying potentially billed calls. Persist exact
requests/responses, versions, hashes, usage and job state; resume completed calls
from the archive. Changing the sample or configuration invalidates stale results.

Acceptance: a synthetic molecular sample completes prepare → infer → validate →
archive → reload in CLI and TUI. Test malformed responses, taxonomy/model drift,
rate limits, cancellation, lost connections and errors without mock substitution.
A live synthetic contract check is recorded separately from offline verification.

## Milestone B: the genomic evidence required by the figure

| Input | First supported representation | Processing requirements |
| --- | --- | --- |
| Somatic mutations | Canonical CSV/TSV, then a documented MAF dialect and annotated VCF subset | Sample selection, build, allele/consequence provenance, declared somatic status, duplicate handling and callable coverage |
| CNA | Gene-level discrete calls and explicitly identified quantitative values | Keep losses, gains and amplifications distinct; retain assay-specific scale and thresholds; missing is not neutral |
| SBS signatures | Precomputed table with signature IDs, values and method metadata | Record signature catalogue/version, units, eligible mutation counts, fitting/projection method and QC |
| Age/sex | Optional typed metadata | Preserve unknown values and censored ages; distinguish age at sampling/sequencing from unrelated dates |
| Expression | Existing imported datasets and compatible reference evidence | Preserve normalization, dictionary, reference and coverage checks |
| IHC/histology | Existing findings plus structured modality attachment | Preserve status, assay, specimen, timepoint and source links |

Introduce signature identity independently of display names. A projection score,
fitted exposure and relative fraction are different quantities. Never infer SBS4
from a smoking history or a single gene mutation. Do not infer amplification from
RNA expression. Clinical notes and assay measurements retain separate provenance.

Use immutable manifests to join modalities to the same sample. Freeze a bounded
evidence-selection policy before prediction; record omitted input features and
reasons. The displayed top ten explanations are selected after attribution, not
used to silently reduce or change the model being explained.

Acceptance: genomic-only and mixed-modality samples work without mandatory RNA;
unknown coverage cannot become wild type; invalid builds/units/signature releases
produce explicit errors or unavailable results; conflicting assays remain visible.

## Milestone C: model-agnostic SHAP/Shapley calculations in Rust

Add `josh-explain`, depending on core contracts and an injected evaluator rather
than terminal widgets. Jev evaluations supply the model outputs. Attribution
math, feature replacement, job scheduling and diagnostics run in Rust.

The target is a fixed class's raw Choice probability. For a background mode,
define `v(S) = mean_b f_c(assemble(x_S, b_not_S))`, where `f_c` includes the frozen
feature processing, request construction and Jev call. Store the target class,
baseline and output scale. The identity to check is:

`baseline probability + sum(feature contributions) = full-input probability`.

Display contributions in percentage points of raw model output. A later
calibrated-output explanation is a different target and must evaluate the full
calibration mapping on every perturbed output.

Model-agnostic permutation attribution and configurable maskers are established
SHAP approaches. The implementation will be native Rust, using the documented
mathematics rather than invoking the Python package.
[Permutation explanation](https://shap.readthedocs.io/en/latest/generated/shap.PermutationExplainer.html),
[masker and output semantics](https://shap.readthedocs.io/en/latest/generated/shap.Explainer.html)

### Baselines and feature validity

- Reference-background mode uses a frozen, compatible development-only background
  cohort. Store its release/hash, selection rule and weights. Test samples and
  their patient groups cannot enter it.
- An explicit masked-evidence mode replaces hidden observations with unavailable
  evidence. Label it `masked-evidence Shapley`; its baseline measures information
  disclosure, not the expected output of a population. Missing is never encoded
  as a measured negative. Do not silently substitute this for background mode.
- Preserve fixed assay context and valid schema structure. Group inseparable
  measurements, such as compositional signature vectors, where independent
  replacement would invalidate the input. Report the actual attribution unit.
- Track raw-to-derived dependencies. Hiding a mutation must not leave a summary
  sentence revealing it. When explaining raw inputs, recompute all downstream
  features. When explaining final model-input features, clearly state that scope
  and handle correlated summaries as declared feature bundles.
- Current expression evidence contains class similarities, not per-gene Jev
  inputs. Initially explain the expression evidence bundle. Gene-level claims
  require perturbing measurements and recomputing the comparison pipeline.
- Support an internal explanation-only baseline evaluation when all explainable
  evidence is masked. Retain normal inference's empty-evidence safeguards and
  data policy; baseline evaluations cannot be published as ordinary diagnoses.
- Invalid or failed coalition evaluations leave an incomplete explanation. Do
  not drop them selectively, assign zero contributions, or silently renormalize.

### Algorithms, cost and diagnostics

Use exact coalition enumeration for small numbers of feature groups and seeded
forward/reverse permutation sampling for larger feature sets. A grouped game and
a feature-level game produce different attributions: label them separately. For
a feature-level ring, category totals are sums of the absolute leaf contributions
from that same run. Never distribute a category score arbitrarily among genes.
If a later constrained hierarchy is added, identify its values as Owen values.

Four groups have 16 coalitions; six have 64, before background sampling,
replications or retries. Ten individual features have 1,024 coalitions before
those multipliers. Large inputs therefore need explicit evaluation budgets.
Permutation planning uses a conservative bound of approximately
`2 × permutations × (features + 1) × background_samples` evaluations before reuse.

Provide dry-run request/usage estimates, a maximum request count, token and wall
time limits, cancellation and resume. Reserve budget for repeated full/baseline
calls to measure provider variability. Cache keys bind every relevant model,
prompt, taxonomy, feature, background and masking version. One distribution can
support explanations for multiple target classes without rerunning those states.

Report permutation sampling error, baseline sensitivity, repeated-call variation,
completed evaluations and additivity residual. These diagnostics do not establish
causality or clinical confidence intervals. Pinning and a seed make archived
replay reproducible; they do not guarantee identical future hosted responses.

An optional leave-one-feature-out sensitivity view may be useful, but it must
have its own label and never substitute for SHAP or additive contributions.

Acceptance: exact Rust test oracles for additive and interacting functions; dummy
and symmetry properties; exact-versus-sampled comparisons; dependency/missingness
tests; deterministic replay; failure/cost-limit behavior; class-specific sums;
and live repeatability measurements before promoting live explanations.

## Milestone D: Ratatui graphics matching the requested workflow

Ratatui already provides Canvas, custom shapes, chart axes and Braille/half-block
rendering. The installed 0.30.2 stack can support custom annular sectors and
variable-size points. These require application widgets, not a stock donut widget.
[Ratatui Canvas](https://ratatui.rs/examples/apps/canvas/),
[Ratatui charts](https://ratatui.rs/examples/widgets/chart/)

Create one Rust chart-data model shared by terminal rendering and export.

| Component | Proposed behavior |
| --- | --- |
| Prediction header | Selected cancer class, raw Jev score, calibration status, review/abstention state and run identity |
| Circular explanation | Outer modality ring; inner feature sectors; sector area/angle proportional to absolute contribution; category colors; top ten labels plus visible remainder |
| Attribution scatter | Signed contribution on Y; declared feature encoding on X; zero line; point area proportional to absolute contribution; selected feature label and uncertainty detail |
| Waterfall/bars | Baseline, positive/negative contributions, aggregate remainder and final output; numeric reconciliation |
| Feature inspector | Raw value, display transform, contribution/sign, uncertainty, assay/QC, source record and explanation method |
| Progress/footer | Completed/budgeted evaluations, elapsed time, cancellation, completion or unavailable reason |

The circular chart shows relative attribution magnitude, not cancer probability.
Store positive and negative contributions separately in its details; a strong
opposing feature must not look like positive support merely because its sector
is large. For the remainder, retain both signed sum and sum of absolute values.
Zero total contribution produces an explicit empty state.

Mixed features cannot share an unlabeled raw-value axis. Use a versioned,
feature-specific reference-percentile transform for eligible numerical features;
show categorical features in explicitly coded/faceted views. Preserve original
units in the inspector. If a valid reference transform is unavailable, use a
ranked attribution plot rather than fabricate comparable coordinates.

Keep category colors consistent across views: CNA green, mutations red,
signatures blue, demographic context neutral, and distinct expression/IHC colors.
Use signs, labels and focus indicators as well as color. Correct for terminal cell
aspect ratio; handle label collisions with selection and a side legend.

Wide terminals show the circular and scatter views together. Medium terminals
stack or tab them; small terminals show a readable table and signed bars. Set
breakpoints from render tests, initially targeting 140×45, 120×40, 80×24 and 60×20.
Retain the existing guide/search/navigation behavior. Rendering must not trigger
provider calls. Jev jobs run away from the UI event loop.

Export JSON/CSV and a standalone SVG from the same archived explanation. Add a
Rust PNG renderer if raster output is needed. Native terminal rendering is the
required path; optional terminal image protocols can improve presentation later.
The terminal cannot reproduce publication typography pixel for pixel.

Acceptance: test signed/all-zero/tied contributions, missing methods, extreme
values, top-ten/remainder totals, resize, keyboard selection and cancellation.
Inspect real terminal output and exported charts; verify linked views refer to
the same feature/run. Charts must remain usable offline from an archived run.

## Milestone E: native signature derivation and scientific evaluation

First support imported, provenance-bearing signature values so the complete
workflow can be built without prematurely asserting a signature estimator.
Then add a separately versioned Rust pipeline for compatible annotated variants
plus a pinned reference genome: trinucleotide contexts, SBS96 counts, and the
selected projection or fitting method against a pinned signature catalogue.
Declare callable territory/opportunity assumptions, sparse-count limits,
reconstruction diagnostics and uncertainty. Do not equate a new fitting method
with the paper's preprocessing. Test numerical kernels on known spectra and
review targeted-panel suitability before enabling derived signatures by default.

Add a Rust evaluation runner with patient-disjoint development, calibration and
test partitions; institution/panel/time holdouts where available; and archived
failures/abstentions in the denominator. Freeze taxonomy, evidence selection,
prompts, backgrounds and operating thresholds before final test access.

Report top-1/top-3 performance, macro/weighted F1, class counts, confusion matrices,
Brier score, log loss, reliability and coverage-versus-error. Compare genomic-only,
expression-only and combined inputs; include demographic-only and missingness
checks. Measure explanation ranking/sign stability, background sensitivity and
provider latency/usage. Evaluate calibration only with a suitable held-out split.

True CUP cases with unresolved origin cannot be assigned invented ground truth.
Published OncoNPC results are context; they are not validation of a Jev workflow.
The primary quantitative comparison here is against known labels and frozen
simple reference baselines, without introducing an XGBoost runtime.

Acceptance: an engineering test report and a distinct scientific report that
states supported modalities/populations, failures and unresolved dependencies.
Data access, provider real-data eligibility and sufficient labeled cohorts remain
external requirements for this stage; synthetic implementation can proceed first.

## File and crate ownership

| Location | Work |
| --- | --- |
| `josh-core` | Feature/taxonomy/run/explanation contracts, validation and compatibility |
| `josh-ingest` | Variant/CNA/signature adapters, sample joins, provenance and rejection reports |
| `josh-features` | Genomic summaries, dependency tracking, numerical transforms and signatures |
| `josh-jev` | Shared evaluator/transport, exact request validation and operational limits |
| New `josh-explain` | Baselines, coalition planning, permutation math, diagnostics and replay |
| `josh-app` | CLI orchestration, jobs, linked Ratatui widgets and exports |
| New `josh-eval` or equivalent Rust module | Cohort metrics, calibration experiments and report artifacts |
| `contracts`, fixtures, docs | Schema generation, analytical/synthetic fixtures, guides and verified status |

Conceptual CLI additions: `molecular import`, `analyze prepare`, `analyze run`,
`explain plan`, `explain run`, `explain inspect`, and `explain export`. Final naming
will follow the existing CLI conventions. Commands share services with the TUI.

## Delivery sequence and review points

1. Update scope/contracts and implement molecular inference integration (A).
2. Add genomic import/features (B), while building linked chart widgets against
   explicitly synthetic analytical fixtures (D).
3. Implement/test Rust attribution and connect the actual run archive (C + D).
4. Deliver the first complete single-sample import → Jev → explanation → terminal
   graphics → export workflow using imported signatures.
5. Add raw-variant signature derivation and cohort/scientific evaluation (E).

The first integrated milestone is complete only when predictions and both main
charts use the same archived data, contributions reconcile numerically, missing
evidence remains explicit, jobs stay within budget, and the legacy/expression
regressions pass. Synthetic demonstrations remain visibly labeled.

Run formatting, clippy, workspace tests, schema drift checks and dependency
inventory checks, plus numerical attribution tests and a real PTY smoke test.
No current test pass is claimed by this planning document.

Approval of this proposal authorizes the implementation sequence and the revised
explanation scope. It does not require a further design choice to start synthetic
engineering. Real-data evaluation depends on the named data/provider prerequisites
and an explicit run budget; it is not silently launched as part of building charts.
