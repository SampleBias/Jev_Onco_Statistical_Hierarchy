# Phase 03 — Jev classifier and inference service

Status: initial REST adapter and question set delivered; live verification and hardening remain. Lead: Rust backend engineer. Co-owner: ML engineer. Reviewers: clinical lead and QA. Estimate: 6–10 person-days. Depends on: Phase 01; Phase 02 fixtures needed for final acceptance.

## Outcome

A dependable, versioned Jev classification service with inspectable requests, intact probabilities, controlled cost and explicit failures. Use the [official API contract](https://docs.typesafe.ai/api) and [TypeSafe skill](https://github.com/typesafe-ai/skills/blob/main/skills/typesafe-ai/SKILL.md); recheck them when upgrading.

## Current implementation

The adapter uses the official endpoint and pinned model. A Choice ranks the origin options; independent Noul questions assess evidence sufficiency and conflict. Rust checks response structure and probabilities. The CLI can make one live synthetic call with a configured key. No live call has yet been verified; no retries, queue or shared client pool is implemented.

## Work packages

| ID | Owner | Task and concrete output |
| --- | --- | --- |
| P03-01 | Backend | Run an authenticated synthetic smoke test; record actual response shape, resolved version, usage, latency and sanitized provider request identifier |
| P03-02 | Clinical + ML | Replace demo taxonomy descriptions with Phase 00 definitions; write reviewed prompts and development-set examples; keep outcome labels out of case state |
| P03-03 | Backend | Add a transport boundary supporting a shared Reqwest client and an injected mock server; production destinations remain allowlisted HTTPS endpoints |
| P03-04 | Backend | Implement bounded backoff with jitter and `Retry-After` handling for documented transient failures; cap attempts, wall time and cost; define policy for ambiguous timeouts |
| P03-05 | Backend | Add request queue, concurrency limits, cancellation, circuit breaker and duplicate-job handling; do not claim provider idempotency unless documented |
| P03-06 | ML + backend | Measure token usage and enforce request budgets; surface oversize/unsupported inputs explicitly instead of silently truncating them |
| P03-07 | Backend | Persist immutable raw output, exact request/version fingerprint and operational metadata; redact content from normal telemetry |
| P03-08 | QA | Test unauthorized, validation, rate-limit, overload, timeout, redirect, oversized/malformed body, partial answer, unexpected taxonomy and changed-model responses |
| P03-09 | ML | Compare flat Choice prompt variants only on the development split; record option-order sensitivity, demographic-only behavior and adversarial report text |
| P03-10 | Backend + data steward | Add a reviewed policy for authorized research cohorts; keep synthetic-only mode as the default development configuration |

## Question and probability rules

Origin assignment, evidence sufficiency and conflicting evidence are separate judgments. They can share one request but do not condition on each other's answers. The initial origin taxonomy includes no-match options. If a future stage needs the first-stage answer to fetch supporting evidence, make it a second explicit request and version the composed workflow.

Keep one origin distribution; do not add a softmax, multiply independent answers or normalize a top-N display. Preserve provider confidence separately. The parser should reject model drift and invalid probability vectors. A high-probability answer with insufficient evidence remains abstained; a transport error remains an error.

Evidence excerpts in the UI must come from stored sources. Jev does not generate medical explanations. If evidence-selection questions are added, score their faithfulness separately; selected passages are supporting observations, not proof of a causal model attribution.

## Acceptance criteria

- Synthetic live request/response matches the documented contract; the exact result is recorded honestly even if model judgments are poor.
- Contract failure cases are covered with a mock HTTP server, including body streaming limits and sanitized errors.
- Provider requests have versioned taxonomy/prompt/model; every result retains the complete raw distribution.
- Retries are bounded and observable; duplicate job submissions cannot create an unbounded bill or inconsistent active results.
- A reproducible evaluation runner can consume case IDs and return per-case results/errors without leaking labels.
- Measurements report end-to-end p50/p95 latency, token usage, provider failure rate and cost per case at the tested concurrency.
- Provider outage cannot cause the application to substitute a mock result as a real prediction.

## Handoff and risks

Deliver the provider contract report, reviewed question bundle, transport tests, inference API and an operational limits report. Phase 04 receives frozen classifier versions; Phase 05 receives asynchronous job/result interfaces. Changes to options, their descriptions or evidence packing are model changes for evaluation purposes, even when the provider version is unchanged.
