use std::collections::BTreeMap;

pub fn taxonomy() -> BTreeMap<String, String> {
    [
        ("lung", "Primary lung malignancy"),
        ("breast", "Primary breast malignancy"),
        ("colorectal", "Primary colon or rectal malignancy"),
        ("pancreas", "Primary pancreatic malignancy"),
        ("biliary_tract", "Primary biliary tract malignancy"),
        (
            "upper_gastrointestinal",
            "Primary gastric or esophageal malignancy",
        ),
        ("renal", "Primary kidney malignancy"),
        ("urothelial", "Primary urothelial malignancy"),
        ("prostate", "Primary prostate malignancy"),
        ("gynecologic", "Primary gynecologic malignancy"),
        ("thyroid", "Primary thyroid malignancy"),
        ("melanoma", "Melanocytic malignancy"),
        (
            "other_origin",
            "Evidence supports an origin outside the listed categories",
        ),
        (
            "insufficient_evidence",
            "Evidence is missing, conflicting, or does not support assigning a primary origin",
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v.into()))
    .collect()
}
