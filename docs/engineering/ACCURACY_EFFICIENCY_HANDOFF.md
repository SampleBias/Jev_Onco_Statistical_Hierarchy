# Accuracy and efficiency: developer handoff

Status: **approved for implementation by the next developer; work has not started**

Review baseline: `main` at `f601ad1`, 2026-11-13

Audience: next-shift Rust developer, research lead, and clinical/data reviewers

## Objective and boundaries

Improve the reliability, reproducibility, measured efficiency, and evaluation of
the molecular-origin workflow. Preserve Jev as the origin classifier and retain
the existing CLI, TUI, saved runs, and research-only interpretation. Do not claim
that these changes establish clinical accuracy or optimize the classifier in the
absence of a suitable independent evaluation.

The user approved this plan for the next developer to implement. Current live
provider calls are restricted to data declared synthetic. Preserve that restriction
and existing data-handling safeguards. Plan approval does not authorize billable
calls, use of patient records, changes to provider eligibility, or deployment.
Obtain the relevant approval before each such action. In particular, prepare a
live-call protocol and explicit cost/call budget for approval before sending requests.

Keep existing prompt versions and stored request hashes reproducible. Do not
silently modify a frozen prompt, historical result, or completed experiment.
Do not change clinical-looking gates merely to increase synthetic coverage.

## Baseline and evidence

- The default molecular request is `molecular-origin-v6`; the current comparison
  command defaults to v3 versus v5. See `crates/josh-core/src/molecular.rs` and
  `crates/josh-app/src/prompt_comparison.rs`.
- A v6 full request has five additional boundary questions. Masked explanation
  requests omit them. The archive stores the original inference, but the generated
  full-evidence masked request has a different hash, so it can be evaluated again
  and the reported full value need not be the saved value. See
  `crates/josh-explain/src/lib.rs` and `prepare_versioned` / `prepare_masked_versioned`
  in `crates/josh-core/src/molecular.rs`.
- Comparison metadata states `automatic_retry: false`, while the shared client
  retries HTTP 429 and 529 up to twice. See `crates/josh-jev/src/lib.rs` and
  `crates/josh-app/src/prompt_comparison.rs`. Distinguish a logical evaluation,
  an HTTP attempt, a confirmed provider response, and uncertain completion.
- The context check estimates one token per four UTF-8 bytes. It can underestimate
  token use; it is not proof a request fits. See `within_model_context` in
  `crates/josh-core/src/molecular.rs`.
- The evaluator requires truth to be a named biological class, so its current
  labeled metrics cannot directly evaluate cases whose reference outcome is
  insufficient evidence or an unlisted origin. See
  `crates/josh-features/src/evaluation.rs`.
- The documented v5 synthetic development run had zero gate coverage across
  ten cases, and an intended unlisted-origin case returned insufficient evidence.
  These are v5 observations, not v6 or real-patient performance estimates. See
  `docs/reports/cup-samples-jev-v5.md`.
- `docs/references/JEV_WORKFLOW.md` describes v5 as current even though the README
  and implementation use v6. Historical reports should remain clearly dated.

The reviewer started `cargo test --workspace --locked --offline`, but stopped it
during dependency compilation. No current test pass or failure was established.
No live calls or application changes were made during this review.

## Phase 0 — establish a reproducible baseline

**Owner:** Rust developer. **Dependencies:** approved plan.

1. Record the branch, commit, working-tree status, toolchain, and local environment.
   Do not overwrite uncommitted work or existing run archives. Read the current
   CI workflow and these references: `README.md`, `docs/references/JEV_WORKFLOW.md`,
   `docs/evaluation/feasibility-protocol.md`, and this handoff.
2. Run the locked offline workspace tests and required CI checks where dependencies
   are available. Record failures and environmental limitations separately. If
   compilation overloads the machine, use a modest Cargo job limit, for example
   `cargo test --workspace --locked --offline -j 2`.
3. Prepare representative small, typical, and large **synthetic** inputs and
   identify which fixtures and archived examples can be used without network access.
   Record current request size, logical evaluation count, and wall-clock measurements;
   repeat timings enough to identify variance. Do not publish these as clinical results.

**Exit:** baseline commit, checks, environment, and measurements documented; any
existing failures classified before changes begin.

## Phase 1 — explanations reproduce the intended saved result

**Owner:** Rust developer. **Priority:** highest. **Dependencies:** Phase 0.

1. Trace archive construction, coalition generation, request hashing, validation,
   caching, exact/permutation attribution, and both masked-evidence and reference-
   background modes. Confirm behavior for v2–v6 and existing completed/partial archives.
