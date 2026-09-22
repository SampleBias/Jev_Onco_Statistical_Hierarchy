# Generated contracts

These artifacts are generated from the Rust types and checked in CI:

- [Case input](case.schema.json)
- [Jev request](jev-request.schema.json)
- [Jev response](jev-response.schema.json)
- [Result output](result.schema.json)
- [Error envelope](error.schema.json)
- [Offline OpenAPI 3.1](openapi.json)
- [Import report](import-report.schema.json)
- [Evaluation label sidecar](labels.schema.json)
- [Patient-group partitions](splits.schema.json)

Run `cargo run -p nexus-app --example export_contracts --locked` to refresh them, or `nexus schema case` to print one contract. Cases support schema 1 and 2; current results use schema 2. Import reports and split manifests use version 1. Runtime validation adds relational and byte-budget checks documented in the [foundation handoff](../docs/engineering/foundation.md) and [import guide](../docs/data/IMPORT_GUIDE.md).
