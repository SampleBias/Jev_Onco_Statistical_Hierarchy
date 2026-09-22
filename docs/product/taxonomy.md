# Origin dictionary — development draft

Version: `demo-primary-sites-v0.1`. Owner: clinical/pathology reviewer, unassigned. Review status: pending for every entry. The executable dictionary is `crates/josh-core/src/taxonomy.rs`; `josh taxonomy` prints its current descriptions. This draft documents the existing prototype, not a medically approved classification system.

| Runtime ID | Prototype grouping | Inclusion/exclusion questions to resolve |
| --- | --- | --- |
| `lung` | Primary lung malignancy | Histologic/lineage eligibility and boundaries with other thoracic sites |
| `breast` | Primary breast malignancy | Eligible histologies and uncommon lineages |
| `colorectal` | Colon or rectal primary | Appendix and other lower gastrointestinal boundaries |
| `pancreas` | Pancreatic primary | Ductal, neuroendocrine and other lineage eligibility |
| `biliary_tract` | Biliary primary | Site grouping and distinction from pancreatic/hepatic origins |
| `upper_gastrointestinal` | Gastric or esophageal primary | Junctional sites and whether to split the grouped label |
| `renal` | Kidney primary | Renal parenchymal versus urothelial and non-epithelial eligibility |
| `urothelial` | Urothelial primary | Cross-organ lineage group; precedence over organ labels |
| `prostate` | Prostatic primary | Histologic eligibility and rare variants |
| `gynecologic` | Gynecologic primary | Included organs and whether this group must be split |
| `thyroid` | Thyroid primary | Lineage and histologic eligibility |
| `melanoma` | Melanocytic malignancy | Lineage versus organ assignment; overlaps and unknown-primary handling |
| `other_origin` | Supported origin outside the listed categories | Reviewer-defined reference labels that map here; must not absorb missing evidence |
| `insufficient_evidence` | Cannot support a primary-origin assignment | Missing/conflicting evidence; not a biological tumor class |

No ontology mappings are asserted yet. Phase 02 must record the source diagnosis code, ontology/version and reviewer-approved mapping to a target label, or an explicit unmapped reason. Do not infer mappings from substring matches or copy upstream class order.

The medical review must produce mutually exclusive label definitions, inclusions, exclusions and precedence rules; adjudicate rare/hematologic/multiple-primary cases; decide how no-match outcomes are scored; and assign a new taxonomy version before a medical benchmark. Keep the current vocabulary stable for synthetic contract tests until that review is complete. A fixture choosing `lung` only exercises software behavior.
