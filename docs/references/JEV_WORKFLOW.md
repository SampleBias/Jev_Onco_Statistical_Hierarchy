# How JOSH uses Jev for CUP research

Checked against official TypeSafe documentation on 2026-09-26. The current
molecular request is `molecular-origin-v5`, using pinned `jev-1.13.0`.
This document is included in the TUI's existing searchable F1 guide.

## 1. Give the model interpretable evidence

The [new sample pack](../../fixtures/cup-realistic-v2/README.md) contains ten
invented metastatic workups. The importer validates typed values, units, status,
assay scope, source hashes and attribution groups before constructing a request.
It rejects inconsistent missing values and unsupported encodings.

OncoNPC motivates mutation counts, CNA, mutational signatures and demographics.
IHC, histology and contextual workup observations are this application's extension.
They must be evaluated as a separate input setting. See the
[authors' implementation](https://github.com/itmoon7/onconpc) and
[published paper](https://www.nature.com/articles/s41591-023-02482-6).

Jev receives structured JSON with semantic feature names and measured values.
The app expands CNA codes into meanings, identifies censored ages, clarifies
tumor-only versus reported somatic/germline annotations, and computes an evidence
inventory locally. The inventory counts supplied rows; it does not weight evidence.
Unavailable tests remain explicit. The model sees neither scenario candidates
nor patient IDs, source paths, local provenance hashes or evaluation partitions.
This follows TypeSafe's [state guidance](https://docs.typesafe.ai/concepts/state).

## 2. Ask three independent, bounded questions in one call

| Question | Primitive | Meaning in this application |
| --- | --- | --- |
| primary_site | Choice | Distribution over 22 named cancer classes plus insufficient_evidence and other_origin |
| evidence_sufficient | Noul | Whether interpretable evidence distinguishes a specific cancer class from alternatives |
| conflicting_evidence | Noul | Whether competing reliable observations support incompatible assignments |

Each question evaluates the evidence independently; it cannot read another
question's answer. A missing result is not a contradiction. A coherent supported
unlisted cancer can have sufficient evidence while still requiring other_origin.
Overlapping markers can justify insufficiency without implying an actual conflict.
These distinctions follow [independent questions](https://docs.typesafe.ai/introduction),
[Choice](https://docs.typesafe.ai/primitives/choice) and
[Noul](https://docs.typesafe.ai/primitives/noul).

The request adds bounded class distinctions for shared markers and neighboring entities,
including pancreatic/biliary, breast/urothelial, gynecologic and neuroendocrine
boundaries. They are research rubrics, not marker-to-diagnosis rules. Custom classes
reusing a standard ID but changing its name/group do not inherit those hints.
The rules are the same for all cases; scenario expectations are not embedded.

V5 asks shorter yes/no questions and explicitly separates evidence favoring an
origin from imaging actually locating a primary mass. This change followed a
development check in which the initial v4 rubric gave low sufficiency and excess
conflict to coherent panels. v4 is frozen for archive verification; the comparison
baseline remains the pre-change v3. See the [verification report](../reports/cup-samples-jev-v5.md).

The main Choice remains flat. Twenty-four options fit comfortably within the
[documented Choice limit](https://docs.typesafe.ai/primitives/choice).
A hierarchical cascade would introduce additional failure paths and conditional
scores; TypeSafe's [hierarchy cookbook](https://docs.typesafe.ai/cookbooks/hierarchical_classification)
does not establish a calibrated clinical posterior for such a cascade.

## 3. Keep numerical work in Rust

Rust performs import validation, row counts, probability checks, rankings,
threshold decisions, reference comparisons, statistical calculations and attribution
bookkeeping. Jev evaluates semantic evidence. It does not calculate mutation
burden, derive a signature from a few variants, fit calibration, invent RNA data,
or predict treatment benefit in this workflow.

TypeSafe describes weaknesses with counting, arithmetic, numeric representations,
indirection and irrelevant long context in its
[Jev limitations](https://docs.typesafe.ai/model-jaggedness/jev-1.13).
Consequently, full papers and whole raw variant dumps are not appended to each
patient request. Units, assay limits and concise contextual findings are included.
SBS and expression are unavailable in the new panel-only examples rather than
being invented as computed laboratory outputs.

## 4. Validate the response and preserve uncertainty

The client uses the documented HTTPS endpoint with bearer authentication, then
checks model identity, question and option IDs, answer types, probability ranges
and sum, and consistency of the selected option. See the
[HTTP contract](https://docs.typesafe.ai/api). Failed responses remain failures;
the app does not repair or renormalize them into successful predictions.

Current research gates abstain for an unresolved option, top score below 0.75,
margin below 0.15, sufficiency below 0.8, or conflict above 0.2. These pre-existing
engineering defaults are unchanged and have no established clinical operating
characteristics. Passing them gives `review_required`, never a clinical diagnosis.

The provider's `confidence` summarizes distribution concentration. It is not the
probability that this patient's diagnosis is correct. The origin distribution and
two Noul judgments are retained separately, not multiplied together. See
[TypeSafe confidence](https://docs.typesafe.ai/confidence).
The distribution also contains operational outcomes; it is not exclusively a
posterior over biological origins.

The launch blog's type-safety claim concerns output shape. Correctly typed values
can still represent an incorrect medical judgment. Vendor calibration claims are
not validation for this CUP dataset. See the vendor's
[launch discussion](https://typesafe.ai/blog/introducing-system-one-models-and-jev).

## 5. Compare before claiming improvement

Prepare the sample suite using the command in its README. The current
`molecular compare-prompts` command compares v3 against v5 on identical inputs.
For an explicitly executed, bounded synthetic comparison:

```bash
./target/debug/josh molecular compare-prompts results/cup-realistic-v2-v5/full-manifest.json \
  --out-dir results/cup-realistic-v2-v5/full-live --execute \
  --max-evaluations 10 --max-input-tokens 350000 --max-seconds 120
```

This makes at most ten billable calls: five scenarios, two versions. Omit
`--execute` for preparation only. Use new output directories. Genomic/pathology
manifests are paired views of the same patients; preserve their grouping. The
five ambiguous or unlisted scenarios are reviewed separately, not forced into
the comparison runner's known-class truth field.

Inspect score changes, insufficiency, conflict, rank, failures and request sizes.
In 003, inspect confident origin assignment from weak evidence; in 005 and 008,
inspect specificity beyond the supported lineage; in 009, inspect contradictory
specimens; in 010, inspect whether an absent cancer class is forced into a listed
one. Do not tune to ten handcrafted answers and then call those cases a test set.

Jev's [model documentation](https://docs.typesafe.ai/models) describes adaptation
through state and question criteria, not per-customer weight training. The app
pins the model version and request hashes. Ten synthetic patients test wiring and
failure modes; domain accuracy needs independently reviewed patient labels,
patient-separated development/calibration/test sets, assay/institution strata and
external validation. A larger justified experiment should compare actual baselines
on the same patients and retain failures/abstentions in denominators.

## 6. Explain the specific saved model output

The existing optional explanation workflow uses grouped, model-agnostic Shapley
evaluations. Every masked request uses its saved prompt version and unchanged
questions. Inventory fields are rebuilt from visible evidence so hidden names,
values, assays and modality counts are not leaked through derived metadata.
The full and masked requests/answers are archived for inspection and reuse.

With roughly ten groups per case, exact enumeration can be expensive. Start with
a bounded permutation explanation in the existing budget dialog and inspect its
sampling uncertainty. Feature contributions describe changes in raw Jev output,
not causality or validated clinical probabilities. More explanation calls do not
improve the underlying classifier. v2/v3 archives retain their original wording
and continue to verify and explain with that version.

The TUI layout, rendering, navigation and styling are unchanged by this work.
