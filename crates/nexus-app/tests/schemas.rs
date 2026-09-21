use nexus_app::{contracts, workflows};
use serde_json::{Value, json};

#[test]
fn published_contracts_match_rust_types_and_all_references_resolve() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts");
    for (name, generated) in contracts::documents() {
        let committed: Value =
            serde_json::from_slice(&std::fs::read(root.join(&name)).unwrap()).unwrap();
        assert_eq!(
            generated, committed,
            "regenerate {name}: cargo run -p nexus-app --example export_contracts"
        );
        check_refs(&generated, &generated);
    }
}

fn check_refs(root: &Value, value: &Value) {
    match value {
        Value::Object(fields) => {
            if let Some(Value::String(reference)) = fields.get("$ref") {
                assert!(reference.starts_with('#'), "no external reference fetching");
                assert!(
                    root.pointer(&reference[1..]).is_some(),
                    "unresolved {reference}"
                );
            }
            for child in fields.values() {
                check_refs(root, child);
            }
        }
        Value::Array(items) => {
            for item in items {
                check_refs(root, item);
            }
        }
        _ => {}
    }
}

#[test]
fn examples_and_real_outputs_validate_against_exported_schemas() {
    let docs = contracts::documents();
    let case = workflows::example_case();
    let examples = [
        ("case.schema.json", serde_json::to_value(&case).unwrap()),
        (
            "jev-request.schema.json",
            serde_json::to_value(nexus_core::prepare(&case).unwrap()).unwrap(),
        ),
        (
            "jev-response.schema.json",
            serde_json::to_value(nexus_core::mock_response()).unwrap(),
        ),
        (
            "result.schema.json",
            serde_json::to_value(workflows::demo(&case).unwrap()).unwrap(),
        ),
        (
            "error.schema.json",
            serde_json::to_value(nexus_core::errors::ErrorEnvelope::new(
                nexus_core::errors::ErrorCode::InvalidJson,
            ))
            .unwrap(),
        ),
    ];
    for (name, example) in examples {
        let validator = jsonschema::validator_for(&docs[name]).unwrap();
        assert!(validator.is_valid(&example), "{name}");
    }
}

#[test]
fn case_schema_rejects_wrong_versions_leakage_types_and_limits() {
    let schema = nexus_core::schema::input::<nexus_core::Case>();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let case = serde_json::to_value(workflows::example_case()).unwrap();
    for (field, value) in [
        ("known_primary", json!("lung")),
        ("schema_version", json!(2)),
        ("age_years", json!(121)),
        ("case_id", json!("bad id")),
        ("findings", json!({})),
    ] {
        let mut invalid = case.clone();
        invalid[field] = value;
        assert!(!validator.is_valid(&invalid), "{field}");
    }
    let mut optional = case;
    optional["age_years"] = Value::Null;
    optional["specimen_site"] = Value::Null;
    optional["sex_at_birth"] = Value::Null;
    assert!(validator.is_valid(&optional));
}

#[tokio::test]
async fn actual_routes_conform_to_generated_openapi_responses() {
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let spec = contracts::openapi();
    for (path, method, body, response_type) in [
        ("/health", "GET", String::new(), "HealthResponse"),
        (
            "/v1/prepare",
            "POST",
            serde_json::to_string(&workflows::example_case()).unwrap(),
            "PreparedResponse",
        ),
        (
            "/v1/demo",
            "POST",
            serde_json::to_string(&workflows::example_case()).unwrap(),
            "ResultRecord",
        ),
    ] {
        assert!(spec["paths"][path][method.to_lowercase()].is_object());
        let response = nexus_app::router()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(response.status().is_success());
        let body: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        // Validate the response within the document so its absolute component refs resolve.
        let schema = json!({"$schema": "https://json-schema.org/draft/2020-12/schema", "$ref": format!("#/components/schemas/{response_type}"), "components": spec["components"]});
        assert!(
            jsonschema::validator_for(&schema).unwrap().is_valid(&body),
            "{path}"
        );
    }
}
