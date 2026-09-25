# Sample-centered workbench delivery — 2026-09-25

## Delivered

- One active sample session with Data and Results. The primary toolbar has Open,
  one contextual next action and Menu; cohort studies is a secondary workspace.
- No automatically loaded, unrelated clinical or expression demonstration patients.
  Try a sample explicitly loads invented evidence without predicting anything.
- Local browse/import staging, detected molecular sample IDs and row counts,
  selected-sample CSV/TSV/MAF import with original source hashes/record references,
  and preserved single-sample annotated VCF restrictions.
- Five basic expression import fields, full Advanced settings, explicit data class,
  and retained import settings after asynchronous failure. Cancellation leaves the
  previous sample active, including when a worker must finish a write boundary.
- Explicit expression-only preparation tied to the selected sample, dataset and
  comparison. Changed evidence invalidates current analysis and retains the earlier
  input/run separately. No cross-taxonomy probability merging or assumed patient link.
- Session-local Previous samples and Earlier runs. Jev run files remain durable;
  temporary UI history is not a persistent project or a replacement for archives.
- All molecular charts in one Results selector; every cohort view remains. Legacy
  cases/bundles retain evidence, visualizations, guidance and review history in a
  read-only compatibility view. New clinical classification and guideline review
  are retired from the main TUI, while historical CLI/API contracts remain.
- Portable inference-only JSON exports include evidence and validated inference
  (kind josh_sample_run, schema_version 1). Bare legacy inference files still load
  with their features.json sidecar; explanation/checkpoint formats are unchanged.
  Additional live explanation work uses the original saved run directory.
- A failed new inference attempt cannot attach its directory to an old result.
  Existing result provenance remains intact and the failed attempt is identified.
- Updated built-in help, quickstart, terminal reference and modality guides.

## Verification

- **222 workspace tests passed**, including selected-sample provenance, portable
  export/reopen/tamper rejection, lossless cancellation, failed imports, expression
  binding invalidation, legacy read-only behavior, session restoration and rerun
  failure provenance.
- Workspace build, formatting, diff checks and all-target clippy with warnings
  denied passed. No dependency, lockfile or inference model change.
- Actual Ratatui buffers rendered at 80×24 and larger sizes; tiny sizes down to 1×1
  tested without panics. All main controls tested in both Tab directions and through
  mouse routing; chart selection and header integrity have regression coverage.
- Real PTY smoke test: open built-in samples, load an input, open the one-request
  confirmation, cancel without sending, and quit with mouse/terminal restoration.
  That process used a dummy credential, not the user's secret.
- Transport tests required localhost-binding permission for mock HTTP servers.
  No live Jev requests, real patient data, model changes, commits or pushes.

## Boundaries

This is an interface/state-management refactor, not scientific validation. Jev
remains jev-1.13.0, live input eligibility remains synthetic-only, raw scores remain
uncalibrated, and explanations remain separately budgeted. Representative cancer
benchmarks, calibration, independent CUP validation and outcome/actionability
evidence remain open in the [objective audit](../assessment/ONCONPC_PARITY.md).

Legacy case-table import still uses the existing CLI before opening its bundle.
Expression compatibility requires declared units, platform, transformation and
appropriate mapping/reference assets; the interface does not invent these facts.
Clinical records are not automatically joined to molecular samples.
