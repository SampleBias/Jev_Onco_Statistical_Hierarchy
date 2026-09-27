# Jev Onco Statistical Hierarchy (JOSH)

Rust workbench (Ratatui TUI and CLI) for cancer-of-unknown-primary research, version **0.8.0**. It loads molecular, expression, and clinical evidence, asks the hosted model **Jev** (`jev-1.13.0`) for an origin distribution, then checks, gates, charts, and exports the answer locally.

## How Jev is used

```mermaid
flowchart TD
  A[Sample, expression profile, or case] --> B[Import and validate in Rust]
  B --> C["One molecular-origin-v6 request"]
  C --> Q1[primary_site — Choice over 22 classes plus unresolved outcomes]
  C --> Q2[evidence_sufficient and conflicting_evidence]
  C --> Q3[Five boundary Nouls on this call only]
  Q1 --> J["Jev · jev-1.13.0 · api.typesafe.ai/v1/systemone"]
  Q2 --> J
  Q3 --> J
  J --> V[Check the model, answer keys, and probability mass]
  V --> G[Local gates: abstained or review_required]
  G --> R[Rankings, parent group, charts, Markdown]
  R -.->|optional Shapley| M[Original three questions, feature groups masked]
  M -.-> J
```

New prepares use `molecular-origin-v6`. The Choice, the two gated checks, and five boundary checks share the evidence and cannot read each other's answers. Boundary answers are reported with the result. Gates still use the leading score, its margin, sufficiency, and conflict. The parent of the leading class is a local lookup on the taxonomy. A request may use Jev's window: 64k tokens for the call, and 32k for the evidence plus the longest question.

A live call accepts declared **synthetic** data and `TYPESAFE_API_KEY`. Rate-limit and overload responses retry at most twice. A saved run keeps the prompt it was prepared with, so older archives stay on v2–v5. `josh molecular prepare`, `josh reference compare`, and `josh reference prepare` stay offline. `josh reference run` sends one synthetic expression-similarity request, separate from the panel classifier.

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

## License

The study question takes its inspiration from OncoNPC:

Moon et al., *Machine learning for genetics-based classification and treatment response prediction in cancer of unknown primary*, Nature Medicine 29, 2057–2067 (2023). [doi:10.1038/s41591-023-02482-6](https://doi.org/10.1038/s41591-023-02482-6)

Authors' repository: [itmoon7/onconpc](https://github.com/itmoon7/onconpc)

JOSH is a separate Rust program. It contains no OncoNPC source, weights, or data. Origin inference is Jev. OncoNPC's classifier is XGBoost, and that model is not in this repository.

Licensed under the [MIT License](LICENSE).
