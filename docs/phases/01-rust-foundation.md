# Phase 01 — Rust foundation and contracts

Status: initial implementation delivered; hardening tasks remain. Lead: Rust backend engineer. Reviewers: technical lead and QA. Estimate: 4–7 person-days. Depends on: Phase 00 scope draft. Unlocks: ingestion, provider integration and mock UI development.

## Outcome

A reproducible workspace and a small stable contract between imports, Jev and the app. Developers can prepare cases and test failure paths without credentials or patient data.

## Delivered now

- `nexus-core`: strict JSON case types, illustrative taxonomy, typed Jev questions/answers, result validation, abstention, version strings and request fingerprint.
- `nexus-jev`: pinned-model HTTPS adapter with synthetic-only preflight, bounded responses and sanitized errors.
- `nexus-app`: `validate`, `prepare`, `demo`, `replay`, `classify`, `example`, `doctor`, `taxonomy`, `tui`, and loopback `serve` commands. CLI includes stdin, JSON/text output and protected file exports.
- Ratatui TUI: evidence/request/results/help views, open/reload, demo/live actions, scrolling, JSON exports and terminal restoration.
- Synthetic example, offline API tests, model-contract tests, `Cargo.lock`, formatting/lint/test CI definition.

## Remaining work packages

| ID | Owner | Task and concrete output |
| --- | --- | --- |
| P01-01 | Backend | Review public types against Phase 00; publish machine-readable JSON Schema and an OpenAPI document generated from the same contract |
| P01-02 | Backend | Separate domain, prompt/taxonomy and policy modules as they grow; keep all question text and all gate constants easy to review |
| P01-03 | Backend | Add structured error codes across CLI/API with consistent JSON error envelopes; preserve source content and secrets from error logs |
| P01-04 | QA | Extend delivered CLI/TUI tests with trailing-JSON and additional size-limit fixtures; retain pipes, exports, credential masking, keyboard workflows and terminal restoration checks |
| P01-05 | Backend | Specify canonical serialization/order behavior for fingerprints; bind result artifacts to request and case revision for authenticated replay later |
| P01-06 | Platform | Select a reproducible toolchain/MSRV, supported OS targets and dependency-update policy; test a clean machine build |
| P01-07 | QA | Pin CI action revisions and add dependency/license inventory consistent with the project's eventual distribution choice |
| P01-08 | Technical lead | Publish module ownership and contract-change rules; require schema/version bumps for incompatible changes |

## Interfaces and behavior

`Case -> prepare -> JevRequest` is a pure, local operation. `JevResponse -> interpret -> ResultRecord` also runs locally and rejects invalid outputs. Network transport does not own probability normalization or clinical semantics. Keep these boundaries so transport failures and scientific failures can be tested separately.

Local API routes currently are `GET /health`, `POST /v1/prepare`, and `POST /v1/demo`. The proposed authenticated application routes in Phase 05 are a different milestone. Do not advertise undocumented registration/history endpoints from the old README.

The live adapter accepts declared synthetic cases. That is an initial development restriction, not a PHI detector. No later developer should remove it merely to make a real-data fixture pass; enabling an authorized cohort is an explicit data-policy configuration change after Phase 00/02 work.

## Acceptance criteria

- On a fresh supported machine, `cargo build --workspace --locked`, formatting, Clippy and tests pass.
- Offline commands require no network access once dependencies are cached.
- Invalid inputs, invalid class sets, nonfinite/range errors, probability-sum errors, incorrect answer types and model drift fail deterministically.
- High model probability never creates an autonomous diagnosis; all mock/replay outputs identify their source.
- No key, raw patient text or provider error body appears in diagnostic logs.
- Schema docs and examples are generated/checked against the real types, not manually maintained incompatible copies.

## Handoff

Deliver the workspace, clean-build log, contract artifacts, module ownership and a short recorded demo. Phase 02 receives the case schema; Phase 03 receives the prepared-request contract; Phase 05 receives offline routes. Foundation completion does not establish live provider compatibility or cancer accuracy.
