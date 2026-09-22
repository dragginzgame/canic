use super::*;
use canic_core::ingress::payload_contract::{
    CANDID_PAYLOAD_CONTRACT_PREFIX, PayloadContract, UpdatePayloadDescriptor,
};

const CANDID: &str = r#"
type Nested = record { field : text };
type Status = variant { Readiness };
service : (record { init : text }) -> {
  canic_observability : (Status) -> (bool) query;
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
    let canic_observability = endpoints
        .iter()
        .find(|endpoint| endpoint.name == "canic_observability")
        .expect("canic_observability endpoint");
    let icrc10 = endpoints
        .iter()
        .find(|endpoint| endpoint.name == "icrc10-supported-standards")
        .expect("icrc10 endpoint");
    let canic_update = endpoints
        .iter()
        .find(|endpoint| endpoint.name == "canic_update")
        .expect("canic_update endpoint");

    assert_eq!(endpoints.len(), 3);
    assert_eq!(
        canic_observability.candid,
        "canic_observability : (Status) -> (bool) query;"
    );
    assert_eq!(canic_observability.modes, vec![EndpointMode::Query]);
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
  application_log : (MaybeText, Level) -> ();
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

fn payload_contract_candid(
    contract: &canic_core::ingress::payload_contract::PayloadContract,
) -> String {
    let mut encoded = Vec::new();
    ciborium::ser::into_writer(contract, &mut encoded).unwrap();
    format!(
        "service : {{ bare : () -> (); named : () -> (); read : () -> () query; variant_command : () -> (); }};\n{}{}\n",
        canic_core::ingress::payload_contract::CANDID_PAYLOAD_CONTRACT_PREFIX,
        canic_core::cdk::utils::hash::hex_bytes(&encoded)
    )
}

#[test]
fn payload_contract_distinguishes_defaults_overrides_query_and_variant_methods() {
    let contract = PayloadContract {
        schema_version: 1,
        default_update_ingress_max_bytes: 16384,
        update_overrides: vec![
            UpdatePayloadDescriptor {
                method: "named".into(),
                max_bytes: 49152,
            },
            UpdatePayloadDescriptor {
                method: "variant_command".into(),
                max_bytes: 65536,
            },
        ],
        variant_dependent_methods: vec!["variant_command".into()],
    };
    let endpoints = parse_candid_service_endpoints(&payload_contract_candid(&contract)).unwrap();
    let get = |name| {
        endpoints
            .iter()
            .find(|endpoint| endpoint.name == name)
            .unwrap()
            .payload_limits
            .as_ref()
    };
    assert_eq!(
        get("bare"),
        Some(&EndpointPayloadLimits {
            ingress_max_bytes: Some(16384),
            update_guard_max_bytes: None,
            ingress_basis: IngressPayloadBasis::ManagedDefault
        })
    );
    assert_eq!(
        get("named"),
        Some(&EndpointPayloadLimits {
            ingress_max_bytes: Some(49152),
            update_guard_max_bytes: Some(49152),
            ingress_basis: IngressPayloadBasis::ExplicitOverride
        })
    );
    assert_eq!(
        get("variant_command"),
        Some(&EndpointPayloadLimits {
            ingress_max_bytes: None,
            update_guard_max_bytes: Some(65536),
            ingress_basis: IngressPayloadBasis::VariantDependent
        })
    );
    assert_eq!(get("read"), None);
    let plain = parse_candid_service_endpoints("service : { bare : () -> (); };").unwrap();
    assert_eq!(plain[0].payload_limits, None);
    for change in 0..4 {
        let mut invalid = contract.clone();
        match change {
            0 => invalid.schema_version = 2,
            1 => invalid
                .update_overrides
                .push(invalid.update_overrides[0].clone()),
            2 => invalid.update_overrides[0].method = "read".into(),
            3 => invalid
                .variant_dependent_methods
                .push("variant_command".into()),
            _ => unreachable!(),
        }
        assert!(matches!(
            parse_candid_service_endpoints(&payload_contract_candid(&invalid)),
            Err(CandidEndpointError::InvalidPayloadContract)
        ));
    }
}

#[test]
fn payload_contract_rejects_duplicate_malformed_and_trailing_records() {
    let contract = PayloadContract {
        schema_version: 1,
        default_update_ingress_max_bytes: 16384,
        update_overrides: vec![],
        variant_dependent_methods: vec![],
    };
    let valid = payload_contract_candid(&contract);
    let record = valid
        .lines()
        .find(|line| line.starts_with(CANDID_PAYLOAD_CONTRACT_PREFIX))
        .unwrap();
    for document in [
        format!("{valid}{record}\n"),
        format!("service : {{}};\n{CANDID_PAYLOAD_CONTRACT_PREFIX}invalid"),
        format!("{}00\n", valid.trim_end()),
    ] {
        assert!(matches!(
            parse_candid_service_endpoints(&document),
            Err(CandidEndpointError::InvalidPayloadContract)
        ));
    }
}
