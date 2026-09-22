# Source register

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
