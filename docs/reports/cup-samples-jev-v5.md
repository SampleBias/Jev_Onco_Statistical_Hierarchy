# Synthetic CUP samples and Jev integration verification

2026-09-26. See the [sample definitions](../../fixtures/cup-realistic-v2/README.md)
and [step-by-step Jev workflow](../references/JEV_WORKFLOW.md).

## Delivered

- Ten distinct synthetic patients, 227 feature rows, loaded through the normal
  importer in **Open data → Samples**. No predictions are bundled with inputs.
- Explicit positive/negative/unknown/not-tested findings, panel coverage,
  specimen quality, workup limitations, paired-specimen conflict and taxonomy gaps.
- Versioned semantic interpretation of gene counts, CNA, somatic annotation,
  specimen context and shared markers; deterministic visible-evidence inventory.
- Short independent yes/no questions that distinguish evidence for an origin
  from imaging actually locating its primary mass. The model remains pinned to
  `jev-1.13.0`; the current molecular prompt is `molecular-origin-v5`.
- Thirty full/genomic/pathology input views and sixty v3/v5 request previews,
  with local-only scenario metadata and hashes. Views retain patient grouping.
- Existing layout, styles, controls and charts preserved. New documentation is
  available through the existing F1 guide. The previously requested Menu/Open
  toolbar arrangement is preserved.

## Live development check

Twenty-five requests completed against hosted Jev: five v3 baseline requests,
ten initial v4 requests and ten final v5 requests. Every completed response passed
the strict response validator. Provider-reported usage totaled **130,572 input
tokens and 6,618 output tokens**. No automatic retries were enabled.

Ten earlier connection attempts were blocked by the sandbox and remain archived
separately under `results/cup-realistic-v2/full-live/`. Its reserved token count is
not measured usage. After network access was granted, the successful comparison
used a new directory. No failed archive was overwritten or converted to success.

The initial v4 state/Choice additions were retained, but its long yes/no rubrics
flagged conflict in all ten cases and gave low sufficiency to coherent panels.
V5 explicitly separates absence of a localized mass from evidence favoring a
lineage and shortens those questions. It flags conflict only in case 009. These
are development observations on constructed cases; the causal contribution of
individual wording changes has not been isolated by repeated experiments.

### Final v5 observations

Scores below are raw provider outputs, **not patient-level cancer probabilities**.
Each row is one call, not a repeatability estimate. Every case remained abstained.

| Case | Highest-scoring outcome | Raw score | Sufficiency | Conflict |
| --- | --- | ---: | ---: | ---: |
| 001 | NSCLC | 0.95 | 0.75 | 0.11 |
| 002 | COADREAD | 0.93 | 0.72 | 0.12 |
| 003 | insufficient_evidence | 0.99 | 0.40 | 0.11 |
| 004 | BRCA | 0.79 | 0.72 | 0.12 |
| 005 | OVT | 0.50 | 0.64 | 0.14 |
| 006 | PRAD | 0.93 | 0.73 | 0.12 |
| 007 | RCC | 0.95 | 0.79 | 0.12 |
| 008 | insufficient_evidence | 0.48 | 0.65 | 0.17 |
| 009 | insufficient_evidence | 0.93 | 0.52 | 0.50 |
| 010 | insufficient_evidence | 0.76 | 0.68 | 0.20 |

The five clearer cases ranked their author-intended class first in v3, v4 and
v5. This does not demonstrate improved cancer discrimination. Their final
sufficiency values of 0.72–0.79 remain below the existing 0.8 gate. Thresholds were
not lowered to make the examples pass. There is therefore **zero assignment
coverage in this ten-case v5 run**, despite several strong Choice rankings.

Case 010 did not meet its author-intended `other_origin` behavior: Jev returned
0.20 for other_origin and 0.76 for insufficient_evidence. This is an unresolved
taxonomy/abstention distinction, not evidence that the system reliably detects
every unlisted cancer. The sample intentionally keeps this failure visible.

## Reproducibility and software checks

- Original v2, v3 and initial v4 requests retain golden request hashes. Old runs
  can still verify against their saved prompt version; explanations use that
  version's questions. V4 was not silently rewritten after the live experiment.
- Tests cover all ten imports, response validation and archive roundtrips,
  all three evidence views, label isolation, unavailable versus measured-zero
  values, hidden-feature leakage, and custom taxonomy meanings.
- All 235 workspace tests, formatting, all-target Clippy with warnings denied
  and the dependency inventory check passed. No dependencies were added; the
  inventory remains 278 packages. Mock HTTP tests require loopback sockets.
- Both the new preparation example and the historical v2/v3 example completed
  offline. The historical example still uses the original five fixtures.
- A final v5 live archive and an initial v4 live archive both reopened and
  exported Markdown offline through the normal application workflow.

Local artifacts from this run:

- `results/cup-realistic-v2-v5/`: final input views, v3/v5 previews, hashes and
  development comparison plans. Preparation itself makes zero provider calls.
- `results/cup-realistic-v2/full-live-network/`: successful v3/v4 paired runs.
- `results/cup-realistic-v2/stress-003-live/` (and 005/008/009/010): initial v4
  stress-case runs.
- `results/cup-realistic-v2/final-001-live/` through `final-010-live/`: final v5
  runs, directly loadable in the TUI, including features, request and inference.
- `results/cup-legacy-v2-v3-check/`: historical example verification.

These generated results are ignored by Git. The TSVs, scenario manifest,
preparation example, documentation and tests are repository files. Original
requests and complete accepted distributions remain in the local archives.

## What remains unestablished

This is a feature-level engineering suite, not a representative patient cohort.
No real sequencing assay was run, no signatures were fitted and no treatment or
survival outcomes were invented. Gene counts omit allele-level specificity.
Independent pathology review, external evaluation, empirical calibration,
operating-threshold selection and model repeatability remain necessary before
making clinical performance claims. Genomic-only and pathology-only requests were
prepared and validated locally, but were not sent during this bounded live check.

The next experiment should freeze v5 and compare it with v3 on independently
reviewed development cases, including inconclusive and unlisted cancers, then
assess gates on a separate calibration set. A held-out evaluation must include
failures and abstentions and must not reuse paired patient views across splits.
