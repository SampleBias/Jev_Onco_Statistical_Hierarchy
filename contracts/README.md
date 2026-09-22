# Generated contracts

These artifacts are generated from the Rust types and checked in CI:

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

Run `cargo run -p josh-app --example export_contracts --locked` to refresh them, or `josh schema case` to print one contract. Cases support schemas 1, 2 and 3; current results use schema 2. Guidance reports, import reports and split manifests use version 1. Runtime validation adds relational and byte-budget checks documented in the [foundation handoff](../docs/engineering/foundation.md), [import guide](../docs/data/IMPORT_GUIDE.md) and [clinical review guide](../docs/CLINICAL_REVIEW_GUIDE.md).
