# Intended use — research draft 0.1

**Direction update (0.5.0):** the primary product is a sample/dataset molecular
workbench. Expression import, mapping, QC and exploration are implemented; cancer
reference comparison and molecular Jev inference remain pending. No patient profile
is required. See the [active Phase 1 plan](../redesign/01-samples-expression-and-reference.md)
and [expression guide](../data/EXPRESSION_GUIDE.md). The original case-study draft
below is retained for the legacy classifier; its modality priority is superseded.

Owner: product lead with clinical/pathology reviewer. Written 2026-09-21. Status: engineering scope drafted; clinical approval pending. This document is not evidence that Jev can identify cancer origins accurately.

The application lets a researcher load structured evidence for a malignancy, inspect the exact Jev request, and review ranked candidate origins and unresolved outcomes. The proposed first study uses retrospective, independently confirmed known-primary cases with the origin hidden during inference. Adults are the proposed first population; the general case parser accepts ages 0–120 and does not enforce clinical eligibility. Eligibility belongs in the reviewed cohort manifest.

Current live requests accept declared synthetic cases only. Local preparation, validation and mock workflows can process the supported case contract. A data-class declaration is not an identifier detector or permission to upload data. The clinical reviewer, data steward and study owner must close the relevant Phase 00 decisions before real records are sent to a provider.

## Users and workflow

Researchers and clinical/pathology reviewers import evidence, resolve quality issues, inspect the request, run the experimental classifier, and review the full distribution alongside source findings. An unresolved result leads to review of missing or conflicting evidence. A ranked result also requires review. The application does not select treatment or establish a diagnosis.

The CLI supports scripts and reproducible exports; the TUI supports case-by-case inspection. Neither interface may silently treat mock or unverified replay output as a live Jev result. The initial workbench does not yet edit findings or persist review decisions.

## Evidence contract

| Field | Meaning and rule |
| --- | --- |
| `schema_version` | Case format version; currently 1 |
| `case_id` | Local pseudonymous ID; never included in the prepared Jev state |
| `data_class` | Declared synthetic or deidentified research; local policy metadata, not provider evidence |
| `age_years` | Age at the evidence cutoff, if known; missing/null means unknown |
| `sex_at_birth` | Optional recorded value; never inferred from a name or unrelated observation |
| `specimen_site` | Site sampled, which may be a metastasis; not a primary-origin label |
| `findings[].id` | Stable evidence identifier; unique within a case |
| `findings[].kind` | Histology, IHC, molecular or clinical observation |
| `findings[].name` | Recorded test or observation name; normalization provenance is added by Phase 02 |
| `findings[].value` | Recorded result, including an explicit negative when actually measured; omitted findings mean unavailable, not negative |

Every finding must be available by the study's prediction-time cutoff. The final primary diagnosis, later adjudication, treatment selected because of that diagnosis and evaluation labels are excluded from inference. Unknown fields are rejected, but allowed free-text fields can still contain leakage; Phase 02 curation must inspect their contents. No raw image or sequencing-file interpretation is implemented.

## Cohort rules awaiting review

The first feasibility cohort is proposed to exclude unresolved ground truth, multiple concurrent primaries without adjudication, cases outside the approved label set, and records with no usable pre-cutoff evidence. Log exclusions with reasons; do not simply drop difficult cases after seeing model output. True CUP cases form a separate descriptive cohort until there is an independent reference standard.

Structured histology/IHC/clinical findings are the initial integration assumption. Genomic-only and combined evidence are separate experiment tracks. No scientific conclusion transfers between those tracks without measurement.

## Meaning of a result

`raw_probability` is Jev's distribution across the supplied Choice options. Provider `confidence` and the two Noul scores are separate quantities. `calibrated_probability` remains null until a compatible artifact passes Phase 04 validation. Neither a high raw score nor an engineering gate is a validated CUP probability.

The two runtime states are `abstained` and `review_required`. No state authorizes an autonomous diagnosis. Preserve no-match probability mass and show the full distribution, model, prompt, taxonomy, policy and case/request fingerprints.

## Approval record

Product owner: unassigned; clinical reviewer: unassigned; data steward: unassigned. Population, eligibility, labels and success criteria remain pending review. Software development may continue with synthetic fixtures while these decisions are open. See [decisions](decisions.md), [taxonomy draft](taxonomy.md) and [feasibility protocol](../evaluation/feasibility-protocol.md).
