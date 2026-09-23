# Redesign Phase 3 — Secondary modalities and dataset connectors

Status: CNA and imported/experimental derived SBS signatures were brought forward
into 0.7.0 by the approved [OncoNPC adaptation](04-onconpc-jev-explanations.md).
Connectors, methylation, fusions and slide embeddings remain proposed. Native
signature fitting has numerical tests, not targeted-panel validation; see the
[molecular guide](../data/MOLECULAR_GUIDE.md).

Depends on [Phase 1](01-samples-expression-and-reference.md)
adapter/artifact contracts and [Phase 2](02-multimodal-and-cohorts.md) multimodal
pipeline. Leads: data engineer and bioinformatician. Reviewers: data steward,
statistics lead, Rust backend and QA. Stack remains Rust, Jev and Ratatui.
Direction: [gap assessment](../assessment/DATA_FIRST_GAP_ASSESSMENT.md).

## Outcome

Add source adapters and modalities without changing Sample identity, core workflow
or score semantics. Acquisition, parsing, normalization and inference eligibility
remain distinct stages. No connector automatically enrolls unknown-origin samples
as labeled reference cancers.

| ID | Owner | Work package and artifact |
| --- | --- | --- |
| R3-01 | Data engineer | GDC/TCGA connector: project/file metadata, chosen expression/variant assets, stable sample mapping and acquisition manifest |
| R3-02 | Data engineer | GEO connector: accession metadata, platform and supplementary-file inventory, selected supported data products and mapping preview |
| R3-03 | Data engineer | cBioPortal connector: study/sample/molecular-profile selection, explicit datatypes/units and source versions |
| R3-04 | Data engineer + curator | GENIE release adapter: sample/patient/mutation/coverage and supported CNA joins, provenance and orphan/conflict reports |
| R3-05 | Data steward + backend | EGA-controlled acquisition where authorized, keeping dataset credentials separate from Jev credentials and checking external-processing eligibility |
| R3-06 | Bioinformatician | Copy-number and methylation schemas/adapters, QC, feature methods and compatible references |
| R3-07 | Bioinformatician | Fusion and structural-variant evidence with breakpoints, build, assay, caller and supporting measurements |
| R3-08 | Bioinformatician + backend | Digital-pathology/whole-slide embedding and histology feature imports with producing-model/coordinate provenance and reference compatibility |
| R3-09 | Bioinformatician | miRNA and proteomics modality adapters and feature/reference contracts where input assets are selected |
| R3-10 | Backend + QA | Dataset catalog, schema-drift contracts, download resume/cache/limits, per-source errors and offline reproducibility |

Deliver each connector/modality as an independent tested capability. The table is
a coverage plan, not a promise that every repository exposes identical APIs or that
every accession yields immediately inferable data. Inspect current official source
schemas and access policies when implementing each adapter.

## Connector contract

A connector resolves source identifiers into a recorded file/metadata inventory.
The user selects supported assays/data products before acquisition. The adapter then
uses the same detection → parse → normalization → QC → sample conversion services
as local imports; UI code has no repository-specific file logic.

Record source URL/identifier, study/accession, release or retrieval snapshot,
checksums, citation, processing metadata, selected files/samples and access terms.
Support caching, bounded retries, interrupted downloads and clear partial failures.
Pin integration fixtures rather than requiring live repositories during unit tests.

An accession such as `GSE42392` is an identifier to resolve, not a declaration that
the study is CUP, has known-primary truth, or provides comparable expression values.
An accession may require supplementary metadata or manual configuration. Unknown
platform processing remains unknown; do not download files and silently guess units.

EGA access authorization does not by itself authorize sending controlled material
to a hosted classifier. Preserve local exploration for datasets ineligible for
external inference; make that eligibility part of the dataset/run configuration.
Credentials must never enter manifests, source control or Jev evidence packages.

## Modality requirements

- **Copy number:** distinguish segments, log ratios, absolute/relative values and
  discrete calls; retain genome, assay and processing assumptions. Missing data is
  not diploid. Normalization and comparison must match the representation.
- **Methylation:** retain probe/region identities, platform, beta/M-value scale,
  preprocessing, missingness and mapping versions. A methylation reference is a
  separate scientific asset, not an RNA reference with renamed fields.
- **Fusions/structural variants:** preserve partner/breakpoint ambiguity and caller
  confidence. Unsupported rearrangements remain explicit rather than collapsed
  into ordinary point mutations.
- **Pathology/WSI embeddings:** first accept precomputed features. Record producer
  model/version, vector dimension, slide/region provenance, preprocessing and
  compatible reference. Raw-slide embedding generation is a separately scoped
  capability, not implied by embedding import.
- **miRNA/proteomics:** declare identifier authority, assay scale, missingness and
  annotation versions; do not apply the gene-expression normalizer by default.

Jev receives bounded interpreted/comparison evidence produced in Rust. It does not
directly consume images or opaque embedding tensors; TypeSafe currently documents
text/structured state input. A producing embedding model is an input-processing
dependency to record, not a new JOSH tissue-of-origin classifier.
[TypeSafe state contract](https://docs.typesafe.ai/concepts/state)

## Exit criteria

- Every enabled connector has source-specific fixture tests and a documented live
  acquisition check for an eligible study. Unsupported files/modalities are shown,
  not silently dropped or reported as processed.
- Dataset Explorer shows actual samples, class annotations, modalities, feature
  counts, missingness, processing platform, citations and generated dictionary.
- Adding each modality uses the existing Sample/artifact contracts and produces
  source-linked QC/features/comparison evidence.
- Compatibility checks reject mismatched reference/model/feature versions without
  preventing raw-data inspection.
- New modalities and combinations have held-out comparison/classification and
  rejection evaluation before research performance claims are broadened.
- Saved runs can be inspected/replayed offline even when source endpoints change;
  newly downloaded releases create new provenance, not replacements for old artifacts.

Handoff: connector capability matrix and access notes; pinned integration fixtures;
new modality schemas/QC methods; reference manifests; performance/shift evaluation;
updated CLI/Ratatui guides and reproducible acquisition instructions.
