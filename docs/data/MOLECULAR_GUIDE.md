# Molecular inference and explanations

JOSH implements molecular inference, model-agnostic Shapley explanations and
linked Ratatui charts in Rust. Jev remains the hosted origin classifier. The
experimental taxonomy has 22 detailed classes plus insufficient/other outcomes.
Live result probabilities are raw Jev outputs; analytical demos are explicitly
marked mock. No CUP calibration is installed.

## Try the graphics offline

```bash
cargo run --locked --bin josh -- tui
# d computes the analytical demo; Tab opens charts.
./target/debug/josh molecular demo --out-dir /tmp/josh-demo --format text
./target/debug/josh tui /tmp/josh-demo/explanation.json
./target/debug/josh molecular export /tmp/josh-demo/explanation.json \
  --kind svg --output /tmp/josh-figure.svg
./target/debug/josh molecular export /tmp/josh-demo/explanation.json \
  --kind csv --output /tmp/josh-contributions.csv
```

For the guided Load/Analyze/Results/Markdown workflow, see [quickstart](../QUICKSTART.md).
`molecular check FEATURES` checks local prerequisites. Each live run automatically
saves `report.md`; a completed explanation adds `explanation-report.md`.
`molecular export RUN_DIRECTORY --output NEW.md` exports offline and defaults to
Markdown. Use explicit --kind svg/csv/json with a completed explanation JSON file for chart/data
formats. The TUI chooses its export format by extension; the CLI uses --kind.
Inference JSON needs its matching features.json sidecar to reopen or export Markdown.

Choose new destinations. Demo measurements and coefficients are invented software
fixtures, not a replication of the paper's patient or a substitute cancer model.
Analytical scores are determined by the bundled demo function, not a trained model.

The donut displays magnitude; the scatter and table display sign. Category totals
use absolute contributions from the same attribution game. The waterfall shows
the baseline and signed steps. A feature group is never divided into invented
per-gene values. Top-ten SVG labels retain a visible signed/absolute remainder.

Press t to change the explained class offline. A reference-percentile scatter
axis is used only when the available background and all displayed numerical
features support it; otherwise X is feature rank. Original values/units remain in
the inspector. Narrow terminals fall back to a table and signed bars.

## Feature input and provenance

`molecular example` prints editable input JSON. `schema molecular-features` prints
the generated contract. Each record has a sample/patient-group association, typed
measurement, modality, assay, reference build, coverage, source hash/record, and
attribution group. Unknown/not-tested observations have null values; zero is a
measured value. Identifiers and source records remain outside the Jev state.

```bash
./target/debug/josh molecular example --output /tmp/molecular-features.json
./target/debug/josh molecular prepare /tmp/molecular-features.json
./target/debug/josh tui /tmp/molecular-features.json
```

Canonical CSV/TSV requires `sample_id,id,name,modality,value,units,status` columns.
Use `status=observed|unknown|not_tested`; unavailable values must be empty. Optional
columns: `assay,coverage,reference_build,group,gene,chromosome,position,reference,
alternate,consequence,somatic,catalogue,method,mutation_count,reference_sha256,
depends_on`. All rows must match the selected sample. Unknown headers, malformed
rows and conflicts reject the whole import; no partially accepted sample is sent.

- `mutation`: coordinates/gene/alleles/consequence for a variant, or an explicit
  integer count with units. Somatic status is true, false or unknown.
- `copy_number`: gene plus discrete call -2..2 and `units=discrete_call`, or an
  explicitly named quantitative scale. No conversion threshold is guessed.
- `signature`: SBS ID in `name`, nonnegative value, units, catalogue, method and
  mutation count. Fractions from one assay/catalogue must share a group and total
  at most one. Projection scores, fractions and fitted counts are distinct.
- `demographic`: age with years (including > bounds), or a categorical value.
- `expression`: numerical values, or an inseparable reference vector.
- `ihc`/`histology`: bounded categorical observations; no inferred positivity.

```bash
./target/debug/josh molecular import input.tsv --input-format tsv \
  --sample S1 --patient-group P1 --source-id assay-export --assay panel-v1 \
  --reference-build GRCh38 --synthetic --output /tmp/features.json
```

The MAF subset requires Hugo_Symbol, Chromosome, Start_Position, Reference_Allele,
Tumor_Seq_Allele2, Variant_Classification, Tumor_Sample_Barcode and NCBI_Build.
Mutation_Status is optional; absent is unknown. Only explicit GRCh37/38 coordinates
and A/C/G/T alleles are accepted. Unanchored `-` alleles need upstream normalization.

The VCF subset requires exactly one selected sample, PASS biallelic records,
an alternate GT, and explicit INFO/GENE and INFO/CONSEQUENCE annotations. SOMATIC
is optional; absent is unknown. A build must be supplied. Symbolic, multiallelic,
unannotated and unsupported genotypes are rejected. No automatic annotation or
left-alignment is performed. Original source bytes are identified by hash; retain
the input file alongside the exported feature set.

Join same-sample feature sets with `molecular merge`. Attach existing compatible
expression evidence with `molecular attach-expression FEATURES EVIDENCE`. Duplicate
feature IDs fail. After expression comparison in the workbench, m carries the
sample and exact reference classes into the shared Analysis section (F2), replacing
its previous input/result. Switching with F2 alone preserves the current analysis.

## Jev runs and explanation budgets

Configure TYPESAFE_API_KEY in the process environment. It is not loaded from .env
or saved in archives. Live calls retain the existing synthetic-only restriction.

```bash
./target/debug/josh molecular run /tmp/features.json --out-dir /tmp/inference-run
./target/debug/josh molecular plan /tmp/inference-run --target NSCLC --method exact
./target/debug/josh molecular explain /tmp/inference-run --target NSCLC \
  --method permutation --pairs 32 --seed 42 --max-evaluations 1024 \
  --max-input-tokens 2000000 --max-seconds 600 --format text
```

