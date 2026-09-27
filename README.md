# Jev Onco Statistical Hierarchy (JOSH)

Rust workbench (Ratatui TUI and CLI) for cancer-of-unknown-primary research, version **0.8.0**. It loads molecular, expression, and clinical evidence, asks the hosted model **Jev** (`jev-1.13.0`) for an origin distribution, then checks, gates, charts, and exports the answer locally.

## How Jev is used

```mermaid
flowchart TD
  A[Sample, expression profile, or case] --> B[Import and validate in Rust]
  B --> C[One request, identifiers and labels withheld]
  C --> Q1[primary_site — Choice over 22 classes plus unresolved outcomes]
  C --> Q2[evidence_sufficient — Noul]
  C --> Q3[conflicting_evidence — Noul]
  Q1 --> J["Jev · api.typesafe.ai/v1/systemone"]
  Q2 --> J
  Q3 --> J
  J --> V[Reject a mismatched model, keys, or probability mass]
  V --> G[Local gates: abstained or review_required]
  G --> R[Rankings, charts, Markdown]
  R -.->|optional Shapley| M[Same questions, feature groups masked]
  M -.-> J
```

The three questions share the evidence and cannot read each other's answers. Rust owns counts, thresholds, reference comparison, cohort statistics, and rendering. A live call accepts declared **synthetic** data, uses `TYPESAFE_API_KEY`, and is sent once after confirmation. Prompt `molecular-origin-v5` is the current molecular request.

| Crate | Role |
| --- | --- |
| `josh-core` | Contracts, prompts, response checks, gates |
| `josh-jev` | HTTPS client |
| `josh-ingest` | File and table import |
| `josh-features` | Mapping, signatures, reference comparison, cohort stats |
| `josh-explain` | Shapley values over a saved run |
| `josh-app` | CLI, TUI, loopback API |

## Start

```bash
./scripts/start
```

Requires Cargo, pinned Rust **1.98.1**, and a C compiler. In the TUI: open a bundled sample, then **Data → Run analysis**. `josh molecular prepare` prints the request without sending it. `josh --help` lists commands.

[Quickstart](docs/QUICKSTART.md) · [User guide](docs/USER_GUIDE.md) · [Jev workflow](docs/references/JEV_WORKFLOW.md)
