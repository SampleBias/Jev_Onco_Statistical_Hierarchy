# Expression fixtures

`synthetic-expression.tsv` contains invented TPM-like measurements for two samples.
There are no ground-truth cancer labels and no reference cohort. These values are
software-test inputs, not biological evidence.

`hgnc-subset.tsv` contains six real approved HGNC identifier mappings extracted from
the public complete-set download. The acquisition URL, date and complete-source
SHA-256 are in [the provenance record](hgnc-subset.provenance.json). This is a small
mapping fixture, not a complete dictionary. No cancer dataset or model weights are
included.

See the [expression workflow guide](../../docs/data/EXPRESSION_GUIDE.md) for import,
QC, exploration and reproduction commands.
