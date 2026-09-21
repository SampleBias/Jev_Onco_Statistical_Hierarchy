# Foundation handoff — application 0.2.0

## Reproducible build

Supported and tested baseline: Rust/Cargo 1.98.1 on Linux x86-64. CI uses Ubuntu 24.04 with a pinned toolchain and action commits; local development was also tested on Arch Linux. macOS and Windows are not yet supported/tested targets. No lower Rust version is claimed as the minimum supported version.

Install Rust through your normal toolchain manager and a C build toolchain. `rust-toolchain.toml` selects the exact toolchain for rustup users; distribution-managed installations must provide the matching compiler, formatter and Clippy. Python 3 is used only by the dependency inventory check, not by the application.

```sh
cargo fetch --locked
cargo build --workspace --offline --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --offline --locked -- -D warnings
cargo test --workspace --offline --locked
python3 scripts/dependency_inventory.py --check
```

The lockfile pins registry versions/checksums. Toolchain and lockfile pinning improve reproducibility but do not claim bit-identical binaries across OS/linker versions. A clean GitHub runner exercises the supported build without a cached project target directory.

## Contracts

Run `nexus schema case`, `nexus schema result`, `nexus schema jev-request`, `nexus schema jev-response`, `nexus schema error` or `nexus schema openapi`. Committed artifacts are in [contracts](../../contracts/README.md). Regenerate them with:

```sh
cargo run -p nexus-app --example export_contracts --offline --locked
```

The developer generator intentionally updates committed artifacts. Normal `--output` and TUI exports still refuse to overwrite existing files. Tests compare committed documents to generated values, resolve every local reference, validate example values against the schemas and validate actual HTTP responses against OpenAPI components. Schema generation uses [Schemars](https://docs.rs/schemars/1.2.2/schemars/); the test-only validator has default network-fetching features disabled.

JSON Schema defines shape and many bounds. Runtime validation additionally enforces UTF-8 byte limits, nonblank values, unique finding IDs, total serialized case budget, exact provider question/label sets, selected-choice consistency and probability sum. Passing an external schema validator alone does not authorize inference. `JevRequest.state` is a generic provider JSON field; only `prepare(Case)` constructs the application's permitted state.

HTTP endpoints remain `/health`, `/v1/prepare` and `/v1/demo`. Route descriptions are maintained beside the router; response schemas come from the same Rust types. This OpenAPI document does not describe a live provider endpoint or a future authenticated application.

## Stable failures

CLI JSON mode writes `{"error":{"code":"invalid_json","message":"input does not match the JSON schema"}}` to stderr and keeps stdout empty on failure. `--format text` gives a readable message. Invalid CLI arguments exit 2; workflow failures exit 1; success and ordinary broken stdout pipes exit 0. Help and version remain human-readable successful commands. An `--output` path never receives an error record.

HTTP parsing, case validation, body limits, missing routes and unsupported methods use the same envelope type. Expected status/code mappings: 400 malformed/trailing JSON, 413 `input_too_large`, 415 `unsupported_media_type`, 422 `invalid_json` or `invalid_case`, 404 `not_found`, 405 `method_not_allowed`. Parser text, submitted values, keys and provider bodies are excluded from error messages. Clients branch on `code`, not the prose message.

The 0.2.0 envelope replaces 0.1.0 plain CLI errors and the former HTTP string-valued `error`. Integrators must update error parsing. Mock result behavior, provider request bytes, case schema version 1 and existing workflow commands remain compatible. Results add an explicit result schema version and case fingerprint; replay probability semantics now say `unverified_replay_distribution`.

## Fingerprints

`fingerprint_version = typed-json-sha256-v1` specifies lowercase SHA-256 over `serde_json::to_vec` of the **typed value**. This is not RFC 8785/JCS and does not hash the original file bytes. Struct fields follow declaration order, maps use sorted keys, array order is significant, and optional case fields serialize as null. Input whitespace and input object key order do not affect the hash. Omitted optional fields and explicit null normalize to the same typed value. Text content is not Unicode-normalized or trimmed.

`request_sha256` covers the compact provider request body: model, state and questions. It excludes local case ID, data class, credentials and HTTP headers. `case_revision_sha256` covers the whole validated case, including local identity and data class. Case edits or reordering findings create a new revision. Renaming only the case changes the case revision but not the provider request. Tests freeze the initial synthetic request digest to detect accidental wire changes.

Every result includes both hashes, result schema version 1, model, prompt, taxonomy and policy versions. Hashes identify content; they do not establish who produced it, authenticate a response or prove clinical validity. Local raw-response replay still cannot prove that its response belongs to its case. Persisted request/response binding and authenticated replay verification belong to later storage/provider phases.

## Ownership and changes

| Module/artifact | Role owner | Review needed |
| --- | --- | --- |
| `nexus-core/domain.rs` and case schema | Backend + data engineer | Import compatibility, field semantics, limits |
| `prompt.rs`, `taxonomy.rs` | ML lead + clinical reviewer | Evidence leakage, label definitions, evaluation impact |
| `policy.rs` | ML/statistics + clinical reviewer | Abstention behavior and unvalidated operating points |
| `provider.rs`, `schema.rs`, `provenance.rs`, `errors.rs` | Backend lead | Wire/schema/hash compatibility |
| `nexus-jev` | Backend + platform | Provider boundary, data policy, reliability |
| `nexus-app` | App lead + QA | CLI/TUI/API behavior, exports, regression coverage |
| CI/toolchain/dependency inventory | Platform lead | Reproducible supported builds and dependency review |

Breaking case/result shape changes require the corresponding schema version and application release bump, an explicit migration note, regenerated artifacts and tests. Changes to field order or fingerprint serialization require a fingerprint version review. Changes to preprocessing, prompts, labels or gates require the matching version change and relevant development/held-out evaluation; package version alone is insufficient. Never silently replace the pinned Jev model.

Dependency updates are explicit reviewed changes to manifests/lockfile. Keep Schemars pinned because schema layout may change across releases. Regenerate [dependencies.json](dependencies.json) with `python3 scripts/dependency_inventory.py`, review declared license changes, regenerate schemas if needed and run CI. The inventory includes development/build/conditional packages, is not a vulnerability assessment, and does not resolve the project's distribution license. There is no automatic dependency or model update in inference.
