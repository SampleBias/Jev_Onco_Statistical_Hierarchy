//! Generated contract artifacts for the offline router.
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub fn documents() -> BTreeMap<String, Value> {
    let mut docs = josh_core::schema::documents();
    docs.insert("openapi.json".into(), openapi());
    docs.insert(
        "import-report.schema.json".into(),
        josh_core::schema::output::<josh_ingest::ImportReport>(),
    );
    docs.insert(
        "labels.schema.json".into(),
        josh_core::schema::output::<Vec<josh_ingest::LabelRecord>>(),
    );
    docs.insert(
        "splits.schema.json".into(),
        josh_core::schema::output::<josh_ingest::SplitManifest>(),
    );
    docs
}

fn rebase(value: &mut Value, name: &str) {
    match value {
        Value::Object(fields) => {
            if let Some(Value::String(reference)) = fields.get_mut("$ref")
                && let Some(path) = reference.strip_prefix('#')
            {
                *reference = format!("#/components/schemas/{name}{path}");
            }
            for child in fields.values_mut() {
                rebase(child, name);
            }
        }
        Value::Array(items) => {
            for item in items {
                rebase(item, name);
            }
        }
        _ => {}
    }
}

pub fn openapi() -> Value {
    let mut schemas = BTreeMap::from([
        ("Case", josh_core::schema::input::<josh_core::Case>()),
        (
            "GuidanceReport",
            josh_core::schema::output::<josh_core::guidance::GuidanceReport>(),
        ),
        (
            "ErrorEnvelope",
            josh_core::schema::output::<josh_core::errors::ErrorEnvelope>(),
        ),
        (
            "HealthResponse",
            josh_core::schema::output::<crate::HealthResponse>(),
        ),
        (
            "PreparedResponse",
            josh_core::schema::output::<crate::PreparedResponse>(),
        ),
        (
            "ResultRecord",
            josh_core::schema::output::<josh_core::ResultRecord>(),
        ),
    ]);
    for (name, schema) in &mut schemas {
        rebase(schema, name);
    }
    let response = |name: &str, description: &str| {
        json!({
            "description": description,
            "content": {"application/json": {"schema": {"$ref": format!("#/components/schemas/{name}")}}}
        })
    };
    let operation = |id: &str, result: &str| {
        json!({
            "operationId": id,
            "requestBody": {"required": true, "content": {"application/json": {"schema": {"$ref": "#/components/schemas/Case"}}}},
            "responses": {
                "200": response(result, "Offline research result; no provider request"),
                "400": response("ErrorEnvelope", "Malformed or trailing JSON"),
                "413": response("ErrorEnvelope", "Body exceeds 16384 bytes"),
                "415": response("ErrorEnvelope", "Unsupported media type"),
                "422": response("ErrorEnvelope", "Invalid case or JSON structure"),
                "default": response("ErrorEnvelope", "Structured failure")
            }
        })
    };
    json!({
        "openapi": "3.1.0",
        "info": {"title": "Jev Onco Statistical Hierarchy (JOSH) offline research API", "version": env!("CARGO_PKG_VERSION"), "description": "Research development. No live classification, authentication, persistence or clinical calibration."},
        "servers": [{"url": "http://127.0.0.1:3000"}],
        "paths": {
            "/health": {"get": {"operationId": "health", "responses": {"200": response("HealthResponse", "Local readiness only; Jev connectivity not checked")}}},
            "/v1/prepare": {"post": operation("prepareCase", "PreparedResponse")},
            "/v1/demo": {"post": operation("demoCase", "ResultRecord")},
            "/v1/guidance": {"post": operation("guidanceCase", "GuidanceReport")}
        },
        "components": {"schemas": schemas}
    })
}