2. Design a versioned explanation behavior in which the saved inference is the
   full-evidence anchor and every newly generated coalition uses the same question
   definitions as that saved request. In particular, a v6 coalition must preserve
   its original boundary questions while hiding evidence correctly. Do not mutate
   the original v6 inference prompt or invalidate its existing request hash.
3. Reuse the saved response for the full coalition without a provider call. Keep
   coalition identity, question set, source, and provenance explicit. Do not assume
   separate provider calls are deterministic: record that masked evaluations may
   vary and that attribution is not causality or a calibrated clinical probability.
4. Version the explanation/archive semantics where needed. Older archives must
   remain readable and verifiable under the behavior that produced them; never
   silently recalculate or relabel existing results. Clearly label any historical
   v6 explanation whose full coalition used a fresh, three-question evaluation.
   Define safe behavior for incomplete archives and explicit migration/restart.
5. Add focused tests with an evaluator that returns different values on successive
   calls. Verify the saved full value is reused, no redundant full call occurs,
   all new coalitions retain identical questions, hidden features and derived
   inventory do not leak, and exact/permutation additivity and background behavior
   remain correct. Cover v2–v6 and historical archive verification.

**Exit:** regression tests demonstrate that a new explanation's full value equals
the saved inference value; no extra full-coalition call is needed; archive and
masked-feature compatibility checks pass. Document any irreducible provider
variation and limitations of older archives.

## Phase 2 — compare the actual deployed prompt

**Owner:** Rust developer. **Dependencies:** Phase 0; can be developed alongside Phase 1.

1. Add explicit CLI selection of baseline and candidate prompt versions. Default
   a new comparison to v5 versus the current v6, while allowing historical v2–v5
   comparisons. Validate versions before making files or calls and show both in
   help, preview, protocol, and results.
2. Freeze model, taxonomy, input/request hashes, order, partitions, retry policy,
   budget, and version pair in each protocol. Preserve label isolation from provider
   state and patient grouping across partitions. Keep failed and unrun cases visible.
3. Add offline tests for default and explicit version pairs, request hash stability,
   invalid versions, grouping/leakage checks, and preservation of historical prompt
   preparation. Reopen existing comparison artifacts without changing their meaning.
4. Update `docs/references/JEV_WORKFLOW.md`, `docs/evaluation/PROMPT_COMPARISON.md`,
   command help, and relevant user-facing references. Mark historical v5 reports
   as historical instead of editing their observations.

**Exit:** an offline preview visibly produces v5 and v6 requests for identical
inputs; explicit historical pairs work; archived comparisons remain interpretable;
tests and docs agree with actual behavior.

## Phase 3 — trustworthy limits, retries, and accounting

**Owner:** Rust developer. **Dependencies:** Phase 0; coordinate with Phase 2.

1. Make provider retry behavior explicit and configurable for each workflow.
   Decide whether experiments disable retries or record every HTTP attempt. Ensure
   execution metadata reflects the actual policy; never describe a retrying client
   as having retries disabled. Verify provider semantics before assuming a status
   proves a request was unaccepted or unbilled.
2. Preserve uncertain-completion checkpoints. Account separately for logical
   evaluations, actual HTTP attempts, confirmed usage, and conservative reservations
   for uncertain/failed calls. Test the interaction of retry, time, token, and call
   limits. Never silently resend after an ambiguous timeout.
3. Inspect the provider's current documentation for token counting and context
   limits. Use an authoritative tokenizer/counting facility if available; otherwise
   surface the estimate and uncertainty clearly, provide a defensible conservative
   guard or warning, and retain provider rejection as a possible outcome. Do not
   silently truncate evidence. Do not represent byte reservations as actual billing.
4. Add deterministic local HTTP tests for 429/529, retry delays, exhausted attempts,
   transport failure and timeout, large/malformed response, context boundaries,
   and cancellation. Avoid network-dependent unit tests.

**Exit:** tests show records and limits match real attempts and failure states;
users can distinguish estimated, reserved, and provider-reported token counts;
no automatic retry of uncertain completion occurs.

## Phase 4 — synthetic evaluation and repeatability

**Owner:** Rust developer and research lead. **Dependencies:** Phases 2–3.

1. Extend evaluation contracts to represent independently specified named-class,
   unlisted-origin, and genuinely indeterminate reference outcomes without assuming
   they are interchangeable. Define eligibility and metrics for each outcome before
   implementation; avoid forcing unknown truth into a named class. Keep patient-group
   separation and all failures/abstentions in the relevant denominators.
