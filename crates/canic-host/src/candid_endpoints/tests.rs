use super::*;

const CANDID: &str = r#"
type Nested = record { field : text };
service : (record { init : text }) -> {
  canic_ready : () -> (bool) query;
  "icrc10-supported-standards" : () -> (vec record { text; text }) query;
  canic_update : (Nested) -> (
  variant { Ok; Err : text },
);
}
"#;

// Ensure generated Candid service files can be reduced to endpoint signatures.
#[test]
fn parses_candid_service_endpoints() {
    let endpoints = parse_candid_service_endpoints(CANDID).expect("parse endpoints");
    let canic_ready = endpoints
        .iter()
        .find(|endpoint| endpoint.name == "canic_ready")
        .expect("canic_ready endpoint");
    let icrc10 = endpoints
        .iter()
        .find(|endpoint| endpoint.name == "icrc10-supported-standards")
        .expect("icrc10 endpoint");
    let canic_update = endpoints
        .iter()
        .find(|endpoint| endpoint.name == "canic_update")
        .expect("canic_update endpoint");

    assert_eq!(endpoints.len(), 3);
    assert_eq!(canic_ready.candid, "canic_ready : () -> (bool) query;");
    assert_eq!(canic_ready.modes, vec![EndpointMode::Query]);
    assert_eq!(
        icrc10.candid,
        "\"icrc10-supported-standards\" : () -> (vec record { text; text }) query;"
    );
    assert!(canic_update.modes.is_empty());
    assert_eq!(canic_update.arguments.len(), 1);
    std::assert_matches!(
        &canic_update.arguments[0],
        EndpointType::Named {
            name,
            resolved: Some(_),
            ..
        } if name == "Nested"
    );
    std::assert_matches!(
        &canic_update.returns[0],
        EndpointType::Variant { cases, .. } if cases.len() == 2
    );
}

// Ensure multiline argument lists are parsed as structured endpoint types.
#[test]
fn parses_multiline_endpoint_arguments() {
    let candid = r#"
service : {
  "import" : (
record {
  payload : text;
},
  ) -> (variant { Ok; Err : text });
}
"#;

    let endpoints = parse_candid_service_endpoints(candid).expect("parse endpoints");

    assert_eq!(endpoints.len(), 1);
    assert_eq!(endpoints[0].name, "import");
    assert!(endpoints[0].candid.starts_with("\"import\" : "));
    assert_eq!(endpoints[0].arguments.len(), 1);
    std::assert_matches!(
        &endpoints[0].arguments[0],
        EndpointType::Record { fields, .. }
            if fields.len() == 1 && fields[0].label == "payload"
    );
    std::assert_matches!(
        &endpoints[0].returns[0],
        EndpointType::Variant { cases, .. }
            if cases.iter().any(|case| case.label == "Ok")
                && cases.iter().any(|case| case.label == "Err")
    );
}

// Ensure multiple arguments retain cardinality and named type structure.
#[test]
fn parses_multiple_endpoint_arguments() {
    let candid = r"
type PageRequest = record { cursor : opt text };
service : {
  update : (opt text, record { items : vec record { id : nat; label : text } }, PageRequest) -> ();
}
";

    let endpoints = parse_candid_service_endpoints(candid).expect("parse endpoints");

    assert_eq!(endpoints.len(), 1);
    assert_eq!(endpoints[0].arguments.len(), 3);
    std::assert_matches!(
        &endpoints[0].arguments[0],
        EndpointType::Optional {
            cardinality: EndpointCardinality::Optional,
            inner,
            ..
        } if matches!(inner.as_ref(), EndpointType::Primitive { name, .. } if name == "text")
    );
    std::assert_matches!(
        &endpoints[0].arguments[1],
        EndpointType::Record { fields, .. }
            if fields.iter().any(|field| matches!(
                &field.ty,
                EndpointType::Vector {
                    cardinality: EndpointCardinality::Many,
                    ..
                }
            ))
    );
    std::assert_matches!(
        &endpoints[0].arguments[2],
        EndpointType::Named {
            name,
            resolved: Some(_),
            ..
        } if name == "PageRequest"
    );
}

// Ensure fields named service before the top-level service do not confuse discovery.
#[test]
fn ignores_service_named_record_fields() {
    let candid = r#"
type Envelope = record {
  "service" : text;
  payload : text;
};
service : {
  ready : () -> (bool) query;
}
"#;

    let endpoints = parse_candid_service_endpoints(candid).expect("parse endpoints");

    assert_eq!(endpoints.len(), 1);
    assert_eq!(endpoints[0].name, "ready");
    assert_eq!(endpoints[0].candid, "ready : () -> (bool) query;");
}

// Ensure JSON exposes structured types instead of requiring callers to parse strings.
#[test]
fn serializes_structured_endpoint_json() {
    let candid = r"
type MaybeText = opt text;
type Level = variant { Debug; Info; Error : text };
service : {
  canic_log : (MaybeText, Level) -> ();
}
";

    let endpoints = parse_candid_service_endpoints(candid).expect("parse endpoints");
    let json = serde_json::to_string(&endpoints[0]).expect("serialize endpoint");

    assert!(json.contains(r#""kind":"optional""#));
    assert!(json.contains(r#""cardinality":"optional""#));
    assert!(json.contains(r#""kind":"named""#));
    assert!(json.contains(r#""name":"MaybeText""#));
    assert!(json.contains(r#""name":"Level""#));
    assert!(json.contains(r#""kind":"variant""#));
    assert!(json.contains(r#""label":"Error""#));
}
