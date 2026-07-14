use super::super::catalog as deploy_catalog;
use super::super::inspect as deploy_inspect;
use super::super::output_format::JsonTextOutputFormat;
use super::fixtures::*;
use super::*;

#[test]
fn deploy_catalog_options_parse_list_defaults_to_text() {
    let options = deploy_catalog::DeployCatalogOptions::parse_list_test([
        OsString::from("--__canic-network"),
        OsString::from("local"),
    ])
    .expect("parse catalog list");

    assert_eq!(options.deployment, None);
    assert_eq!(options.network, "local");
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

    assert_eq!(options.deployment.as_deref(), Some("demo-local"));
    assert_eq!(options.network, "local");
    assert_eq!(options.format, JsonTextOutputFormat::Json);
    assert_eq!(options.output, Some(PathBuf::from("catalog.json")));
}

#[test]
fn deploy_catalog_command_dispatches_list_and_inspect() {
    assert_catalog_dispatches_leaf("list", []);
    assert_catalog_dispatches_leaf("inspect", [OsString::from("demo-local")]);
}

#[test]
fn deploy_catalog_help_documents_passive_deployment_target_scope() {
    let help = deploy_catalog::usage();
    let list_help = deploy_catalog::list_usage();
    let inspect_help = deploy_catalog::inspect_usage();

    assert!(help.contains("deployment targets recorded under .canic/<network>/deployments"));
    assert!(help.contains("canic deploy inspect catalog list"));
    assert!(help.contains("do not query"));
    assert!(help.contains("infer deployments from fleet-template names"));
    assert!(list_help.contains("--json"));
    assert!(list_help.contains("--output <path>"));
    assert!(inspect_help.contains("deployment target, not a fleet template"));
}

#[test]
fn writes_catalog_json_output_file() {
    let out = temp_json_path("deploy-catalog-output.json");
    let options = deploy_catalog::DeployCatalogOptions {
        deployment: None,
        network: "local".to_string(),
        format: JsonTextOutputFormat::Json,
        output: Some(out.clone()),
    };
    let report = sample_catalog_report();

    deploy_catalog::write_report(&options, &report).expect("write catalog");
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(&out).expect("read catalog")).expect("parse catalog");

    fs::remove_file(out).expect("clean catalog");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["entries"][0]["deployment"], "demo-local");
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
