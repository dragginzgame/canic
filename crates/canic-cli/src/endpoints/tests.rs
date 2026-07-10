use super::*;

// Ensure plain output renders function, mode, and signature columns.
#[test]
fn renders_plain_endpoint_signatures_as_table() {
    let endpoints = vec![
        EndpointEntry {
            name: "canic_log".to_string(),
            candid: "canic_log : (opt text, opt text, Level, PageRequest) -> () query;".to_string(),
            modes: vec![EndpointMode::Query],
            arguments: vec![
                test_endpoint_type("opt text"),
                test_endpoint_type("opt text"),
                test_endpoint_type("Level"),
                test_endpoint_type("PageRequest"),
            ],
            returns: Vec::new(),
        },
        EndpointEntry {
            name: "canic_import".to_string(),
            candid: "canic_import : (Envelope) -> (Result);".to_string(),
            modes: Vec::new(),
            arguments: vec![test_endpoint_type("Envelope")],
            returns: vec![test_endpoint_type("Result")],
        },
        EndpointEntry {
            name: "canic_response_capability_v1".to_string(),
            candid: "canic_response_capability_v1 : (Envelope) -> (Result) query oneway;"
                .to_string(),
            modes: vec![EndpointMode::Query, EndpointMode::Oneway],
            arguments: vec![test_endpoint_type("Envelope")],
            returns: vec![test_endpoint_type("Result")],
        },
    ];

    assert_eq!(
        render_plain_endpoints(&endpoints),
        [
            "FUNCTION                       MODE           SIGNATURE",
            "----------------------------   ------------   ----------------------------------------------",
            "canic_log                      query          (opt text, opt text, Level, PageRequest) -> ()",
            "canic_import                   update         (Envelope) -> (Result)",
            "canic_response_capability_v1   query oneway   (Envelope) -> (Result)",
        ]
        .join("\n")
    );
}

fn test_endpoint_type(candid: &str) -> EndpointType {
    EndpointType::Named {
        candid: candid.to_string(),
        cardinality: EndpointCardinality::Single,
        name: candid.to_string(),
        resolved: None,
    }
}

// Ensure endpoint options parse local and live lookup controls.
#[test]
fn parses_endpoint_options() {
    let options = EndpointsOptions::parse_info([
        OsString::from("test"),
        OsString::from("app"),
        OsString::from(crate::cli::globals::INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/bin/icp"),
        OsString::from("--json"),
    ])
    .expect("parse options");

    assert_eq!(options.deployment, "test");
    assert_eq!(options.canister, "app");
    assert_eq!(options.network.as_deref(), Some("local"));
    assert_eq!(options.icp, "/bin/icp");
    assert!(options.json);
}
