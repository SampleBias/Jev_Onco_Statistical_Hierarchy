# Expression reference guide

## What is implemented

A local reference contains fixed canonical genes and per-class mean transformed
expression, source/member checksums, separate known-origin labels, patient grouping,
compatibility metadata and fixed overlap thresholds. Every class is compared on the
same observed genes using signed Pearson correlation. Jev remains the sole origin
classifier: comparison is numerical evidence, never a replacement classifier.

Only bulk human TPM with the explicit log2-one-plus transform is eligible in this
first comparison implementation. Query and reference must match platform/pipeline
label, genome declaration (including unknown), organism and exact HGNC dictionary
hash. This is a conservative metadata gate, not cross-platform harmonization or
proof that batches are biologically comparable. Counts/FPKM/microarray data remain
importable, but cannot enter this comparison path. Scientific cohort selection,
evaluation, reference representativeness and cancer-label review remain open.

## Build a reference from known-origin expression

Import a dataset using the Expression Guide, including the correct platform:

```bash
josh dataset import known-primary.tsv --dataset-id known-v1 \
  --units tpm --transform log2-one-plus --platform study-pipeline-v1 \
  --gene-map data/gene-maps/hgnc-complete-2026-09-22.tsv \
  --gene-map-release hgnc-2026-09-22 --out-dir results/known-v1
```

Prepare a separate UTF-8 TSV with EXACTLY these columns. Each reference sample needs
one row, and each patient group must occur once (curate repeat samples/aliquots
before building). Multiple samples per cancer class are expected. This initial
builder accepts one dataset bundle; it does not join heterogeneous cohorts.

```text
sample_id   patient_group_id   class_id   cancer_type
KNOWN-01    GROUP-01           luad       Lung adenocarcinoma
KNOWN-02    GROUP-02           coad       Colon adenocarcinoma
```

Use tabs, not aligned spaces. Labels must exactly cover the bundle samples. Names
must be consistent within classes; class IDs/names must be unique. unknown and
other_origin are reserved. Labels must come from reviewed known-primary metadata,
not from a prior JOSH prediction. These example lines describe a format, not a
provided cancer reference cohort.

```bash
josh reference build results/known-v1 --labels known-labels.tsv \
  --release-id known-primary-v1 --citation 'Study accession or publication' \
  --min-genes 100 --min-overlap 0.8 --output results/reference-v1.json
josh reference inspect results/reference-v1.json --format text
```

The builder re-verifies and reproduces the source dataset. Blocked QC, absent
mapping, inconsistent labels, repeated patients and mixed synthetic/research data
are rejected. Genes must have mapped finite measurements in EVERY reference sample;
this intersection is frozen before a query is seen. Each class mean uses only its
own reference samples. No query labels or measurements influence the reference.

Limits: 2–100 classes, 512 reference samples, 100,000 genes and one million centroid
values; reference JSON at most 32 MiB. Minimum genes must be at least 100 for research
and at least 3 for synthetic tests; minimum overlap is 0.5–1.0. These are engineering
limits/defaults, not validated scientific operating points.

## Compare a query and prepare Jev evidence

Import the query independently with the same actual assay/processing settings and
gene dictionary. Then:

```bash
josh reference compare results/reference-v1.json --dataset results/query-v1 \
  --sample CUP-001 --output results/comparison-001.json
josh reference prepare results/reference-v1.json --dataset results/query-v1 \
  --sample CUP-001 --output results/jev-request-001.json
```

`compare` returns a structured EvidencePackage. All classes use the same intersected
query/reference genes, with missing expression excluded explicitly. The frozen
minimum-gene and overlap requirements are applied before correlations are emitted.
Values remain in [-1,1]; no softmax, probability conversion or top-N renormalization
is applied. Constant/degenerate query or class profiles produce undefined_similarity
and block request preparation. The full class list is retained, including negative
correlations. Equal scores use class ID ordering for reproducible display.

Query/reference source hash, sample ID, measurement hash and available patient group
are checked for overlap. Matching identity/source is rejected. Patient linkage is
NOT inferred from names, expression values or filenames; absent query group metadata
is reported as a limitation. Dataset import does not yet join patient annotation
files, so external group-separation review remains necessary. Hash checks do not
prove there is no duplicated tumor under a different identifier or source format.

`prepare` first recomputes the comparison. On success it creates a bounded (32 KiB)
Jev Choice over the reference class IDs plus unknown/other_origin and independent
Noul questions for sufficiency and contradiction. The exact request and SHA-256,
model/prompt version, complete comparison evidence and `sends_to_provider:false`
are exported. Query identifiers, patient IDs, file paths and labels are excluded
from the provider state. No key is read and no network call is made.

Reference preparation produces the typed Choice/Noul request offline. For synthetic
live inference, attach a usable comparison to Analysis with Data's m key. Molecular
response handling and archives are available; calibrated CUP probabilities remain
unavailable. See the [Molecular Guide](MOLECULAR_GUIDE.md).

## TUI workflow

In Data (F3), r opens reference JSON, a compares the selected sample, and Analyze shows signed
correlations, sample counts, overlap, gate failures and scientific limitations.
s on Analyze exports the complete evidence package; e exports the offline request
with evidence. Open g to search this guide; F1/Ctrl+g work inside forms. Loading a
new dataset/reference or switching samples invalidates the previous comparison.
The shared Analysis result remains intact until you explicitly attach another
usable comparison with m or replace its input. Comparison and request export do
not call Jev.

## Reproducibility and interpretation

Keep the reference dataset bundle, exact label TSV, dictionary and reference JSON.
The release records source manifest/input/label hashes, member measurement hashes,
patient grouping, fixed genes, processing configuration and class sizes. Comparison
records query source/measurement hashes, exact reference-file hash, compared gene
IDs, all scores and the pipeline version. Rerunning on unchanged artifacts produces
the same numerical evidence. Creation timestamps differ when rebuilding a reference;
archive the original reference release to reproduce its exact file hash.

A reference file is validated structurally but is not cryptographically signed. An
independently edited reference needs independent curation review. Provenance hashes
are traceability, not scientific approval.

Conflict assessment is not_assessed_expression_only. Missing modalities are listed;
absence is not a negative assay. OOD is not_validated. A high correlation does not
establish tumor origin, cancer-specific discrimination, clinical utility or confidence.
No real cancer cohort or validated cancer reference is bundled with this release.
Synthetic fixture classes are named Demonstration A/B and cannot support biological
claims. The roadmap still requires reference curation, group-separated held-out
experiments, representative provider repeatability studies and calibration. Local
transport tests and a limited live synthetic check have been completed; see the
[0.7.0 report](../reports/0.7.0-molecular-validation.md).
