use super::*;

#[test]
fn wire_endpoint_report_preserves_shapes_and_all_budgets() {
    let method = serde_json::json!({
        "name":"inspect", "id":"9001", "kind":"query", "mode":"query",
        "request":{"k":"unit"}, "response":{"k":"ref","name":"reply"},
        "requestBytes":128,"requestAllocation":256,"requestNodes":16,"requestDepth":8,
        "responseBytes":512,"responseAllocation":1024,"responseNodes":64,"responseDepth":16
    });
    let mut document = serde_json::json!({"schema_version":1,"methods":[method],
        "types":{"reply":{"k":"u","bits":64}}});
    let schema = model::read_schema(&serde_json::to_vec(&document).unwrap()).unwrap();
    assert_eq!(serde_json::to_value(&schema.methods[0]).unwrap(), method);
    assert_eq!(schema.types, document["types"]);
    let rendered = render_plain_endpoints(&schema.methods);
    assert!(rendered.contains("9001"));
    assert!(rendered.contains("request=128/256/16/8 response=512/1024/64/16"));
    document["methods"].as_array_mut().unwrap().push(method);
    assert!(matches!(
        model::read_schema(&serde_json::to_vec(&document).unwrap()),
        Err(EndpointsCommandError::Schema)
    ));
}

// Ensure endpoint options parse Fleet selection and output controls.
#[test]
fn parses_endpoint_options() {
    let options = EndpointsOptions::parse_info([
        OsString::from("test"),
        OsString::from("app"),
        OsString::from(crate::cli::globals::INTERNAL_ENVIRONMENT_OPTION),
        OsString::from("local"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/bin/icp"),
        OsString::from("--json"),
    ])
    .expect("parse options");

    assert_eq!(options.fleet, "test");
    assert_eq!(options.canister, "app");
    assert_eq!(options.environment.as_deref(), Some("local"));
    assert!(options.json);
}

#[test]
fn selected_build_requires_finalized_identity() {
    let id = "01".repeat(32);
    let options = EndpointsOptions::parse_info(
        ["demo", "app", "--release-build", &id, "--json"].map(OsString::from),
    )
    .unwrap();
    assert_eq!(options.release_build.unwrap().to_string(), id);
    let root = crate::test_support::TempDir::new("endpoint-build-selection");
    assert!(matches!(
        transport::endpoint_report_at(&root, &options),
        Err(EndpointsCommandError::BuiltSchema(
            canic_host::release_set::BuiltDeclarationError::ReleaseSet(_)
        ))
    ));
    assert!(matches!(
        EndpointsOptions::parse_info(
            ["demo", "app", "--release-build", "invalid",].map(OsString::from)
        ),
        Err(EndpointsCommandError::Usage(_))
    ));
}
