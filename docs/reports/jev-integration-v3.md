# Jev integration v3 implementation and verification

## Delivered

- Precise, sanitized response diagnostics for JSON shape, model, question/option
  IDs, probability ranges/totals, confidence and chosen-option consistency.
  Failed runs retain request/body hashes and numeric metadata, never raw rejected
  provider text, credentials or headers. Automatic retries remain disabled.
- Versioned `molecular-origin-v3` requests with explicit CNA and missingness
  semantics and independent outcome/sufficiency/contradiction criteria.
  Probability validation remains strict; no silent normalization was added.
- Original v2 archive verification and explanation requests remain reproducible.
- Bounded paired v2/v3 comparisons with frozen request hashes, isolated labels,
  patient-separated partitions and failures retained in eligible denominators.

The existing full-name header and masked session-key entry changes were preserved
and their regression test passed. A TUI key remains in that process only.

## Executed checks

- Workspace tests: 230 passed, zero failed.
- Workspace build, formatting and all-target Clippy with warnings denied passed.
- Dependency inventory check passed: 278 packages; no dependencies added.
- Existing checkpoint `josh-run-1790384074419634835/checkpoint.json` opened
  successfully offline with its 23 completed evaluations preserved. No uncertain
  provider call was resumed or retried.
- Prepared `results/jev-v3-comparison` from the bundled synthetic examples: full
  and genomic paired plans, sparse-case requests and a separate CLI preparation.
- An explicitly injected offline response totaling 0.99 produced the expected
  `probability_sum` diagnostic with numeric total, hashes and HTTP-status metadata.
  This fixture is not a live Jev result.

## Not established

No live provider comparison was executed: the command-line environment has no
`TYPESAFE_API_KEY`, and the running TUI's private key was not accessed. No claim
of improved cancer accuracy or calibration follows from these checks. Scenario
labels are invented fixture intent, not independently reviewed ground truth.

The rejected response from `josh-run-1790384302150566646` was not retained by the
old implementation. Its precise cause remains unknown. New failures will retain
the diagnostic information needed to distinguish causes.

See [comparison instructions](../evaluation/PROMPT_COMPARISON.md) for bounded live
commands and scientific validation limits. Restart JOSH with `./scripts/start`
to load the rebuilt application; re-enter the session key through the menu.
