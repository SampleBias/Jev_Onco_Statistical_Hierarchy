# Jev suitability for this project

Reviewed against official TypeSafe documentation on 2026-09-21. Recommendation: **build a Jev-centered research classifier and measure its suitability before promising CUP diagnostic probabilities.** No reviewed source establishes Jev-specific CUP diagnostic accuracy.

## Verified capabilities

Jev accepts textual state, including structured JSON. Choice returns a selected option and distribution; Noul returns a yes-probability; Score evaluates an ordered rubric. Independent questions can share a request. The REST contract is `POST https://api.typesafe.ai/v1/systemone`, with bearer authentication and `model`, `state`, `questions`. [API reference](https://docs.typesafe.ai/api), [introduction](https://docs.typesafe.ai/introduction)

The documented model is `jev-1.13.0`. It accepts text, not images/audio/video, and supports a 64k total request budget with 32k for state plus the longest question. TypeSafe does not offer customer fine-tuning/LoRA for Jev; domain adaptation uses evidence and criteria. Published direct-API input pricing is $0.042 per million tokens, with output free. These are dated vendor specifications, not a guaranteed quote. [Models](https://docs.typesafe.ai/models)

The vendor identifies weaknesses with arithmetic, irrelevant long context, indirection and adversarial state content. Our response is deterministic preprocessing, concise evidence, explicit questions, and adversarial evaluation. The provider's `confidence` summarizes the shape of its probability distribution; it is not the probability that an entire diagnostic workflow is correct. [Known limitations](https://docs.typesafe.ai/model-jaggedness/jev-1.13), [confidence](https://docs.typesafe.ai/confidence)

## Mapping to CUP

| Requirement | Jev role | Rust/application role |
| --- | --- | --- |
| Rank possible primary origins | Choice over a reviewed taxonomy | Preserve full distribution, handle no-match categories |
| Determine whether evidence supports assignment | Separate Noul judgment | Combine with missing-data checks and measured abstention rules |
| Detect conflicting observations | Separate Noul judgment | Retain individual findings and show conflicts for review |
| Import tumor mutation/CNA records | Evaluate a compact, semantic evidence representation | Join records, preserve assay coverage, annotate and compute exactly |
| Interpret IHC/pathology findings | Evaluate normalized findings or bounded report excerpts | Preserve positive/negative/unknown states and source provenance |
| Present supporting evidence | Later select source evidence using bounded questions | Display actual source excerpts with provenance and uncertainty |
| Report a trustworthy cancer probability | Supply raw scores for evaluation | Fit and independently test a calibration layer, if data supports it |

This table is our proposed design, not a claim that the provider has validated these medical tasks.

## What importing data means

Data serves three separate purposes: case inputs for inference; reviewed references or examples for prompt development; and labeled cohorts for evaluation/calibration. Importing a GENIE cohort does **not** train Jev's weights. Keep outcome labels outside model inputs. If retrieval is later added, retrieve only from an approved development partition and document it as a separate classifier version.

Start with structured findings and an optional compact molecular summary. A genomic-only experiment is useful because it matches the original project, but Jev may not match a trained numerical classifier on sparse mutation/CNA features. Establish this empirically. RNA expression and raw variant-file interpretation are later, separate feasibility studies, not features promised by the initial adapter.

## Probability design

Use one mutually exclusive Choice distribution for origin ranking. Do not normalize independent per-origin Noul outputs into purported cancer probabilities. Do not multiply evidence-sufficiency, conflict, and origin probabilities: these are not established independent events. `other_origin` and `insufficient_evidence` are operational outcomes, so this initial distribution is over **assignment outcomes**, not exclusively biological origins.

Store raw outputs intact. A `0.85` raw lung score means Jev placed that mass on the lung option for this request. It does not yet justify saying “85% chance this patient's primary is lung.” Domain calibration needs a held-out cohort, uncertainty estimates, and external testing. A calibration model cannot rescue an uninformative classifier or establish generalization to genuinely unresolved CUP cases by itself.

The initial gates (`top >= 0.75`, `margin >= 0.15`, sufficiency `>= 0.8`, conflict `<= 0.2`) are deliberately labeled engineering defaults. They have no established clinical operating characteristics. Even results passing them require review. See [Phase 04](../phases/04-evaluation-and-calibration.md).

## Why the first implementation stays flat

TypeSafe's [hierarchical classification cookbook](https://docs.typesafe.ai/cookbooks/hierarchical_classification) is a useful future approach for a large ontology. Its path-ranking construction is not automatically a normalized clinical posterior. We start with one compact Choice taxonomy to make coverage, probabilities, and failures easier to audit. Broad-to-subtype classification is a separate measured experiment after the flat baseline.

## Operational unknowns

Confirm target medical/research use with the provider, applicable data-processing terms, retention, region, account limits and version availability before uploading a real cohort. The [legal index](https://docs.typesafe.ai/legal) links the relevant agreements and says enterprise zero retention is available; this does not establish a healthcare agreement for this project. No documented public self-hosting path was verified. The first runtime is a Rust client of hosted Jev, not a Rust port of Jev's model weights.

The user-provided coding-agents URL was unavailable during review. The accessible [agent guide](https://docs.typesafe.ai/agent-skill) and its [official SKILL.md](https://github.com/typesafe-ai/skills/blob/main/skills/typesafe-ai/SKILL.md) were used directly. We followed the live API, state, uncertainty and classification guidance; no installer was run.
