use super::super::catalog as deploy_catalog;
use super::super::inspect as deploy_inspect;
use super::super::output_format::JsonTextOutputFormat;
use super::fixtures::*;
use super::*;

#[test]
fn deploy_catalog_options_parse_list_defaults_to_text() {
    let options = deploy_catalog::DeployCatalogOptions::parse_list_test([
        OsString::from("--__canic-environment"),
        OsString::from("local"),
    ])
    .expect("parse catalog list");

    assert_eq!(options.fleet, None);
    assert_eq!(options.environment, "local");
    assert_eq!(options.format, JsonTextOutputFormat::Text);
    assert_eq!(options.output, None);
}

#[test]
fn deploy_catalog_options_parse_inspect_json_output() {
    let options = deploy_catalog::DeployCatalogOptions::parse_inspect_test([
        OsString::from("demo-local"),
        OsString::from("--json"),
        OsString::from("--output"),
        OsString::from("catalog.json"),
    ])
    .expect("parse catalog inspect");

    assert_eq!(options.fleet.as_deref(), Some("demo-local"));
    assert_eq!(options.environment, "local");
    assert_eq!(options.format, JsonTextOutputFormat::Json);
    assert_eq!(options.output, Some(PathBuf::from("catalog.json")));
}

#[test]
fn deploy_catalog_command_dispatches_list_and_inspect() {
    assert_catalog_dispatches_leaf("list", []);
    assert_catalog_dispatches_leaf("inspect", [OsString::from("demo-local")]);
}

#[test]
fn deploy_catalog_help_documents_network_scoped_fleet_authority() {
    let help = deploy_catalog::usage();
    let list_help = deploy_catalog::list_usage();
    let inspect_help = deploy_catalog::inspect_usage();

    assert!(help.contains(".canic/networks/<network-id>/fleets"));
    assert!(help.contains("canic deploy inspect catalog list"));
    assert!(help.contains("read-only local-state reports"));
    assert!(help.contains("live Fleets"));
    assert!(help.contains("infer Fleets from App names"));
    assert!(list_help.contains("--json"));
    assert!(list_help.contains("--output <path>"));
    assert!(inspect_help.contains("operator-facing label, not an App identity"));
}

#[test]
fn writes_catalog_json_output_file() {
    let out = temp_json_path("deploy-catalog-output.json");
    let options = deploy_catalog::DeployCatalogOptions {
        fleet: None,
        environment: "local".to_string(),
        format: JsonTextOutputFormat::Json,
        output: Some(out.clone()),
    };
    let report = sample_catalog_report();

    deploy_catalog::write_report(&options, &report).expect("write catalog");
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(&out).expect("read catalog")).expect("parse catalog");

    fs::remove_file(out).expect("clean catalog");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["entries"][0]["fleet_name"], "demo-local");
    assert_eq!(value["entries"][0]["app"], "demo");
    assert_eq!(value["canonical_network_id"], "01".repeat(32));
    assert!(value.get("envelope_schema").is_none());
}

fn assert_catalog_dispatches_leaf<const N: usize>(command: &'static str, args: [OsString; N]) {
    let expected_args = args.to_vec();
    let parsed = parse_subcommand(
        deploy_command(),
        std::iter::once(OsString::from("inspect"))
            .chain(std::iter::once(OsString::from("catalog")))
            .chain(std::iter::once(OsString::from(command)))
            .chain(args),
    )
    .expect("parse deploy inspect catalog")
    .expect("inspect command");

    assert_eq!(parsed.0, "inspect");
    let inspect = parse_subcommand(deploy_inspect::command(), parsed.1)
        .expect("parse nested inspect")
        .expect("catalog command");
    assert_eq!(inspect.0, "catalog");
    let nested = parse_subcommand(deploy_catalog::command(), inspect.1)
        .expect("parse nested catalog")
        .expect("catalog leaf command");
    assert_eq!(nested.0, command);
    assert_eq!(nested.1, expected_args);
}
