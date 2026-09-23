use josh_core::sample::*;
use josh_features::{GeneMapper, process};

fn mapper() -> GeneMapper {
    GeneMapper::from_tsv(include_bytes!(
        "../../../fixtures/expression/hgnc-subset.tsv"
    ))
    .unwrap()
}
fn records(values: &[(&str, &str)]) -> Vec<ExpressionRecord> {
    values
        .iter()
        .enumerate()
        .map(|(i, (gene, value))| ExpressionRecord {
            original_gene_id: (*gene).into(),
            original_value: (*value).into(),
            source_record: i as u64 + 2,
            source_column: 2,
            raw_expression: None,
            transformed_expression: None,
            mapping: GeneMapping {
                status: MappingStatus::NotAttempted,
                gene: None,
                candidates: vec![],
            },
        })
        .collect()
}
fn config() -> ExpressionConfig {
    ExpressionConfig {
        units: ExpressionUnit::Tpm,
        platform: Some("declared test platform".into()),
        ..Default::default()
    }
}

#[test]
fn real_hgnc_symbols_ensembl_entrez_and_ids_resolve_to_same_identity() {
    let m = mapper();
    for id in ["TP53", "HGNC:11998", "ENSG00000141510", "7157"] {
        let r = m.resolve(id, GeneNamespace::Auto);
        assert_eq!(r.gene.unwrap().symbol, "TP53");
        assert_eq!(r.status, MappingStatus::Exact);
    }
    assert_eq!(
        m.resolve("ENSG00000141510.18", GeneNamespace::Auto).status,
        MappingStatus::VersionStripped
    );
    assert_eq!(
        m.resolve("ENSG00000141510.invalid", GeneNamespace::Auto)
            .status,
        MappingStatus::Unmapped
    );
    assert_eq!(
        m.resolve("tp53", GeneNamespace::Auto).status,
        MappingStatus::Unmapped
    );
}

#[test]
fn shared_alias_is_ambiguous_and_never_selects_first_gene() {
    let bytes=b"hgnc_id\tsymbol\tstatus\tensembl_gene_id\tentrez_id\talias_symbol\nHGNC:1\tEXAMPLE_A\tApproved\tENSG1\t1\tSHARED|OLD_A\nHGNC:2\tEXAMPLE_B\tApproved\tENSG2\t2\tSHARED\n";
    let m = GeneMapper::from_tsv(bytes).unwrap();
    let result = m.resolve("SHARED", GeneNamespace::Symbol);
    assert_eq!(result.status, MappingStatus::Ambiguous);
    assert!(result.gene.is_none());
    assert_eq!(result.candidates.len(), 2);
    assert_eq!(
        m.resolve("OLD_A", GeneNamespace::Symbol).status,
        MappingStatus::Alias
    );
}

#[test]
fn malformed_or_duplicate_mapping_assets_are_rejected() {
    assert!(GeneMapper::from_tsv(b"gene\tvalue\nTP53\t1\n").is_err());
    assert!(GeneMapper::from_tsv(b"hgnc_id\tsymbol\tstatus\tensembl_gene_id\tentrez_id\nHGNC:1\tEXAMPLE\tApproved\tENSG1\t1\nHGNC:1\tOTHER\tApproved\tENSG2\t2\n").is_err());
}

#[test]
fn log_transform_has_independent_expected_values_and_preserves_raw_values() {
    let mut r = records(&[("TP53", "0"), ("KRAS", "3"), ("KRT7", "7")]);
    let config = ExpressionConfig {
        transform: Transform::Log2OnePlus,
        ..config()
    };
    let qc = process(&mut r, &config, Some(&mapper()));
    assert_eq!(qc.status, QcStatus::Pass);
    for (i, expected) in [0.0, 2.0, 3.0].iter().enumerate() {
        assert!((r[i].transformed_expression.unwrap() - expected).abs() < 1e-12);
    }
    assert_eq!(r[1].original_value, "3");
    assert_eq!(r[1].raw_expression, Some(3.0));
    process(&mut r, &config, Some(&mapper()));
    assert!(
        (r[1].transformed_expression.unwrap() - 2.0).abs() < 1e-12,
        "never log a previous derived value"
    );
}

#[test]
fn missing_invalid_and_measured_zero_are_distinct() {
    let mut r = records(&[
        ("TP53", "0"),
        ("KRAS", "NA"),
        ("KRT7", "NaN"),
        ("KRT20", "inf"),
        ("CDX2", "-1"),
        ("NKX2-1", ""),
    ]);
    let qc = process(&mut r, &config(), Some(&mapper()));
    assert_eq!(
        (
            qc.measured_values,
            qc.zero_values,
            qc.missing_values,
            qc.invalid_values
        ),
        (1, 1, 2, 3)
    );
    assert_eq!(r[0].raw_expression, Some(0.0));
    assert_eq!(r[1].raw_expression, None);
    assert_eq!(qc.status, QcStatus::Blocked);
}

#[test]
fn counts_must_be_integral_but_z_scores_can_be_negative() {
    let mut r = records(&[("TP53", "1.2"), ("KRAS", "-3")]);
    let counts = ExpressionConfig {
        units: ExpressionUnit::Counts,
        ..config()
    };
    assert_eq!(process(&mut r, &counts, Some(&mapper())).invalid_values, 2);
    let scores = ExpressionConfig {
        units: ExpressionUnit::ZScore,
        ..config()
    };
    assert_eq!(process(&mut r, &scores, Some(&mapper())).invalid_values, 0);
    let invalid = ExpressionConfig {
        transform: Transform::Log2OnePlus,
        ..scores
    };
    assert_eq!(
        process(&mut r, &invalid, Some(&mapper())).status,
        QcStatus::Blocked
    );
    assert!(r.iter().all(|r| r.transformed_expression.is_none()));
}

#[test]
fn canonical_duplicates_are_reported_without_summing_or_dropping() {
    let mut r = records(&[("TP53", "3"), ("7157", "7")]);
    let qc = process(&mut r, &config(), Some(&mapper()));
    assert_eq!(qc.duplicate_gene_records, 1);
    assert_eq!(qc.status, QcStatus::Blocked);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].raw_expression, Some(3.0));
    assert_eq!(r[1].raw_expression, Some(7.0));
}

#[test]
fn unknown_units_and_absent_mapping_allow_exploration_but_warn() {
    let mut r = records(&[("TP53", "1")]);
    let qc = process(&mut r, &ExpressionConfig::default(), None);
    assert_eq!(qc.status, QcStatus::Warning);
    assert_eq!(qc.gene_id_mapping_rate, None);
    assert_eq!(r[0].mapping.status, MappingStatus::NotAttempted);
    assert!(
        qc.issues
            .iter()
            .any(|i| i.code == "expression_units_unknown")
    );
    assert_eq!(qc.reference_compatibility, "not_assessed_no_reference");
}

#[test]
fn counts_outside_exact_integer_range_are_rejected() {
    let mut r = records(&[("TP53", "9007199254740993")]);
    let c = ExpressionConfig {
        units: ExpressionUnit::Counts,
        ..config()
    };
    assert_eq!(process(&mut r, &c, Some(&mapper())).invalid_values, 1);
    assert_eq!(r[0].original_value, "9007199254740993");
    assert!(r[0].raw_expression.is_none());
}
