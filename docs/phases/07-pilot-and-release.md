# Phase 07 — Research pilot and release decision

Status: planned. Leads: clinical lead and product owner. Supporting owners: ML/statistics, QA and platform. Estimate: 8–15 person-days of team effort; cohort collection and study follow-up may require substantially longer. Depends on: accepted Phases 04–06. Outcome: a supported research release, a narrower product, or a documented no-go.

## Pilot design

Run the frozen system in a controlled retrospective or otherwise approved research setting. Maintain blinding to the reference origin during inference and follow the adjudication protocol. Reviewers assess usability and error patterns as well as model output. Do not measure “diagnostic benefit” merely by showing a convincing interface or by comparing with a label derived from Jev itself.

## Work packages

| ID | Owner | Task and concrete output |
| --- | --- | --- |
| P07-01 | Clinical + ML | Finalize eligible cases, ground-truth process, independent reviewers and analysis plan using Phase 04 criteria |
| P07-02 | QA/platform | Freeze release artifacts and verify the pilot environment against the evaluated configuration |
| P07-03 | Clinical + product | Train reviewers on missingness, score interpretation, abstention, disagreement and issue reporting |
| P07-04 | ML | Run blinded cases; preserve all inputs, outputs, technical failures and exclusions under the approved data policy |
| P07-05 | Clinical | Adjudicate disagreements independently; distinguish model error, missing evidence, wrong ground truth and importer error |
| P07-06 | Statistician | Produce final metrics with uncertainty, subgroup/class counts, calibration and coverage; compare against predefined criteria |
| P07-07 | Product/QA | Evaluate whether users misread scores or override abstention; fix misleading wording/workflows and repeat affected checks |
| P07-08 | Technical lead | Issue a go/narrow/no-go decision with exact supported modalities, classes, settings and known limitations |
| P07-09 | Platform + ML | Establish monitoring, version revalidation, incident handling and periodic drift review |
| P07-10 | Product | Publish a developer/research-user guide and release notes that separate implemented capabilities from validated claims |

## Acceptance criteria

- Predefined scientific criteria are met for every released claim; inconclusive results are described as inconclusive.
- Pilot reports include technical failures and abstentions in the denominator and specify where ground truth was unavailable.
- Reviewers understand that a high score can be wrong and can identify unsupported or insufficient-evidence cases.
- Any calibrated output has a compatible artifact and the population/modality scope appears in the report.
- The final bundle can be reproduced and rolled back; responsibility for operation, data and scientific monitoring is assigned.
- Outstanding defects are assessed against the intended research use and are reflected in the actual release scope.

## Release choices

**Research classifier release:** Jev meets the agreed criteria for stated modalities/classes; deploy within that scope and retain review/abstention behavior.

**Narrower evidence assistant:** Jev is useful for evidence organization or triage but does not meet origin-prediction criteria; remove unsupported origin-probability claims rather than obscuring the result.

**No-go:** performance, data validity or operating requirements are inadequate; preserve the implementation and findings for further research without releasing the classifier for use.

## Continuing work

Revalidate changes to provider version, question wording, taxonomy, input preparation and calibration. Watch class/assay/site distributions, missingness, abstention and provider errors; label-backed audits are needed to detect accuracy drift, since score distributions alone cannot prove continued accuracy. Keep human corrections separate from independent outcome labels.

Any proposed clinical decision-support deployment is a new scope with its own intended use, evidence and applicable review process. It is not automatically authorized or validated by this research pilot.
