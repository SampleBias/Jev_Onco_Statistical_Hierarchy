use josh_core::{DataClass, molecular::FeatureValue};
use josh_ingest::molecular::*;
fn options() -> Options {
    Options {
        sample_id: "S1".into(),
        patient_group_id: "P1".into(),
        data_class: DataClass::Synthetic,
        source_id: "test".into(),
        assay: "test-panel".into(),
        reference_build: Some("GRCh38".into()),
    }
}
#[test]
fn canonical_cna_unknown_and_mismatched_sample_are_distinct() {
    let table = "sample_id\tid\tname\tmodality\tvalue\tunits\tstatus\tgene\nS1\tc1\tMDM2\tcopy_number\t2\tdiscrete_call\tobserved\tMDM2\nS1\tc2\tFAT1\tcopy_number\t\tdiscrete_call\tnot_tested\tFAT1\n";
    let r = import(table.as_bytes(), Format::Tsv, &options()).unwrap();
    assert_eq!(r.accepted_records, 2);
    assert!(matches!(
        r.features.features[0].value,
        Some(FeatureValue::CopyNumber { call: 2, .. })
    ));
    assert!(r.features.features[1].value.is_none());
    assert_eq!(r.source_sha256.len(), 64);
    assert!(
        import(
            table.replace("S1", "S2").as_bytes(),
            Format::Tsv,
            &options()
        )
        .is_err()
    );
    assert!(
        import(
            table.replace("\t2\t", "\t3\t").as_bytes(),
            Format::Tsv,
            &options()
        )
        .is_err()
    );
    assert!(
        import(
            table.replace("\tnot_tested\t", "\tobserved\t").as_bytes(),
            Format::Tsv,
            &options()
        )
        .is_err()
    );
}
#[test]
fn maf_import_preserves_unknown_somatic_status_and_rejects_build_mixing() {
    let maf = "Hugo_Symbol\tChromosome\tStart_Position\tReference_Allele\tTumor_Seq_Allele2\tVariant_Classification\tTumor_Sample_Barcode\tNCBI_Build\nKRAS\t12\t100\tG\tA\tMissense_Mutation\tS1\tGRCh38\n";
    let r = import(maf.as_bytes(), Format::Maf, &options()).unwrap();
    assert!(matches!(
        r.features.features[0].value,
        Some(FeatureValue::Mutation { somatic: None, .. })
    ));
    assert!(
        import(
            maf.replace("GRCh38", "GRCh37").as_bytes(),
            Format::Maf,
            &options()
        )
        .is_err()
    );
}
#[test]
fn annotated_vcf_checks_sample_genotype_and_filter() {
    let vcf = "##fileformat=VCFv4.2\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1\n12\t100\t.\tG\tA\t.\tPASS\tGENE=KRAS;CONSEQUENCE=missense;SOMATIC\tGT\t0/1\n";
    let r = import(vcf.as_bytes(), Format::Vcf, &options()).unwrap();
    assert!(matches!(
        r.features.features[0].value,
        Some(FeatureValue::Mutation {
            somatic: Some(true),
            ..
        })
    ));
    for bad in [
        vcf.replace("0/1", "0/0"),
        vcf.replace("PASS", "q10"),
        vcf.replace("GENE=KRAS;", ""),
        vcf.replace("\tA\t", "\tA,C\t"),
    ] {
        assert!(import(bad.as_bytes(), Format::Vcf, &options()).is_err());
    }
}
