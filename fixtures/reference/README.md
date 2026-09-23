# Synthetic reference comparison fixtures

All expression measurements, sample/group IDs and class labels here are invented.
Demonstration A/B are software-test classes, not cancer types or validated references.
Use the six-gene dictionary from ../expression/hgnc-subset.tsv with both datasets.
Declare `--synthetic --units tpm --transform log2-one-plus --platform tutorial-v1`.
Build with `--min-genes 3`. Real research references require at least 100 common genes;
that is an engineering gate, not evidence that 100 genes are scientifically sufficient.
