# Generated contracts

These artifacts are generated from the Rust types and checked in CI:

- [Independent sample](sample.schema.json)
- [Molecular dataset manifest](dataset.schema.json)
- [Expression measurement](expression-record.schema.json)
- [Case input](case.schema.json)
- [Jev request](jev-request.schema.json)
- [Jev response](jev-response.schema.json)
- [Result output](result.schema.json)
- [Local NICE guidance report](guidance.schema.json)
- [Error envelope](error.schema.json)
- [Offline OpenAPI 3.1](openapi.json)
- [Import report](import-report.schema.json)
- [Evaluation label sidecar](labels.schema.json)
- [Patient-group partitions](splits.schema.json)
- [Molecular features](molecular-features.schema.json)
- [Molecular inference archive](molecular-inference.schema.json)
- [Frozen molecular taxonomy](molecular-taxonomy.schema.json)
- [Explanation archive](explanation.schema.json)
- [Development background](explanation-background.schema.json)
- [SBS96 signature catalogue](signature-catalogue.schema.json)
- [Frozen evaluation records](evaluation-records.schema.json)
- [Cohort protocol](cohort.schema.json)

Run `cargo run -p josh-app --example export_contracts --locked` to refresh them, or `josh schema case` to print one contract. Cases support schemas 1, 2 and 3; current results use schema 2. Guidance reports, import reports and split manifests use version 1. Runtime validation adds relational and byte-budget checks documented in the [foundation handoff](../docs/engineering/foundation.md), [import guide](../docs/data/IMPORT_GUIDE.md) and [clinical review guide](../docs/CLINICAL_REVIEW_GUIDE.md).

Sample/dataset/measurement contracts are distinct from legacy Case schemas. The new
dataset CLI/TUI is local; these schemas do not imply new HTTP routes. Relational and
artifact checks are documented in the [expression guide](../docs/data/EXPRESSION_GUIDE.md).

Reference releases and numerical molecular evidence are versioned by
`reference.schema.json` and `molecular-evidence.schema.json`. They describe local
research comparisons and do not declare classifier probabilities or validation.

The 0.7.0 molecular/explanation contracts use schema version 1. Their commands are
CLI/TUI workflows, not new HTTP routes. Numerical/domain checks supplement JSON
Schema; see the [molecular guide](../docs/data/MOLECULAR_GUIDE.md).
