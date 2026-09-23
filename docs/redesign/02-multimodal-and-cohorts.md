# Redesign Phase 2 — Variants, IHC and cohort analysis

Status: partially implemented in 0.7.0 under the approved
[OncoNPC adaptation](04-onconpc-jev-explanations.md). Canonical molecular tables,
documented MAF/VCF subsets, same-sample joins, Jev inference and bounded synthetic
cohort runs are available. Curated modality validation, richer variant/IHC
normalization, cohort TUI and PDF remain open. See the
[molecular guide](../data/MOLECULAR_GUIDE.md) for exact limits.

Depends on [Phase 1](01-samples-expression-and-reference.md)
sample/artifact/reference contracts and pipeline. Leads: bioinformatician and Rust
backend engineer. Reviewers: molecular pathology curator, statistics lead and QA.
Fixed stack: Rust, Jev and Ratatui. Direction: [gap assessment](../assessment/DATA_FIRST_GAP_ASSESSMENT.md).

## Outcome

Import mutation and IHC data into the same Sample model; analyze each supported
modality alone or in combination; make disagreements visible; run reproducible
cohort jobs and export complete machine-readable results plus a PDF summary.

| ID | Owner | Work package and artifact |
| --- | --- | --- |
| R2-01 | Data engineer | VCF, MAF and simplified mutation-table adapters with sample/assay mapping, parse reports and original annotations |
| R2-02 | Bioinformatician | Canonical variant representation, build/coordinate/allele policy, annotation versions and coverage-aware feature summaries |
| R2-03 | Pathology curator + backend | Typed IHC markers, qualitative results, intensity, percentage and H-score, assay details and aliases |
| R2-04 | Bioinformatician | Modality-specific reference comparisons and documented molecular gene-set/pathway features where supported |
| R2-05 | ML + backend | Availability-aware EvidencePackage, source-linked supporting/conflicting signals and per-modality assessments |
| R2-06 | Backend | Cohort definitions, bounded/resumable inference jobs, cancellation, failure accounting and duplicate submission protection |
| R2-07 | Terminal UX | Cohort/sample tables, comparison groups, disagreement view, feature exploration and exploratory clustering |
| R2-08 | Backend | Complete cohort JSON/CSV/TSV exports and Rust-generated PDF report from the same archived result |
| R2-09 | Statistics lead + QA | Modality ablations, multimodal discordance, missingness/coverage and external shift evaluation |

## Variant import and QC

Retain sample and tumor/normal roles, caller, filters, genome, chromosome, position,
reference/alternate alleles, transcript, consequence, protein change, available
depth/allele fractions and source annotation. Define coordinate conventions and
multi-allelic handling explicitly. Gene and effect fields may require a separate
versioned annotation asset; do not assume all VCF files contain them.

Do not declare a variant somatic simply because its filename ends in `.vcf`.
Preserve declared status and supporting source pipeline; report unknown status.
MAF adapters must be dialect-aware. For example, GDC documents protected and masked
somatic MAFs with different filtering and information retention, so they cannot be
treated as identical observations.
[GDC MAF specification](https://docs.gdc.cancer.gov/Data/File_Formats/MAF_Format/),
[GDC VCF documentation](https://docs.gdc.cancer.gov/Data/File_Formats/VCF_Format/).

Normalize variants only with the appropriate reference context; never silently mix
GRCh37/GRCh38. Pin genome/transcript/annotation assets and record transformation
parameters. Unsupported symbolic alleles/structural variants remain explicitly
unsupported records for Phase 3 rather than malformed SNVs.

QC includes accepted/rejected/filter counts, annotation completeness, missing build,
duplicate records, sample identity and assayed coverage. An absent variant is not
automatically wild type. Mutation burden requires a defined callable territory and
counting/filter policy; without those, report a variant count and mark burden
unavailable. Hotspot/driver annotations require versioned evidence assets; do not
invent driver status from a gene name.

## IHC and molecular evidence

Support positive, negative, weak, moderate, strong, equivocal, unknown and not-tested
values while retaining the original result. Model qualitative status and intensity
as separate concepts when the source provides them. Weak/moderate/strong alone must
not silently imply the same positivity rule across assays.

Record percentage-positive and H-score with explicit units/ranges, method, assay,
specimen/timepoint and provenance. Percentage and H-score are not interchangeable.
Use a reviewed marker vocabulary; an IHC marker alias is not automatically identical
to an RNA gene measurement. Preserve conflicting repeated observations rather than
resolving them by last-write-wins.

Define modality-specific evidence and comparison results, each with method, metric,
reference, feature/source IDs and QC. A signal can be supporting, opposing,
non-specific or unavailable. Conflicts must identify actual observations and
candidate differences; missing modalities alone are not contradictions.

Molecular pathway/gene-program scores require declared gene sets, method and assay
coverage with numerical verification. Never reuse the clinical-stage chart as if
it performed pathway analysis. Do not label simple membership in a mutated gene set
as a demonstrated functional pathway activation.

## Cohort workflow and evaluation

Implement immutable cohort membership and selection filters, sample/assay joins,
per-sample stage state, concurrency/cost budgets, retries, cancellation and resume.
Resume must not silently re-send completed requests. Every eligible sample remains
in the report, including QC blocks, transport errors and abstentions.

Provide group comparisons and exploratory clustering only on compatible normalized
features. Record algorithm, distance, parameters, seed, features and preprocessing.
Cluster membership is exploratory and is not an additional cancer classifier.
Display confounding by platform/study where metadata allows; do not infer an origin
from a cluster name assigned after seeing labels.

Evaluate expression only, mutation only, IHC only, expression+mutation,
expression+IHC and expression+mutation+IHC separately. Expose mutation+IHC through
the same capability contract when reference coverage supports it. Missing a modality
must permit supported remaining paths; no usable modality produces explicit UNKNOWN
or QC-blocked output. Compare raw Jev behavior, reference measurements and calibrated
results separately; do not pool incompatible evaluation populations.

Use group-separated known-primary references and held-out cohorts. Keep true CUP
evaluation separate when origin truth is unavailable. Measure confident errors,
selective risk/coverage and rejection under contradictions, assay shift and missingness.

## Exit criteria

- Real eligible VCF/MAF/IHC fixtures traverse adapters, QC, features, comparison and
  Jev without requiring expression or detailed patient data.
- Malformed/multi-sample/multi-allelic records, unknown annotations/builds, missing
  coverage and ambiguous marker status have explicit tested outcomes.
- A deliberate disagreement fixture displays each modality's evidence, source and
  reference result; the application never forces agreement.
- Cohort jobs are bounded, resumable and cancellable; exports reconcile selected,
  completed, failed, blocked and abstained samples.
- The TUI remains responsive during import/inference and permits per-sample drill-down.
- JSON contains the complete manifest/evidence; CSV/TSV and PDF agree on values,
  score semantics, run/version identity and unknown reasons.
- Modality-specific evaluation and appropriate calibration/abstention artifacts are
  published with compatibility checks and limitations before claims are expanded.

Handoff: new adapters and annotation contracts; IHC dictionary; multimodal evidence
schema; cohort job engine; exploration/reporting views; evaluation report. Next:
[Phase 3](03-modalities-and-connectors.md).
