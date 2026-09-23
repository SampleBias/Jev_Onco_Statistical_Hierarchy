//! JSON Schema 2020-12 generated from the actual public Rust types.
use schemars::{JsonSchema, generate::SchemaSettings};
use serde_json::Value;
use std::collections::BTreeMap;

pub fn input<T: JsonSchema>() -> Value {
    serde_json::to_value(
        SchemaSettings::draft2020_12()
            .into_generator()
            .into_root_schema_for::<T>(),
    )
    .expect("schemas contain only JSON values")
}

pub fn output<T: JsonSchema>() -> Value {
    serde_json::to_value(
        SchemaSettings::draft2020_12()
            .for_serialize()
            .into_generator()
            .into_root_schema_for::<T>(),
    )
    .expect("schemas contain only JSON values")
}

pub fn documents() -> BTreeMap<String, Value> {
    BTreeMap::from([
        ("case.schema.json".into(), input::<crate::Case>()),
        (
            "sample.schema.json".into(),
            input::<crate::sample::Sample>(),
        ),
        (
            "dataset.schema.json".into(),
            input::<crate::sample::DatasetManifest>(),
        ),
        (
            "expression-record.schema.json".into(),
            input::<crate::sample::ExpressionRecord>(),
        ),
        (
            "guidance.schema.json".into(),
            output::<crate::guidance::GuidanceReport>(),
        ),
        (
            "jev-request.schema.json".into(),
            output::<crate::JevRequest>(),
        ),
        (
            "jev-response.schema.json".into(),
            input::<crate::JevResponse>(),
        ),
        ("result.schema.json".into(), output::<crate::ResultRecord>()),
        (
            "error.schema.json".into(),
            output::<crate::errors::ErrorEnvelope>(),
        ),
    ])
}
