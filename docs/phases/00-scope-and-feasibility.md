# Phase 00 — Scope and feasibility

Status: engineering scope and feasibility drafts delivered; product/clinical review and live access remain open. Leads: technical lead and clinical/pathology lead. Estimate: 3–5 person-days. Dependencies: none. Unlocks: Phase 01 contracts and synthetic Phase 02 imports.

## Delivered artifacts and pending review

| Work package | Evidence | Status |
| --- | --- | --- |
| P00-01 | [Source audit](../assessment/OPEN_NEXUS.md) | Delivered |
| P00-02 | [Intended use and field meanings](../product/intended-use.md) | Draft delivered; product/clinical approval pending |
| P00-03 | [Versioned prototype dictionary](../product/taxonomy.md) | Draft delivered; clinical inclusions/exclusions and ontology mappings pending |
| P00-04 | [Data inventory and required manifest](../data/inventory.md) | Local inventory delivered; real source/release/counts/access pending |
| P00-05 | [Feasibility protocol](../evaluation/feasibility-protocol.md) | Draft delivered; cohort size and reviewed scientific gates pending |
| P00-06 | Synthetic live contract smoke | Pending credentials/access; no live verification claimed |
| P00-07 | Dataset/provider external-processing decision | Pending data steward; live adapter remains synthetic-only |
| P00-08 | [Architecture and provenance decisions](../product/decisions.md) | User scope decisions recorded; owner/reviewer assignment pending |

Engineering can proceed with synthetic fixtures. Phase 00 as a whole is **not signed off**; these artifacts make the remaining decisions reviewable.

## Outcome

Define exactly what the app is classifying, from which evidence, and how a useful answer will be judged. Establish Jev as the experimental classifier without importing the legacy model's performance claims.

## Work packages

| ID | Owner | Task and concrete output |
| --- | --- | --- |
| P00-01 | Technical lead | Review the pinned [source audit](../assessment/OPEN_NEXUS.md); record which product workflows are retained and which README claims were never implemented |
| P00-02 | Product + clinical lead | Write `docs/product/intended-use.md`: population, users, research setting, evidence available at prediction time, exclusion rules, and review actions |
| P00-03 | Clinical lead | Review each prototype origin option; produce a versioned label dictionary with definitions, inclusion/exclusion rules and ontology mappings where appropriate |
| P00-04 | Data engineer | Inventory available datasets and modalities; record releases, access routes, rights, expected record counts and ground-truth availability |
| P00-05 | ML lead | Draft feasibility protocol separating genomic-only, clinical/IHC-only and combined cases; specify a small development cohort and a locked test cohort |
| P00-06 | Backend lead | Verify provider contract/model with one synthetic request once a key is available; record sanitized shape, model, usage and latency |
| P00-07 | Product + data steward | Confirm permitted external processing and research use with dataset/provider terms before moving beyond synthetic cases |
| P00-08 | Technical lead | Record the Jev-only classifier decision and provenance of any reused data-processing code or reference datasets |

## Decisions and assumptions

The first cohort should contain confirmed malignancies with an independently known origin hidden from inference. True CUP cases are a separate challenge: often the ground truth remains unknown. Do not score “agreement with another prediction” as confirmed correctness.

The current example schema accepts findings already summarized by a person or a deterministic importer. It does not interpret raw sequencing or pathology images. Unless the product owner prioritizes genomic-only operation, use structured pathology/IHC evidence for the first integration and maintain a distinct genomic feasibility experiment.

The illustrative taxonomy is not ready for a medical benchmark. Decide whether labels describe organ, lineage, histology or a deliberate combination; eliminate overlapping labels and specify behavior for metastatic melanoma, hematologic malignancies, rare primaries and multiple concurrent primaries. Do not claim 22-class OncoNPC parity from the current 14-option demo.

## Acceptance criteria

- Each inference field has a documented meaning, available-at-prediction-time rule and missingness convention.
- A reviewer can distinguish a ranked assignment, model score, calibrated probability and unresolved result.
- Cohort feasibility and taxonomy review are recorded; if no suitable labeled cohort exists, development can continue on synthetic data but scientific validation is explicitly pending.
- Provider contract smoke-test outcome is recorded as passed, failed, or pending access; no fabricated “live verified” claim.
- Quantitative release criteria are drafted before the final test set is examined.
- Decisions include owner, date and rationale; open questions have owners rather than arbitrary defaults disguised as clinical facts.

## Handoff and risks

Hand off the intended-use document, taxonomy draft, data inventory, evaluation outline and decision records. The main risk is that Jev lacks discriminative performance for the available modalities. The mitigation is an early blinded feasibility experiment, not a larger UI build. If it fails, refine evidence/taxonomy within the development set or narrow the product to evidence triage; document that decision explicitly.
