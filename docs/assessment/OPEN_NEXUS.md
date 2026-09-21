# Open_Nexus assessment

Inspected 2026-09-21. Repository: [SampleBias/Open_Nexus](https://github.com/SampleBias/Open_Nexus). Exact revision: `ee8069cdaaf997721cb071dffe1cb243651a5f56`. Scope: all 11 tracked files, source inspection, Python syntax parsing, and YAML parsing. The training pipeline was not executed because dependencies, data, and model artifacts are incomplete.

**Historical audit:** the XGBoost code, original model weights and SHAP explanations described below are excluded from the Rust rebuild by project decision. They will not be ported, recovered, retrained or added as optional benchmarks. Jev is the sole classifier; use the [current delivery plan](../PLAN.md) for implementation scope.

## Purpose

The repository aims to expose cancer-origin prediction from targeted tumor sequencing through an authenticated API. Its feature workflow combines somatic mutation counts, copy-number alterations, mutation-signature features, age, and sex. It follows the OncoNPC approach: learn tumor classes from cancers with known primaries, then estimate possible origins for CUP cases. The README also describes history, registration, MongoDB, and SHAP explanations. Those descriptions substantially exceed the implementation present in this revision. [README](https://github.com/SampleBias/Open_Nexus/blob/ee8069cdaaf997721cb071dffe1cb243651a5f56/README.md)

## What actually exists

| File | Implemented material | Rebuild disposition |
| --- | --- | --- |
| `api/app.py` | A short `ModelService` that tries to load metadata and call `predict_proba` | Replace with Rust API and Jev adapter |
| `codes/process_features.py` | GENIE/DFCI loading, cohort selection, feature export | Recover requirements; rewrite imports with explicit schemas |
| `codes/utils.py` | Mutation/CNA/demographic processing, signature feature calculation, XGBoost inference, SHAP plots | Reference for data lineage only; exclude classifier and explanation code |
| `codes/utils_training.py` | XGBoost fitting, filtering, evaluation helpers and fold loop | Excluded from the rebuild |
| `codes/Mutation_Analysis.py` | Exploratory mutation analysis, NNLS helper and visualization | Incomplete research script, not a production pipeline |
| `codes/config.yaml` | Data paths and column names | Invalid YAML; replace |
| `config.yaml` | Empty file | Replace |
| `README.md`, `LICENSE`, `.gitignore`, `.vscode/settings.json` | Documentation, GPL v2 text and development metadata | Preserve source provenance if reusing material |

## Historical machine learning inventory

1. **Classifier training code:** `XGBClassifier` with histogram tree training and configurable parameters. It is code, not an included trained model. [Training helpers](https://github.com/SampleBias/Open_Nexus/blob/ee8069cdaaf997721cb071dffe1cb243651a5f56/codes/utils_training.py#L72)
2. **Inference code:** class probabilities, maximum-probability class, and a Booster path applying a manual softmax to margins. No executable end-to-end service is supplied. [Inference helpers](https://github.com/SampleBias/Open_Nexus/blob/ee8069cdaaf997721cb071dffe1cb243651a5f56/codes/utils.py#L15)
3. **Feature engineering:** gene mutation counts, CNA values, age/sex, SigProfiler context generation, and signature-reference projections. The `utils.py` projection is a matrix multiplication; it should not automatically be interpreted as fitted signature exposure. A separate exploratory script uses nonnegative least squares. [Feature workflow](https://github.com/SampleBias/Open_Nexus/blob/ee8069cdaaf997721cb071dffe1cb243651a5f56/codes/process_features.py)
4. **Explanations:** SHAP TreeExplainer and plotting helpers for XGBoost. These do not explain Jev. Preserve the distinction between cited input evidence and model attribution.
5. **Evaluation:** classification reports, confidence cutoffs and a custom fold loop. No benchmark outputs are committed that establish this fork's claimed optimization.
6. **Absent:** trained weights, feature-order manifest, class mapping pickle, age normalization statistics, signature matrices, labeled cohort, validation report, Jev integration, frontend, tests, and dependency lockfiles.

The separate [OncoNPC repository](https://github.com/itmoon7/onconpc) includes additional model/data/notebook directories. Those are upstream assets, not assets present in Open_Nexus. Availability does not establish permissions or feature compatibility with this fork. The [OncoNPC study](https://pubmed.ncbi.nlm.nih.gov/37550415/) reports performance for its own molecular classifier and cohorts; those results cannot be assigned to Open_Nexus or Jev.

## Confirmed blockers and correctness concerns

| Finding | Evidence | Consequence |
| --- | --- | --- |
| No runnable Flask app | `api/app.py` contains neither a Flask instance nor route definitions | README API endpoints are not implemented |
| Broken service initialization | Imports nonexistent `codes.utils.load_model`; also references undefined `os`, `json`, and `MODEL_PATH` | Import/startup fails before inference |
| Missing launch/build files | No `requirements.txt`, `run.py`, `api/config.py`, routes, database modules, or training entry point advertised by README | Setup instructions cannot work as written |
| Invalid YAML | `*base_path/dfci/...` parsed with PyYAML raises `ScannerError` | YAML aliases cannot concatenate a path suffix this way |
| Missing research artifacts | References to feature/class/statistics pickles and signature CSVs without those files | Feature alignment and inference cannot be reproduced |
| Fold design problem | Each nonfinal fold independently samples the full label table | Validation sets can overlap; not a disjoint k-fold protocol; no patient-group split |
| Fragile class reporting | Threshold-filtered reports mix full class names with labels derived from subsets | Empty/partial-class subsets need explicit handling |
| Data interpretation issues | String ages use `int(x[1:])`; CNA missing values are filled with zero | Plain string ages may lose a digit; unmeasured CNA can become diploid |
| Exploratory placeholder | `process_mutation_data` doubles a `value` column | Not a complete genomics preprocessing operation |
| Feature check too weak | `validate_model` returns early if `feature_names_` is absent | Some models bypass both feature and class checks |

All five Python files passed syntax parsing. That does not resolve runtime errors or demonstrate valid science. Missing data prevented numerical verification of SigProfiler processing, feature orientation, or model parity.

## Requirements for the Jev Rust rebuild

The useful inheritance is the question the product answers and its data categories. The classifier is Jev, so the main work becomes canonical case import, evidence preparation, taxonomy and question design, API reliability, probability validation, and review workflows. Evaluation uses labeled cases and deterministic class-frequency references. Supporting evidence comes from imported observations and their provenance.

The practical first release should evaluate imported, structured case evidence. Raw sequencing files, expression matrices and whole-slide images require dedicated preprocessing and cannot simply replace a Jev text state. A verified public/authorized dataset and specialist-adjudicated labels are the main scientific dependencies; rewriting API code alone cannot supply them.
