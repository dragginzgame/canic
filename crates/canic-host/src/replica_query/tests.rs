use super::*;

// Ensure readiness parsing accepts common command-line JSON result shapes.
#[test]
fn parse_ready_json_value_accepts_nested_true_shapes() {
    assert!(parse_ready_json_value(&serde_json::json!(true)));
    assert!(parse_ready_json_value(&serde_json::json!({ "Ok": true })));
    assert!(parse_ready_json_value(&serde_json::json!([{ "Ok": true }])));
    assert!(parse_ready_json_value(&serde_json::json!({
        "response_candid": "(true)"
    })));
}

// Ensure readiness parsing rejects false and non-boolean result shapes.
#[test]
fn parse_ready_json_value_rejects_false_shapes() {
    assert!(!parse_ready_json_value(&serde_json::json!(false)));
    assert!(!parse_ready_json_value(&serde_json::json!({ "Ok": false })));
    assert!(!parse_ready_json_value(&serde_json::json!("true")));
}

#[test]
fn decodes_bootstrap_status_response_bytes() {
    let bytes = Encode!(&canic_core::dto::state::BootstrapStatusResponse {
        ready: false,
        phase: "root:init:create_canisters".to_string(),
        last_error: Some("registry phase failed".to_string()),
    })
    .expect("encode bootstrap status");

    let status = decode_bootstrap_status_response(&bytes).expect("decode bootstrap status");

    assert!(!status.ready);
    assert_eq!(status.phase, "root:init:create_canisters");
    assert_eq!(status.last_error.as_deref(), Some("registry phase failed"));
}

#[test]
fn decodes_cycle_balance_response_bytes() {
    let response: Result<u128, canic_core::dto::error::Error> = Ok(99_999_000_000_000);
    let bytes = Encode!(&response).expect("encode cycle balance response");

    let cycles = decode_cycle_balance_response(&bytes).expect("decode cycle balance");

    assert_eq!(cycles, 99_999_000_000_000);
}

#[test]
fn decodes_subnet_registry_response_roles_and_cli_json() {
    let root = Principal::from_text("aaaaa-aa").expect("root principal");
    let child = Principal::anonymous();
    let response: Result<SubnetRegistryResponseWire, CanicErrorWire> =
        Ok(SubnetRegistryResponseWire(vec![
            SubnetRegistryEntryWire {
                pid: root,
                role: "root".to_string(),
                record: CanisterInfoWire {
                    pid: root,
                    role: "root".to_string(),
                    parent_pid: None,
                    module_hash: None,
                    created_at: 1,
                },
            },
            SubnetRegistryEntryWire {
                pid: child,
                role: "worker".to_string(),
                record: CanisterInfoWire {
                    pid: child,
                    role: "worker".to_string(),
                    parent_pid: Some(root),
                    module_hash: Some(vec![0xab, 0xcd]),
                    created_at: 2,
                },
            },
        ]));
    let bytes = Encode!(&response).expect("encode subnet registry response");

    let decoded = decode_subnet_registry_response(&bytes).expect("decode subnet registry");
    let registry_json = decoded.to_cli_json();

    assert_eq!(decoded.roles(), vec!["root", "worker"]);
    assert_eq!(registry_json["Ok"][0]["pid"], root.to_text());
    assert_eq!(registry_json["Ok"][1]["role"], "worker");
    assert_eq!(
        registry_json["Ok"][1]["record"]["parent_pid"],
        root.to_text()
    );
}

// Ensure direct local queries use the ICP CLI local endpoint fallback when no project port is configured.
#[test]
fn local_replica_endpoint_defaults_to_icp_cli_port() {
    assert_eq!(
        local_replica_endpoint_with_port(None, None),
        "http://127.0.0.1:8000"
    );
    assert_eq!(
        local_replica_endpoint_with_port(None, Some(8001)),
        "http://127.0.0.1:8001"
    );
    assert_eq!(
        local_replica_endpoint_with_port(Some("http://127.0.0.1:9000/"), Some(8001)),
        "http://127.0.0.1:9000"
    );
}

#[test]
fn parses_local_replica_root_key_from_json_status() {
    let root_key = parse_local_replica_root_key(br#"{"root_key":"308182"}"#);

    assert_eq!(root_key.as_deref(), Some("308182"));
}

#[test]
fn parses_local_replica_root_key_from_cbor_status() {
    #[derive(Serialize)]
    struct Status {
        #[serde(with = "serde_bytes")]
        root_key: Vec<u8>,
    }

    let body = serde_cbor::to_vec(&Status {
        root_key: vec![0x30, 0x81, 0x82],
    })
    .expect("encode cbor status");
    let root_key = parse_local_replica_root_key(&body);

    assert_eq!(root_key.as_deref(), Some("308182"));
}

#[test]
fn rejects_blank_local_replica_root_key_status_values() {
    #[derive(Serialize)]
    struct Status {
        #[serde(with = "serde_bytes")]
        root_key: Vec<u8>,
    }

    assert_eq!(parse_local_replica_root_key(br#"{"root_key":"   "}"#), None);

    let body = serde_cbor::to_vec(&Status { root_key: vec![] })
        .expect("encode empty cbor status root key");

    assert_eq!(parse_local_replica_root_key(&body), None);
}
