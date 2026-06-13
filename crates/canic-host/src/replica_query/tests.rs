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
