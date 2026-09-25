# CUP visualizations and NICE review

Current TUI compatibility note: Open a case or bundle to inspect its explicitly
labelled, read-only legacy records. Evidence, charts, guidance and saved review
history remain available through the view selector; Export record saves a complete
case. The separate F4 classifier and new guideline-review actions are retired from
the main interface. The historical controls below document the original review
implementation; its CLI/API and schema contracts remain compatible. See the
[current User Guide](USER_GUIDE.md) for the supported sample-centered interface.

Current application: 0.8.0. Clinical functionality introduced in 0.4.0; case schema 3
and ruleset `cg104-review-v0.1` are unchanged.
Source checked 2026-09-22. Clinical signoff and diagnostic outcome validation are **pending**.
This is a local research review aid, not a medical-device validation, complete guideline
implementation, automated diagnosis, treatment protocol or clinical approval.

## Start and navigate

```bash
cargo run --locked --bin josh -- tui
# Press F4 for Clinical; l loads a case or bundle in the same workspace.
./target/debug/josh example --clinical --output /tmp/josh-clinical-example.json
./target/debug/josh guidance /tmp/josh-clinical-example.json --format text
./target/debug/josh guidance /tmp/josh-clinical-example.json --output /tmp/josh-guidance.json
```

The initial Clinical demonstration case is deliberately invented, including its investigations, dates and
reviewer. It has no reference diagnosis. The legacy `josh example` command remains schema 1.
No API key is needed for the visualizations, guidance, review entries or offline demo.
These controls apply in Clinical (F4). F2/F3 returns to Analysis/Data without losing
the case. The shared g/F1 guide documents navigation across the workspace.

| View | Controls and meaning |
| --- | --- |
| 6 Visuals | Left/right cycles Scores, IHC matrix, Pathway, Timeline, Evidence; up/down scrolls long lists |
| Scores | All model outcomes on a fixed 0–100% axis; uncalibrated, explicit MOCK/REPLAY/JEV badge; no top-N renormalization |
| IHC matrix | Recorded marker/result/status table, with text and color for positive, negative, equivocal, not tested and unknown; not a site-compatibility predictor |
| Pathway | MUO → provisional → confirmed CUP reference, highlighting only the clinician-recorded stage; alternatives can leave this pathway |
| Timeline | Recorded investigation days from one shared case-specific reference; planned/completed are distinct; unknown dates stay unplotted |
| Evidence | Counts by evidence type, including unknown/not-tested records; counts are not adequacy or completeness scores |
| 7 Guidance | Up/down selects a rule; left/right or PgUp/PgDn scrolls its details; `a` records a review; `s` exports the report |
| 8 Review | Structured clinical context and review history; `s` saves a complete standalone case |

The coordinated navy/teal/cyan/blue/violet theme uses amber for uncertainty and rose for
conflicts; every status also has text. There are no invented calibration, survival,
box plots or confusion matrices. Those require suitable reviewed cohort/outcome data.
These visualizations are implementation choices, **not NICE-mandated chart formats**.

