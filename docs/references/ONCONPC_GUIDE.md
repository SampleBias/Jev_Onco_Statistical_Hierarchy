# OncoNPC research guide and application direction

Checked against the published article, correction and JOSH source on 2026-09-25.

## Research references

Moon I et al. **Machine learning for genetics-based classification and treatment
response prediction in cancer of unknown primary.** Nature Medicine 29, 2057–2067
(2023). [DOI](https://doi.org/10.1038/s41591-023-02482-6),
[full text](https://pmc.ncbi.nlm.nih.gov/articles/PMC11484892/),
[user-supplied ResearchGate page](https://www.researchgate.net/publication/372960217_Machine_learning_for_genetics-based_classification_and_treatment_response_prediction_in_cancer_of_unknown_primary).

This paper **already existed in JOSH's source register** before this change.
The other OncoNPC reference, *Utilizing Electronic Health Records (EHR) and Tumor
Panel Sequencing to Demystify Prognosis of Cancer of Unknown Primary (CUP)
patients*, is the [2022 preprint](https://doi.org/10.1101/2022.12.22.22283696)
of the same work, not independent corroboration.
Use the published article with its
[publisher correction](https://www.nature.com/articles/s41591-023-02693-x)
as the primary research specification; retain the preprint for version history.
The correction changes five cancer-group assignments and the ordering of
Supplementary Data 3 and 5. Freeze the corrected mapping explicitly in a study.

The paper links genomic origin classification, interpretable evidence, independent
CUP validation, prognosis, treatment concordance and actionable alterations.
Those are the application's research objectives. Jev is a different model and
requires its own evaluation. The [current objective audit](../assessment/ONCONPC_PARITY.md)
maps each objective to code and missing evidence.

## Open the OncoNPC ring and scatter in the terminal

Start `./scripts/start`, then choose **Menu → Offline chart demonstration** (d)
and wait for completion. Press **p**, or **Results → View → Paired ring + scatter**.
For saved work, load an explanation JSON or a run directory with a completed
explanation and press **p**. An inference without an explanation cannot supply
feature contributions: **e** opens the optional Jev explanation budget.

The dedicated view is inspired by Extended Data Figure 4. Ring, Scatter and
Waterfall remain in the Results selector; cohort charts and legacy clinical
visualizations remain accessible in their labelled contexts. Tab/Shift-Tab moves
focus through controls; v returns to Summary.

- The outer ring encodes modality totals; inner sectors encode feature-group
  magnitudes. Sector numbers link to the scatter and selected evidence.
- Color identifies modality; white highlights the selected group. Up/Down selects
  a group, including groups outside the largest ten. The scatter then retains
  the largest nine plus that selection. Omitted signed/absolute contributions
  are disclosed when there is room for the expanded inspector.
- The scatter uses signed Shapley contributions in percentage points of raw
  score. Positive and negative values support and oppose the explained class
  relative to the specified baseline. Circle area approximately follows magnitude
  with a visibility floor. The ring alone does not show sign.
- X is a development-background percentile only when compatible numerical values
  exist for every plotted group; otherwise it is explicitly feature rank.
  Measurements with different units are never silently placed on one raw axis.
  Original values, units and provenance appear in the inspector.
- The heading names the **explained target**, including after t changes it using
  cached distributions. It is not necessarily the leading class. The score stays
  labeled raw and uncalibrated; the source and abstention state remain visible.
- Wide terminals show the two plots together. Narrow, tall terminals stack them;
  very small terminals retain the contribution table. At 80×24 the paired view
  uses compact chrome and an abbreviated inspector. Around 120×40 provides more
  room for labels, category percentages, provenance and numerical reconciliation.

JOSH explains Jev through model-agnostic Shapley calculations in Rust. It does not
present the paper's TreeSHAP/XGBoost numbers as Jev results. An analytical demo is
explicitly labeled and does not reuse the screenshot's patient or score.
**s** exports .svg, .csv, .json or .md using the existing export workflows.
SVG is a standalone figure, not a screenshot of current TUI selection/layout.

## North star: trustworthy predictive evidence

The next milestone is a reproducible estimate of Jev's origin-classification
performance on an eligible, representative cohort. Interface completeness is
separate from predictive performance. Preserve **Rust + terminal + Jev**:

1. **Freeze inputs and the experiment.** Review assay coverage, variant/CNA
   encodings, signature provenance, missingness and the corrected taxonomy.
   Separate patient-level development, calibration and test partitions. Pin
   model, prompt, input hashes and preprocessing. Resolve the existing
   synthetic-only live-data boundary before a real-data provider run.
2. **Measure before optimizing.** Compare frozen Jev predictions and external
   baseline predictions on identical held-out patients. Use the existing F5
   confusion, threshold F1/coverage, reliability and bootstrap tools. Add
   institution/panel/time strata and external validation. Retain failures and
   abstentions in denominators. Evaluate calibration on a separate partition.
3. **Improve the evidence Jev receives.** Test compact, consistent genomic
   representations and reviewed modality ablations on development data. The
   current 512-feature/request-size limits need an explicit aggregation protocol;
   silently dropping features would change the experiment. Validate any staged
   hierarchical classification against the single frozen taxonomy. Evaluate
   added expression/IHC/histology as separate extensions.
4. **Make explanations efficient and reliable.** Reuse archived evaluations,
   preserve biologically dependent groups, set a request budget, and compare
   repeatability/background sensitivity. Benchmark time and allocations before
   caching immutable ring geometry or moving work out of redraw. Neither a
   numerical reconciliation nor an attractive explanation proves accuracy.
5. **Complete outcome and actionability evidence.** Review patient linkage,
   time origins and treatment adjudication; validate the statistics independently.
   Add the missing methods named in the audit. Version a reviewed therapy-evidence
   source with cancer context, variant match, evidence level and provenance.
   Keep eligibility/review separate from a claim of individual treatment benefit.

Jev is suited to the typed origin-ranking, evidence-sufficiency and conflict
questions already used here. Candidate extensions include structured extraction
or concordance-review assistance grounded in cited evidence, after task-specific
evaluation and human adjudication. Rust should retain the numerical work:
QC, probabilities/threshold accounting, calibration fitting, Shapley bookkeeping,
survival statistics, hashes, budgets and exports. Do not ask the model to invent
missing measurements or calculate statistical p-values.

## Reduce bloat without removing visualizations

These are recommendations for subsequent implementation; no existing feature or
visualization is removed by this change.

| Area | Proposed simplification | What remains |
| --- | --- | --- |
| Repeated workflow implementations | Reuse common loading, job, provenance and report components beneath the different screens. | Existing commands, file readers and visualizations; explicit molecular/clinical taxonomy boundaries. |
| Duplicate onboarding and old plans | Link to one current research guide and mark historical assessments by date. | Source provenance, useful design history and searchable offline help. |
| Generic workspace expansion | Defer dashboards, project catalogs, web/server expansion and connectors unless a defined cohort/evaluation task needs them. | The terminal research workflow and existing loopback API compatibility. |
| Broad clinical decision support | Keep source-linked review available but focus new work on origin evidence, uncertainty and reviewed treatment associations. | Clinical context, review records and all current clinical charts. |
| Extra model/runtime stacks | Import frozen comparator outputs for benchmarking; avoid installing another classifier runtime into the app. | Jev inference, Rust statistics and native terminal graphics. |

Keep expression QC, reference comparisons, uncertainty, abstention and provenance:
they can directly support trustworthy prediction. Their value should be assessed
with explicit ablations, not removed based on screen count. Keep all current
visualizations, but make their data population, method, uncertainty and research
purpose clear.
