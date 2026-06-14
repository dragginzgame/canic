use super::parse_local_replica_root_key;
use serde::Serialize;

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