Ratatui references used: [BarChart](https://ratatui.rs/examples/widgets/barchart/),
[Table](https://ratatui.rs/examples/widgets/table/),
[Chart](https://ratatui.rs/examples/widgets/chart/), and
[Tabs](https://ratatui.rs/examples/widgets/tabs/).

## Clinical context and migration

Use [the synthetic schema 3 fixture](../fixtures/clinical-case.json) and
[`case.schema.json`](../contracts/case.schema.json) as the exact contract.
Clinical assertions are edited in standalone JSON and reloaded with `r`; the TUI
review dialog records decisions, not clinical observations.

Schema 3 retains schema 2 evidence metadata and requires a `clinical` object.
An empty `clinical: {}` is valid and means unassessed; it does not invent assessment
facts. To upgrade a valid schema 2 JSON case, change `schema_version` to 3 and add
that object, then validate. Schema 1 should first go through the existing
[schema 2 import migration](data/IMPORT_GUIDE.md). Never change a bundle in place:
save a standalone copy, because bundle fingerprints protect the original.

Populated clinical assertions need an `assessment` containing a restricted local
`reviewer_id`, real calendar `recorded_on` date (YYYY-MM-DD), and a source description.
Identity is self-reported, not authenticated. Use pseudonymous IDs; exports are sensitive
local records, not anonymized simply because they contain hashes.

- `stage` and `lineage` are explicitly recorded assessments, never classifier outputs.
- `features` is a typed boolean map: absent key = unknown, false = assessed no,
  true = assessed yes. Free-text findings never set these features automatically.
- `histology_confirmed` and `cytology_confirmed` are distinct. Cytology may support
  provisional CUP; the confirmed-CUP check requires recorded histology.
- `initial_workup_complete` and `further_investigations_complete` mean a clinician
  considers the appropriate phase complete; they do not mean every possible test was done.
- Investigation records hold the latest episode per kind, status, optional relative
  day, result, reason and source. Repeated investigations are not a longitudinal
  database: earlier episode records must remain in prior saved case revisions.
- Completed investigations require a result; declined/not-indicated/contraindicated
  entries require a reason. A recorded result does not establish adequacy.
- ECOG, LDH, albumin and liver-metastasis assessment are review context. Measurements
  require units and source. No prognosis score, survival estimate or unit conversion
  is calculated.
- `pet_ct_discussed_with_cup_team` records discussion of that test; a generic completed
  CUP-team encounter alone is insufficient. Breast MRI likewise has an explicit
  `breast_mdt_assessment_complete` check.

Existing 16 KiB case and 64-finding limits remain. Clinical investigations/reviews each
have a maximum of 32 entries, subject to the total byte limit. Failed review validation
is atomic: no partial note replaces the current case.

## Source-linked rules and boundaries

Primary source: [NICE CG104 recommendations](https://www.nice.org.uk/guidance/cg104/chapter/Recommendations).
Amended 2023-04-26; [NICE's recorded review](https://www.nice.org.uk/guidance/CG104)
was 2025-07-16. The application does not dynamically download guidelines at runtime.
It evaluates a fixed, versioned, locally tested subset:

| Rule group | CG104 reference | Implementation boundary |
| --- | --- | --- |
| Scope and stage | Overview; terms used | Adult/metastatic/lineage/stage checks; unsupported recorded stage flagged, never silently reassigned |
| CUP team and key worker | 1.1.1.1–1.1.1.2 | Recorded involvement/assignment, not a referral dispatch |
| Initial assessment | 1.2.1.1 | History/exam, baseline laboratory tests, chest X-ray, CT and biopsy coverage; conditional myeloma screen and testicular ultrasound |
| Tumour markers | 1.2.2.1 | AFP/hCG/PSA/CA125 contextual exceptions, not a general screening panel |
| GI/breast investigation | 1.2.2.2–1.2.2.4 | Indication checks; breast MRI requires appropriate prior workup and MDT assessment |
| PET-CT | 1.2.2.5–1.2.2.6 | Cervical offer conditions distinct from extra-cervical consideration after discussion |
| Pathology | 1.2.2.7–1.2.2.8 | Source-panel and additional-IHC review prompt; no automated stain interpretation |
| Ascites | 1.2.3.3 | Tissue-sampling review when feasible |
| Benefit/preferences/support | 1.3.1.1–1.3.1.3 | Fitness, management impact, understanding and treatment willingness; explanation/support when investigation is inappropriate |
| Prognostic context | 1.3.2.1–1.3.2.3 | Recorded factors, without inventing numeric prognosis |
| Specialist presentations | 1.4.1.1–1.4.1.3; 1.4.2.1 | Head/neck, breast, inguinal-squamous and brain-only MDT review prompts |
| Genomic update | Withdrawn 1.2.2.9 / 1.3.2.4 | Informational link to current eligibility resources; no active old prohibition |

The [April 2023 update](https://www.nice.org.uk/guidance/cg104/chapter/Update-information)
withdrew the gene-expression restrictions and links to the
[NHS national genomic test directories](https://www.england.nhs.uk/publication/national-genomic-test-directories/).
This is not an endorsement of Jev or permission to infer current genomic test eligibility.
No treatment or genomic-test order is generated.

Known non-CUP lineages, a known primary, and patients under 18 are outside this rule
scope. Unknown adult status does not imply eligibility. Provisional/confirmed CUP
prerequisites are checked separately from access to the CUP team.

The shared benefit gate conservatively pauses investigation prompts, including initial
coverage prompts. That is a **software safety choice**, not a verbatim additional NICE
requirement for each initial test. It does not block referrals/support or decide to stop
care. Completed records remain visible even if current fitness has changed. Source
sex-specific checks use recorded sex at birth, a limited proxy requiring individual
clinical review; they are not a replacement for anatomy or clinical assessment.

Not implemented: all guideline service requirements, bronchoscopy/VATS pathways,
solitary-metastasis treatment, systemic/radiation/surgical treatment selection, trial
eligibility, complete contemporary pathology/genomics guidance or real MDT integration.
The legacy 14-outcome research taxonomy is unchanged and still needs specialist review.

## Review persistence and provenance

On Guidance, select a rule, press `a`, and enter:

```text
reviewer-1 | 2026-09-22 | deferred | Synthetic example: awaiting documented MDT discussion
```

Actions: `acknowledged`, `deferred`, `not_applicable`, `departed`. Every action
requires a reason. A review annotates the rule; it never suppresses its finding,
changes stage, alters evidence or changes model probabilities.

The review stores rule/ruleset IDs and a case fingerprint excluding review history.
Adding another review does not invalidate the first. Changing evidence, clinical
context or the ruleset makes old reviews stale. This is local traceability, not a
signed, tamper-proof audit system.

Review changes clear the displayed Clinical model result; the molecular Analysis
result is independent. Recorded unsaved reviews block case changes and trigger a
quit/discard confirmation even when another section is active. Press **8 then s** to save a new complete
case; this becomes the reload path. Existing files are never overwritten and Unix
exports are owner-readable/writable. Guidance/visual exports do not mark the editable
case as saved. They contain local clinical context and must be handled accordingly.

Provider preparation uses the same explicit evidence allowlist as schema 2.
Clinical context, review notes and reviewer identity are excluded from Jev state.
The guidance engine does not consume Jev results. Live inference remains restricted
to declared synthetic cases, with explicit TUI confirmation.

The offline API adds `POST /v1/guidance`; it returns the same source-linked report,
including clinical context, rule evidence, statuses, current/stale review indicators
and limitations. [Guidance JSON Schema](../contracts/guidance.schema.json).

## Verification and clinical review still required

Automated tests cover unknown/false distinctions, conditional branches, adult/lineage
scope, cytology versus final histology, benefit/MDT gates, exceptions, review
invalidation, private provider state, schema compatibility, imports, screen resizing,
navigation and protected persistence. They verify software behavior, not safety or
diagnostic accuracy in practice.

Before real clinical use: independent CUP oncologist/pathologist review of each rule
and omission; review sex/anatomy applicability; validate on an authorized blinded
cohort; agree taxonomy and operating points; establish privacy, authentication,
durable audit and applicable clinical software governance. Nothing in this release
claims those steps are complete.
