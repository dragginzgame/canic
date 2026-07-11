use super::parse_local_replica_root_key;

#[test]
fn parses_local_replica_root_key_from_json_status() {
    let root_key = parse_local_replica_root_key(br#"{"root_key":"308182"}"#);

    assert_eq!(root_key.as_deref(), Some("308182"));
}

#[test]
fn parses_local_replica_root_key_from_cbor_status() {
    let body = [
        0xa1, 0x68, b'r', b'o', b'o', b't', b'_', b'k', b'e', b'y', 0x43, 0x30, 0x81, 0x82,
    ];
    let root_key = parse_local_replica_root_key(&body);

    assert_eq!(root_key.as_deref(), Some("308182"));
}

#[test]
fn rejects_blank_local_replica_root_key_status_values() {
    assert_eq!(parse_local_replica_root_key(br#"{"root_key":"   "}"#), None);

    let body = [
        0xa1, 0x68, b'r', b'o', b'o', b't', b'_', b'k', b'e', b'y', 0x40,
    ];

    assert_eq!(parse_local_replica_root_key(&body), None);
}
