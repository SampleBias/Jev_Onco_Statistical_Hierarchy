# Scope and architecture decisions

Recorded 2026-09-21. Role owners await assignment to named developers/reviewers.

| ID | Decision or open item | Status and owner | Rationale / completion evidence |
| --- | --- | --- | --- |
| D001 | Rust application; Jev is the sole classifier | Accepted by user; technical lead | Maintain the pure case → request → result boundary; transport does not calculate a replacement classifier |
| D002 | Exclude legacy classifier and original weights; implement Rust model-agnostic explanations of Jev | Revised and approved by user 2026-09-23 | Native exact/permutation Shapley replaces the historical blanket SHAP exclusion; no Python SHAP, TreeSHAP or surrogate classifier |
| D003 | CLI and TUI are supported interfaces | Accepted by user; app lead | Both share validation, preparation and result interpretation |
| D004 | Structured pathology/IHC/clinical evidence first; genomic track separate | Working assumption; product + clinical lead | Existing schema supports summarized observations; no answer to the earlier modality question is recorded |
| D005 | Research review only; no calibrated probability until evaluated | Engineering scope; clinical review pending | Distinct raw scores, calibration metadata, abstention and review state |
| D006 | Keep live synthetic-only restriction | Current implementation; data steward owns expansion | Dataset rights and provider handling approval remain unverified |
| D007 | Use known-primary blinded cases for measurable feasibility | Draft; ML + clinical lead | True CUP ground truth may be unresolved; score against an independent reference |
| D008 | Private GitHub repository | Accepted by user; repository owner | `SampleBias/Jev_Onco_Statistical_Hierarchy` is private; distribution license remains undecided |
| D009 | Keep source audit; write fresh Rust implementation | Implemented; technical lead | Historical reference Open_Nexus commit `ee8069cdaaf997721cb071dffe1cb243651a5f56`; no upstream model weights or patient datasets imported |
| D010 | Pinned toolchain, generated contracts and dependency inventory | Implemented; platform + backend | See the foundation handoff and committed artifacts |

Open decisions: named owners, population/eligibility signoff, nonoverlapping taxonomy and ontology mapping, dataset release/access rights, provider real-data handling, cohort sizes, scientific operating points and distribution/license choice. These are review inputs, not reasons to block synthetic engineering work.

The September 23 approval also prioritizes mutation/CNA/SBS input alongside
expression, detailed experimental cancer labels, and circular/scatter explanations
in Ratatui. The 22-class taxonomy is an experimental option set, not inherited
validation from OncoNPC. Mathematical/operational contracts are described in the
[molecular guide](../data/MOLECULAR_GUIDE.md).
