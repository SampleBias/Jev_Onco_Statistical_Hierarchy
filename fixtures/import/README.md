# Synthetic import fixtures

`synthetic-findings.csv` contains seven authored observations for three invented cases, two patient groups and three specimens. Its labels are test inputs, not medically adjudicated ground truth. It exercises censored age, positive/negative/unknown/not-tested status, repeated patient groups, source references and label separation.

`synthetic-cases.jsonl` contains two renamed copies of the original invented schema 1 case. It exercises migration and unassigned patient groups. Neither fixture contains real patient records or establishes Jev accuracy.

See the [import guide](../../docs/data/IMPORT_GUIDE.md) for commands and formats.
