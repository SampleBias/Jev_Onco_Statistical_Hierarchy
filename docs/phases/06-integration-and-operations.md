# Phase 06 — Integration and operations

Status: planned. Lead: platform/QA engineer. Reviewers: backend, data steward and technical lead. Estimate: 7–12 person-days. Depends on: Phases 02–05 and the selected data-use scope. Unlocks: controlled research deployment.

## Outcome

An integrated release that handles data and provider failures predictably, can be restored, and can be rolled back to a previously evaluated classifier version. This phase turns a local prototype into an operated research service.

## Work packages

| ID | Owner | Task and concrete output |
| --- | --- | --- |
| P06-01 | Platform | Produce reproducible release builds/container images, dependency inventory, environment configuration and deployment manifests |
| P06-02 | Backend/platform | Define secrets, TLS, database roles, per-tenant authorization, session handling and allowed provider destinations |
| P06-03 | Data steward | Verify processing region, retention/deletion behavior and actual provider agreement for the selected data; document what “deidentified” means operationally |
| P06-04 | Platform | Add telemetry for throughput, failures, abstention rate, tokens, cost and latency without logging raw case text |
| P06-05 | Backend | Exercise bounded retries, queue backpressure, cancellation, duplicate requests and provider circuit breaking under load |
| P06-06 | QA | Run integration scenarios from import to report for missing/partial evidence, out-of-taxonomy cases and every relevant provider error |
| P06-07 | Platform | Implement backups and restoration drills for database/artifacts; verify retention and tenant-scoped deletion |
| P06-08 | Technical lead | Version release bundles and document rollback for app, prompt, model, taxonomy, importer and calibration changes |
| P06-09 | QA/security | Test untrusted uploads, injected evidence, oversized requests, authorization bypass and secret leakage; track findings to closure |
| P06-10 | Platform + product | Set operational SLOs and capacity based on measurements, with run budgets and clear behavior at quota exhaustion |

## Failure behavior

An importer rejection must retain an actionable error report. A failed inference must not erase the case. A provider timeout may have incurred cost; retry policy must make that visible. A partial/malformed response cannot become a completed classifier run. A missing calibration artifact falls back only to an explicitly uncalibrated research display if the release policy permits it, never to a false calibrated label.

When Jev is unavailable, queue or fail requests with a user-visible status. Never silently swap to another model or a canned result. Changing the backend changes the validated system. An expired evaluation/model support window should disable new validated claims while preserving old reports and their original provenance.

## Release bundle

Include application binary/image digest, lockfile/dependency report, migrations, case schema, importer versions, prompt bundle, taxonomy, provider/model pin, gate policy, optional calibration artifact, evaluation report and operational runbooks. Missing pieces remain explicit release blockers for the capabilities that depend on them.

Separate private evidence storage from ordinary logs. Case request fingerprints are not anonymity guarantees. Restore and deletion testing must include derived artifacts and caches, not only primary database rows. The current prototype has no persisted patient information to migrate.

## Acceptance criteria

- A clean environment can deploy the same bundle, run the integration suite and recover from a backup using the written procedure.
- No unresolved authorization or secret-handling defect remains in the intended deployment paths.
- Measured performance satisfies written operational targets at expected load; resource/cost limits produce controlled rejection or queuing.
- All provider failure modes preserve truthful job states, and mock/live storage cannot be confused.
- Tenant isolation covers artifacts, exports and cache keys, not only main database tables.
- Rollback restores the full classifier configuration and never pairs an old prompt with an incompatible calibrator.
- The exact allowed data scope and external-processing configuration are recorded for pilot operators.

## Handoff

Deliver the tested release candidate, runbooks, dashboard definitions, integration/restore reports and remaining-risk register. Phase 07 uses this bundle unchanged for its pilot unless a documented defect requires a new version and repeat checks.