Inference validates against the request's actual classes. A custom versioned
taxonomy can be passed with --taxonomy. Genomic-only inference needs no RNA
reference. Separate sufficiency/conflict questions feed explicit abstention using
engineering thresholds; those thresholds are not clinical operating points.

Without --background, explanations use explicitly named masked-evidence Shapley:
hidden observations have no values or revealing summary text. The baseline is
the model's response to withheld evidence, not a population expectation. With
--background FILE, inputs are substituted from a compatible, patient-disjoint,
development-only Background contract. The background is uniformly weighted;
repeated patient groups are rejected. No silent baseline fallback occurs.

Groups sharing dependencies or compositional measurements move together. Exact
enumeration supports at most 16 groups and may exceed the selected budget.
Permutation mode uses seeded forward/reverse order pairs. Sampling SE is computed
over those pairs; it excludes provider variability and background uncertainty.
Both methods check `baseline + contributions = full score` in probability units.

Successful responses and pending requests are checkpointed atomically. A lock
prevents concurrent explanation jobs in one directory. Resume with the same
mathematical configuration; budgets may be increased. An interrupted call with
unknown completion requires explicit --retry-uncertain, because it may already
have been billed. Completed calls are reused. Failed attempts remain in attempt
budgets; unknown usage retains a conservative byte-based reservation.

Requests are sequential with no automatic HTTP retries. This deliberately bounds
traffic and makes 429/529/timeout failures visible. Request budgets include the
original inference. Token reservations are conservative estimates; reported
provider usage is retained. An unexpectedly large reported usage stops new calls;
the checkpoint remains inspectable and budgets may be deliberately increased.
Archives bind to the responses actually observed, not a guarantee of future
provider determinism. The [validation report](../reports/0.7.0-molecular-validation.md)
records the small live synthetic repeatability check and its observed variation.

A completed live inference saves `features.json`, `request.json`, `inference.json`,
`run-status.json` and `report.md`. Explanation adds `checkpoint.json`, then final
`explanation.json` and `explanation-report.md`. Retain the whole run directory. `molecular inspect` checks fingerprints, schema semantics
and recomputes contributions from cached responses. Hashes do not authenticate a
provider response. Schema versions, model, prompt, taxonomy, background and group
definitions all participate in reproducibility. JSON uses float-roundtrip parsing.

Compare two completed archives with `molecular stability LEFT RIGHT --top-k 10`.
The offline report includes sign agreement, magnitude rank correlation with tied
ranks, top-k overlap, attribution RMSE and baseline/full-output differences.
The sample, request and target must match; baseline/seed differences remain named.

`molecular repeatability RUN_DIR --out-dir NEW_DIR --replicates 2
--max-evaluations 4 --max-input-tokens 50000 --max-seconds 120` makes **new billed
synthetic requests**: repeated full inputs and an explicitly masked baseline.
It preserves each attempt/response and reports per-class ranges and sample SDs.
A failed call ends the experiment without automatic retry. Two repeats are a
small engineering diagnostic, not a model-stability study or confidence interval.

## Experimental native SBS96 derivation

```bash
./target/debug/josh molecular signatures /tmp/features.json \
  --fasta reference.fa --fai reference.fa.fai --catalogue signature-catalogue.json \
  --opportunity-profile reviewed-profile-v1 --min-mutations 50 \
  --output /tmp/signature-fit.json
./target/debug/josh molecular attach-signatures /tmp/features.json \
  /tmp/signature-fit.json --output /tmp/with-signatures.json
```

The Rust reader uses the FASTA index to extract trinucleotide context and checks
reference alleles/builds. Only declared somatic SNVs contribute. Purine substitutions
are reverse-complemented into the 96 pyrimidine-centered channels described by
[COSMIC](https://cancer.sanger.ac.uk/signatures/sbs/). Contig aliases are not guessed.

Supply a pinned catalogue containing all 96 channel labels and normalized
signature columns. A nonnegative coordinate-descent solver reports fitted counts,
convergence, reconstructed spectrum, cosine similarity and residual. The declared
opportunity profile must match; no panel correction is inferred. Fifty mutations
is an editable engineering QC default, not a validated threshold. Uncertainty is
explicitly not estimated. Correlated columns may yield nonunique exposures.
Attaching derived signatures groups them with source variants to avoid attribution
leakage. Catalogue/genome data and their rights are external assets; none is bundled.

## Cohorts and evaluation

`molecular cohort MANIFEST --out-dir DIRECTORY` executes a bounded synthetic cohort
with labels in a separate manifest. Records specify feature-file paths, partitions
and known-primary truth. Patient groups cannot cross partitions. Exact requests and
individual outcomes are archived; rerunning an unchanged manifest reuses completed
results without requiring a key. Prior successes and failed-call reservations
count before scheduling new work, and a lock prevents concurrent cohort runs.
Uncertain interrupted requests are not silently resent. Set explicit
evaluation/token/time limits for a run. Actual real-data eligibility remains open.

`molecular evaluate RECORDS --partition test` computes metrics for frozen records
without network access. The generated evaluation-records schema defines the input.
Reports include class counts, top-1/top-3, macro/weighted F1, confusion, Brier,
log loss, ten-bin reliability/ECE, and coverage-versus-error. Failures stay in
eligible accuracy/coverage denominators. Probability metrics separately identify
the returned-prediction population. No probability renormalization is applied.

This release supplies evaluation tooling; no cancer cohort has been validated.
Calibration fitting, patient-bootstrap confidence intervals, panel-shift studies,
signature uncertainty and representative Jev repeatability studies remain scientific
follow-up work. The first live repeatability check is described separately.
