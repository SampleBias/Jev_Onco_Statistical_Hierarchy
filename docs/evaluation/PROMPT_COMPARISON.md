# Comparing Jev request versions

The current CLI compares **v3 versus v5**. The current request adds specimen/assay
interpretation, locally computed visible-evidence counts, class boundaries and
shorter lineage-specific yes/no questions. The
[Jev workflow guide](../references/JEV_WORKFLOW.md) and
[ten-case sample pack](../../fixtures/cup-realistic-v2/README.md) describe the
current preparation and bounded comparison. v2 and v3 remain reproducible.

The following describes the original v3 change and historical preparation example.
`prepare_prompt_comparison` explicitly freezes v2/v3 and uses the original five
fixtures. Invoking `molecular compare-prompts` on its manifests now compares the
current v3/v5 pair; always inspect `protocol.json` for the executed versions.

`molecular-origin-v3` adds explicit measurement meanings and outcome rubrics to
the existing three-question request. `molecular-origin-v2` remains available for
archive verification, explanations and paired comparison. The model remains
`jev-1.13.0`; class IDs, engineering abstention thresholds and probability checks
are unchanged. These rubrics have not undergone independent pathology review or
demonstrated improved cancer accuracy.

## What changed

The request retains original observations and adds English meanings to discrete
CNA calls (-2 deep loss, -1 loss, 0 neutral, +1 gain, +2 high-level amplification).
These are ordinal calls, not absolute copy counts. Age censoring is explicit.
Unknown, not-tested and withheld measurements remain unavailable, never negative.
Masked features reveal neither values nor their derived descriptions.

Choice options now require support for the named class over alternatives;
insufficient evidence is distinguished from a specifically supported unlisted
origin. The sufficiency and contradiction questions define their yes/no meaning
independently. We do not add unvalidated gene-to-cancer rules or turn raw scores
into calibrated cancer probabilities.

This follows the official [state guidance](https://docs.typesafe.ai/concepts/state),
[Choice guidance](https://docs.typesafe.ai/primitives/choice) and
[Jev limitations](https://docs.typesafe.ai/model-jaggedness/jev-1.13).

## Reproduce an offline preparation

From the project directory, choose a new output directory:

```bash
cargo run --locked -p josh-app --example prepare_prompt_comparison -- results/jev-v3-comparison
```

This creates five original synthetic inputs, separate full/genomic development
manifests, two paired request plans, and v2/v3 requests for the sparse sample.
It makes no provider calls. The first two scenario labels are invented design
intent from the fixture README, not independently established ground truth. The
sparse sample has no known origin label and is excluded from accuracy metrics.
Both tracks contain the same two patients and must not be split across partitions.

## Run a bounded live comparison

Set `TYPESAFE_API_KEY` in the launching process using your usual secret manager or
hidden shell input. A key entered inside the TUI is available only to that TUI
process. It cannot authenticate a separate CLI process.

```bash
./target/debug/josh molecular compare-prompts results/jev-v3-comparison/full-manifest.json \
  --out-dir results/jev-v3-comparison/full-live --execute \
  --max-evaluations 4 --max-input-tokens 100000 --max-seconds 120

./target/debug/josh molecular compare-prompts results/jev-v3-comparison/genomic-manifest.json \
  --out-dir results/jev-v3-comparison/genomic-live --execute \
  --max-evaluations 4 --max-input-tokens 100000 --max-seconds 120
```

Each command permits at most four new, potentially billed calls. Omit `--execute`
to prepare requests only. Use a **new** output directory for every invocation;
existing attempts are never automatically resubmitted. Real research inputs can
be prepared locally but live execution still enforces synthetic-only eligibility.

The runner alternates baseline/candidate order by case. It freezes feature and
request hashes, separates labels from provider state, validates patient-disjoint
partitions and saves the budgets before sending. Failed responses and unrun cases
remain in the eligible denominator. Unknown outcomes are retained in the full
distribution. `comparison.json` includes each version's top-1/top-3, F1, coverage,
Brier score, log loss and reliability, alongside paired predictions/failures.
Token accounting retains byte-based reservations for failed/uncertain calls; it
is not a billing measurement. A timeout can still leave a billed provider call.

These small synthetic comparisons test execution and behavior. To test cancer
performance, supply reviewed independent labels and patient-separated development,
calibration and test cohorts using the existing cohort manifest format. Freeze
the protocol before inspecting test results; compare modalities separately. No
calibration model or confidence interval is fitted by this command. Real-data
provider eligibility remains a prerequisite for live use on those cohorts.

## Diagnosing a rejected response

Failed inference runs save the precise reason in `run-status.json` and a
`response-diagnostic-<hash>.json` file. Explanations, cohort runs, repeated-input
experiments and prompt comparisons also retain these diagnostic files. A report
records a fixed error code, HTTP status where available, request/body hashes,
bounded body size, question index/counts, probability total, or JSON field and
parser coordinates. The question index refers to lexically sorted request IDs.

No raw rejected provider body, header or credential is stored. Response size is
capped at 64 KiB. A truncated/oversized body has no full-body hash. Numeric totals
are recorded without renormalizing probabilities; the 1e-6 sum tolerance stays in
force. The UI presents the specific reason, such as a probability total of 0.99,
instead of only “contract validation failed.” CLI error codes remain compatible;
inspect the run's diagnostic file for detail.

The historical failed run `josh-run-1790384302150566646` did not retain its rejected
response. Its exact cause cannot be recovered retroactively by these changes.
