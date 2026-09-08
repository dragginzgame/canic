use super::*;

// Ensure plain output renders function, mode, and signature columns.
#[test]
fn renders_plain_endpoint_signatures_as_table() {
    let endpoints = vec![
        EndpointEntry {
            name: "canic_observability".to_string(),
            candid: "canic_observability : (opt text, opt text, Level, PageRequest) -> () query;"
                .to_string(),
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
            name: "canic_command".to_string(),
            candid: "canic_command : (Envelope) -> (Result);".to_string(),
            modes: Vec::new(),
            arguments: vec![test_endpoint_type("Envelope")],
            returns: vec![test_endpoint_type("Result")],
        },
        EndpointEntry {
            name: "application_stream".to_string(),
            candid: "application_stream : (Envelope) -> (Result) query oneway;".to_string(),
            modes: vec![EndpointMode::Query, EndpointMode::Oneway],
            arguments: vec![test_endpoint_type("Envelope")],
            returns: vec![test_endpoint_type("Result")],
        },
    ];

    let rendered = render_plain_endpoints(&endpoints);
    let lines = rendered.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), endpoints.len() + 2);
    let mode_column = lines[0].find("MODE").expect("mode column");
    let signature_column = lines[0].find("SIGNATURE").expect("signature column");
    let expected = [
        ("query", "(opt text, opt text, Level, PageRequest) -> ()"),
        ("update", "(Envelope) -> (Result)"),
        ("query oneway", "(Envelope) -> (Result)"),
    ];
    for ((line, endpoint), (mode, signature)) in lines[2..].iter().zip(&endpoints).zip(expected) {
        assert_eq!(line[..mode_column].trim(), endpoint.name);
        assert_eq!(line[mode_column..signature_column].trim(), mode);
        assert_eq!(line[signature_column..].trim(), signature);
    }
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
    assert_eq!(options.icp, "/bin/icp");
    assert!(options.json);
}