2. Add hand-calculated tests for confusion matrices, top-k metrics, coverage,
   selective error, calibration measures where meaningful, and failure accounting.
   Label conditional metrics as conditional. Clearly distinguish the full gate
   from curves based only on a top-score threshold.
3. Prepare a frozen, paired v5/v6 synthetic protocol with straightforward cases,
   overlapping classes, insufficient evidence, conflicts, and unlisted origins.
   Include genomic-only and pathology-only views with paired patient grouping.
   Report rankings, gate outcomes, boundary answers, provider failures, runtime,
   token usage, and repeated-input variation. Synthetic design intent is not
   independently adjudicated clinical ground truth.
4. Before live execution, propose the exact manifest, number of calls and repeats,
   timeout, retry policy, input/output token and monetary limits, and stop rules.
   Obtain approval. Preserve every attempt and present all results, including
   adverse or inconclusive results. Offline preparation alone needs no provider call.

**Exit:** evaluation handles the stated outcome types with tested denominators;
the protocol and offline preview are reproducible. If live execution is approved,
publish a dated report with all outcomes and explicit limitations; otherwise mark
the live portion pending rather than imply it passed.

## Phase 5 — independent accuracy and calibration study

**Owners:** research/statistics lead, clinical/pathology reviewers, and data steward.
**Dependencies:** an approved protocol, eligible data and provider arrangements;
software phases can complete while this remains pending.

1. Define intended use, population, prediction-time cutoff, class taxonomy,
   reference standards, inclusion/exclusion criteria, endpoints, sample-size or
   precision rationale, and baseline comparators before inspecting the test set.
   Establish independent adjudication and record disagreements and limitations.
2. Use patient-separated development, calibration, and held-out test sets; keep
   specimens and modality views from the same patient together. Check for temporal,
   diagnostic-label, free-text, and site leakage. Predefine assay, institution,
   cancer-class, and relevant demographic strata and external validation.
3. Evaluate ranking, error, coverage, abstention, unresolved/unlisted outcomes,
   and uncertainty with confidence intervals appropriate to the sampling unit.
   Preserve missing, failed, and excluded cases with reasons. Separate modalities.
4. Fit or select any calibration and operating thresholds only on the designated
   calibration data, freeze them, then evaluate once on held-out data. Review
   clinical implications and decision criteria with the appropriate experts.
   No software test, synthetic benchmark, or vendor calibration claim substitutes
   for this study; no clinical use is authorized by this plan.

**Exit:** a reviewed frozen protocol and data/provider eligibility decision exist.
Completion of the scientific study requires its own recorded results and review;
do not mark it complete merely because the software supports it.

## Phase 6 — profile and improve efficiency

**Owner:** Rust developer. **Dependencies:** baseline from Phase 0; repeat after
correctness changes in Phases 1–3.

1. Measure local request preparation, import, archive handling, explanation
   generation, peak memory, request bytes, logical/provider attempt counts, token
   usage, and end-to-end latency on representative synthetic workloads. Separate
   local CPU time from provider latency and report timing variability.
2. Prioritize measured bottlenecks. Check coalition deduplication and reuse first;
   investigate serialization, allocations, and I/O only if profiling supports it.
   Assess request-size and additional-question costs against observed behavior.
3. Make small, measurable changes without weakening validation, data isolation,
   uncertainty handling, or reproducibility. Do not introduce concurrent provider
   calls unless budgets, cancellation, rate limits, uncertainty, and ordering are
   explicitly accounted for. Do not cache across requests unless the full request,
   model, prompt, and provenance semantics make reuse valid.
4. Repeat the same benchmark on the same hardware and inputs, report before/after
   measurements and uncertainty, and retain changes only when evidence justifies
   them. Do not claim improved accuracy from lower latency or cost alone.

**Exit:** a reproducible benchmark report identifies changes, measured impact,
and any tradeoffs. No optimization is required where no bottleneck is demonstrated.

## Final verification and handoff

Run the repository's CI-equivalent checks, subject to local dependency availability:

```bash
cargo build --workspace --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline
python3 scripts/dependency_inventory.py --check
```

Also run focused tests for every changed path, verify old saved-run and explanation
fixtures, review generated CLI help and docs, and inspect `git diff` and `git status`.
Report the commit, checks and failures, archive compatibility, benchmark results,
live-call approval/usage if applicable, and any pending scientific decisions. Do
not imply the independent validation phase is complete if data or review is pending.
