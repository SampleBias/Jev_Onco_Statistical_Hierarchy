# Source register

## Synthetic CUP workups and Jev v5 — checked 2026-09-26

- [TypeSafe launch article](https://typesafe.ai/blog/introducing-system-one-models-and-jev),
  [documentation index](https://docs.typesafe.ai/llms.txt),
  [state](https://docs.typesafe.ai/concepts/state),
  [Choice](https://docs.typesafe.ai/primitives/choice),
  [Noul](https://docs.typesafe.ai/primitives/noul),
  [confidence](https://docs.typesafe.ai/confidence),
  [models](https://docs.typesafe.ai/models),
  [API](https://docs.typesafe.ai/api),
  [fan-out](https://docs.typesafe.ai/patterns/fan-out),
  [hierarchical classification](https://docs.typesafe.ai/cookbooks/hierarchical_classification)
  and [limitations](https://docs.typesafe.ai/model-jaggedness/jev-1.13):
  current request shape, independent judgments, semantic preprocessing and limits.
- [OncoNPC published article](https://www.nature.com/articles/s41591-023-02482-6),
  [correction](https://www.nature.com/articles/s41591-023-02693-x) and
  [authors' code](https://github.com/itmoon7/onconpc): genomic input families and
  taxonomy. PMC access to the paper returned a browser challenge on this review;
  publisher content, authors' repository and the prior full-text review below
  were used. The paper is not an arXiv/Hugging Face paper, so those endpoints do
  not provide its primary record.
- Losa et al., [SEOM–GECOD guideline (2021), published 2022](https://pmc.ncbi.nlm.nih.gov/articles/PMC8986666/):
  diagnostic workup and IHC patterns/overlap, particularly Table 3. Used as a
  source of diagnostic patterns, not as a current treatment specification.

All new case measurements and combinations are invented. Source-derived design
choices, limitations and scenario notes are documented in the
[sample README](../fixtures/cup-realistic-v2/README.md) and
[Jev workflow](references/JEV_WORKFLOW.md). No real patient dataset was acquired.

## OncoNPC prediction and cohort figures — checked 2026-09-25

Primary research direction: the corrected published study below. It was already
registered before the September 25 request. The preprint is an earlier version of
the same study. The new [research guide](references/ONCONPC_GUIDE.md) and
[current objective audit](assessment/ONCONPC_PARITY.md) are also searchable inside
the terminal through F1, then / and `OncoNPC`.

- Moon I, LoPiccolo J, Baca SC, Sholl LM, Kehl KL, Hassett MJ, Liu D, Schrag D,
  Gusev A. *Utilizing Electronic Health Records (EHR) and Tumor Panel Sequencing
  to Demystify Prognosis of Cancer of Unknown Primary (CUP) patients.* medRxiv.
  Version 1, posted December 26, 2022. DOI:
  [10.1101/2022.12.22.22283696](https://doi.org/10.1101/2022.12.22.22283696).
  [User-supplied full-text link](https://www.medrxiv.org/content/10.1101/2022.12.22.22283696v1.full-text)
  and [indexed version 1 text](https://www.medrxiv.org/content/10.1101/2022.12.22.22283696v1.full).
  This is the preprint requested as the project's research reference.
- Moon I et al. *Machine learning for genetics-based classification and treatment
  response prediction in cancer of unknown primary.* Nature Medicine.
  2023;29:2057–2067. DOI:
  [10.1038/s41591-023-02482-6](https://doi.org/10.1038/s41591-023-02482-6).
  [Full text](https://pmc.ncbi.nlm.nih.gov/articles/PMC11484892/) and
  [user-supplied ResearchGate reference](https://www.researchgate.net/publication/372960217_Machine_learning_for_genetics-based_classification_and_treatment_response_prediction_in_cancer_of_unknown_primary).
  This is the subsequent peer-reviewed publication and primary guide.
- [Publisher correction](https://www.nature.com/articles/s41591-023-02693-x),
  published online November 15, 2023; Nature Medicine 30:607 (2024).
  Consult it when defining cancer groups or using supplementary data.

The September 24 screenshot motivated three implemented cohort views: a confusion
heatmap, survival by predicted cancer type, and treatment concordance. They derive
from a loaded study, with invented demonstration data available. The September 25
image is the single-sample feature ring and scatter from Extended Data Figure 4;
Analysis now has a dedicated OncoNPC view (p). The preprint screenshot uses ten
broad groups; the published article reports thirteen. Freeze the chosen version
and mapping before comparisons. OncoNPC's XGBoost results cannot be attributed to Jev.

The published full article was retrieved from PubMed Central on September 25;
the publisher correction was checked separately. The exact version-1 results and methods were checked against indexed medRxiv text;
direct medRxiv retrieval returned 403/cache errors. Publication metadata and the
correction were checked against PubMed and the publisher. No patient data or model
weights were downloaded. See the [implementation and evidence assessment](assessment/ONCONPC_PARITY.md).

## Molecular explanations — checked 2026-09-23

- [OncoNPC full article and Extended Data Figure 4](https://pmc.ncbi.nlm.nih.gov/articles/PMC11484892/):
  molecular workflow and figure reference; published XGBoost results do not transfer to Jev.
- [PermutationExplainer](https://shap.readthedocs.io/en/latest/generated/shap.PermutationExplainer.html)
  and [Explainer/maskers](https://shap.readthedocs.io/en/latest/generated/shap.Explainer.html):
  attribution semantics; JOSH implements its numerical engine in Rust.
- [Ratatui Canvas](https://ratatui.rs/examples/apps/canvas/): native terminal geometry.
- [COSMIC SBS signatures](https://cancer.sanger.ac.uk/signatures/sbs/): SBS96 context
  conventions; no catalogue data is redistributed by this implementation.
- [GDC MAF format](https://docs.gdc.cancer.gov/Data/File_Formats/MAF_Format/):
  field meaning for the explicitly supported subset.

See the [approved implementation scope](redesign/04-onconpc-jev-explanations.md)
and [separate verification/scientific report](reports/0.7.0-molecular-validation.md).

Checked 2026-09-21. Prefer these original sources over third-party websites using similar Jev branding. Vendor performance and calibration claims are not independent medical validation.

| Source | Used for |
| --- | --- |
| [Open_Nexus pinned source](https://github.com/SampleBias/Open_Nexus/tree/ee8069cdaaf997721cb071dffe1cb243651a5f56) | Actual project contents, gaps, legacy ML and GPL v2 license text |
| [OncoNPC original repository](https://github.com/itmoon7/onconpc) | Upstream context and external assets, distinct from Open_Nexus |
| [Moon et al. OncoNPC study](https://pubmed.ncbi.nlm.nih.gov/37550415/) | Established molecular-classifier research context; no transfer of results to Jev |
| [AACR Project GENIE data](https://aacrprojectgenie.org/data/) | Data access starting point; release/access terms must be checked before acquisition |
| [TypeSafe introduction](https://docs.typesafe.ai/introduction) | Typed model primitives and independent questions |
| [TypeSafe HTTP API](https://docs.typesafe.ai/api) | Endpoint, payload, answers and error types |
| [TypeSafe models](https://docs.typesafe.ai/models) | Version, inputs, limits, customization and dated pricing |
| [Choice](https://docs.typesafe.ai/primitives/choice) | Class distributions and no-match options |
| [Confidence](https://docs.typesafe.ai/confidence) | Difference between confidence and individual probabilities |
| [State](https://docs.typesafe.ai/concepts/state) | Evidence representation |
| [Jev 1.13 limitations](https://docs.typesafe.ai/model-jaggedness/jev-1.13) | Numerical weaknesses, irrelevant context and adversarial content |
| [Hierarchical classification cookbook](https://docs.typesafe.ai/cookbooks/hierarchical_classification) | Future taxonomy exploration, not a clinical posterior recipe |
| [Agent guide](https://docs.typesafe.ai/agent-skill) and [TypeSafe skill](https://github.com/typesafe-ai/skills/blob/main/skills/typesafe-ai/SKILL.md) | Guidance applied to this implementation |
| [TypeSafe legal index](https://docs.typesafe.ai/legal) | Provider agreements and enterprise retention options |

User-supplied link: `https://docs.typesafe.ai/introduction/coding-agents`. It was unavailable through the browser during review; the documentation navigation's accessible agent guide and official skill were read instead. No skill package was installed.

Software references are linked in [Architecture](ARCHITECTURE.md). Dependency resolutions are recorded in `Cargo.lock`; the lockfile is the reproducible build input, not the latest-version tag of a documentation page.

## Clinical review and terminal visualizations — checked 2026-09-22

- [NICE CG104 recommendations](https://www.nice.org.uk/guidance/cg104/chapter/Recommendations):
  selected local diagnostic/review prompts with recommendation IDs; not a full implementation.
- [NICE update information](https://www.nice.org.uk/guidance/cg104/chapter/Update-information):
  withdrawn 2023 gene-expression restrictions, not active prohibitions.
- [NICE overview](https://www.nice.org.uk/guidance/CG104):
  adult scope and recorded 2025-07-16 review; no clinical validation of JOSH is implied.
- [NHS national genomic test directories](https://www.england.nhs.uk/publication/national-genomic-test-directories/):
  external eligibility reference, not rules copied into the classifier.
- [Ratatui BarChart](https://ratatui.rs/examples/widgets/barchart/),
  [Table](https://ratatui.rs/examples/widgets/table/),
  [Chart](https://ratatui.rs/examples/widgets/chart/) and
  [Tabs](https://ratatui.rs/examples/widgets/tabs/):
  implementation references for the installed 0.30.2 release.

NICE pages sometimes returned 403 on direct retrieval; their indexed official
recommendations/update text and official guideline PDF were used for verification.
Exact source mapping, omissions and software-specific safety choices are documented
in the [clinical review guide](CLINICAL_REVIEW_GUIDE.md). Visualizations are interface
choices, not a claim that NICE mandates particular chart formats.

## Cohort statistical methods — checked 2026-09-24

The OncoNPC citations above motivate the study panels; their reported performance
is not assigned to Jev. Method references for the native Rust implementation:

- [R survival: survfit.formula](https://stat.ethz.ch/R-manual/R-devel/library/survival/html/survfit.formula.html):
  product-limit estimation and confidence transformations; weighted observations
  need appropriate variance estimation, not naive Greenwood intervals.
- [R survival: coxph](https://stat.ethz.ch/R-manual/R-devel/library/survival/html/coxph.html):
  proportional hazards, Breslow/Efron ties and infinite-coefficient limitations.
  JOSH explicitly uses Breslow ties, not coxph's usual Efron default.
- [R survival: survdiff](https://stat.ethz.ch/R-manual/R-devel/library/survival/html/survdiff.html):
  log-rank comparison, distinct from propensity-weighted treatment inference.

No R dependency or R implementation is bundled. These references specify methods;
passing software checks is not clinical validation. See the
[Cohort Guide](data/COHORT_GUIDE.md) for explicit assumptions and missing analyses.
