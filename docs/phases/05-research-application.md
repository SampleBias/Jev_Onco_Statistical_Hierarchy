# Phase 05 — Rust research application

Status: planned; current interfaces are CLI, TUI and offline API. Leads: Rust frontend and backend engineers. Reviewers: product, clinical lead and QA. Estimate: 10–18 person-days. Depends on: Phase 01 contracts and Phase 03 interfaces; calibrated display depends on Phase 04. Unlocks: persistent review workflows and browser access.

## Outcome

A researcher can import a case, inspect the evidence sent to Jev, request classification, understand uncertainty, and record review decisions. Implement the web application in Rust using Leptos with Axum. Use PostgreSQL and SQLx for durable state; no legacy MongoDB migration is needed until actual legacy records are identified.

Maintain the CLI and TUI as supported application interfaces. Share persistent case/run/review services with the later web interface so terminal users gain the same provenance and review capabilities.

## User workflow

1. Sign in to the research workspace and create/import a case.
2. Review observations, source locations, missing data and importer warnings; correct source interpretations explicitly.
3. Preview the versioned inference input and start a Jev run.
4. See a ranked distribution with clear raw/calibrated labels, unresolved probability mass, abstention reasons and source evidence.
5. Add a review note, accept the result for research annotation, or mark disagreement/insufficient evidence. A review decision never overwrites the raw model output.
6. Export a research report with versions, limitations and reviewer attribution; compare new runs after an evidence revision.

## Work packages

| ID | Owner | Task and concrete output |
| --- | --- | --- |
| P05-01 | Frontend + clinical | Prototype import/evidence/results/review screens with synthetic data; evaluate comprehension of probability and abstention labels |
| P05-02 | Backend | Implement migrations for cases, revisions, sources, runs, calibration artifacts, reviews and audit events |
| P05-03 | Backend/platform | Integrate an identity provider and roles: researcher, reviewer, administrator; enforce authorization at every record access |
| P05-04 | Backend | Implement `POST /v1/cases`, case revision endpoints, `POST /v1/cases/{id}/runs`, run status/results, review and history routes |
| P05-05 | Frontend | Build accessible Leptos forms, import error review, queue states, distributions and evidence tables |
| P05-06 | Backend | Integrate queued Jev inference; persist request/result metadata and prevent duplicate active runs for the same intended operation |
| P05-07 | Frontend + ML | Display full probability mass, top-N with explicit remainder, abstention and optional calibration provenance; avoid diagnostic-certainty wording |
| P05-08 | Backend/frontend | Add JSON/CSV and readable report exports with schema/version metadata; separate source text from generated template labels |
| P05-09 | QA | Implement end-to-end tests across two users/roles, evidence revisions, failed/retried jobs, abstentions and conflicting reviews |

## Presentation rules

The default label is “Jev model score” or “raw assignment probability,” accompanied by calibration status. “Calibrated probability” appears only for compatible, accepted artifacts. Provider confidence is secondary metadata, not a replacement for the class distribution. The example score must never be presented as a patient prognosis or treatment recommendation.

Show all no-match/unknown mass. If only three categories are displayed initially, show the remainder and an expansion control; do not rescale displayed bars to total 100%. Special outcome labels should be visually distinguishable from origin categories.

The evidence panel shows actual imported observations with source references and correction history. Do not manufacture a narrative justification because Jev does not provide one. Raw input and model judgments must remain distinguishable. Demo mode uses a persistent visible label and separate storage namespace.

## Storage and API requirements

Treat source data, case revisions, model runs and reviews as distinct entities. A new finding creates a new case revision. A prior run remains reproducible, but its review is marked as applying to the old revision. Evaluation labels live outside ordinary inference records. Enforce tenant boundaries on all queries, exports and caches.

API responses include stable machine error codes, job/run IDs and clear retryability. Keys remain server-side. Request body sizes and import quotas are explicit. Replace the current unauthenticated local API only after the authenticated app workflow is ready.

## Acceptance criteria

- The complete import-to-review workflow works against synthetic and authorized research fixtures.
- A reviewer can explain the difference between raw scores, provider confidence, calibration and abstention from the interface.
- Authorization tests prevent cross-user/tenant access, including history, source files, exports and cache reuse.
- New evidence cannot silently update a past result; all runs and reviews preserve their original version bindings.
- Provider outages, invalid responses and canceled jobs produce clear recoverable states without losing imported cases.
- Keyboard navigation and basic accessibility checks pass; probability charts have equivalent tabular text.

## Handoff

Deliver the app, migrations, API specification, review/export templates and end-to-end test evidence. UI completion does not release a clinical product. Phase 06 owns deployment readiness; Phase 07 owns the research-pilot decision.
