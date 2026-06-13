use super::*;
use candid::{CandidType, Encode, Principal};

const ROOT_TEXT: &str = "aaaaa-aa";
const APP_TEXT: &str = "renrk-eyaaa-aaaaa-aaada-cai";
const WORKER_TEXT: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

// Ensure registry parsing accepts the wrapped registry JSON shape.
#[test]
fn registry_entries_parse_wrapped_cli_json() {
    let entries = parse_registry_entries(&registry_json()).expect("parse registry");

    assert_eq!(
        entries,
        vec![
            RegistryEntry {
                pid: ROOT_TEXT.to_string(),
                role: Some("root".to_string()),
                kind: Some("root".to_string()),
                parent_pid: None,
                module_hash: None,
            },
            RegistryEntry {
                pid: APP_TEXT.to_string(),
                role: Some("app".to_string()),
                kind: Some("singleton".to_string()),
                parent_pid: Some(ROOT_TEXT.to_string()),
                module_hash: Some("01ab".to_string()),
            },
            RegistryEntry {
                pid: WORKER_TEXT.to_string(),
                role: Some("worker".to_string()),
                kind: Some("replica".to_string()),
                parent_pid: Some(APP_TEXT.to_string()),
                module_hash: Some(HASH.to_string()),
            },
        ]
    );
}

fn registry_json() -> String {
    serde_json::json!({
        "Ok": [
            {
                "pid": ROOT_TEXT,
                "role": "root",
                "record": {
                    "pid": ROOT_TEXT,
                    "role": "root",
                    "kind": "root",
                    "parent_pid": null,
                    "module_hash": null
                }
            },
            {
                "pid": APP_TEXT,
                "role": "app",
                "kind": "singleton",
                "record": {
                    "pid": APP_TEXT,
                    "role": "app",
                    "parent_pid": [ROOT_TEXT],
                    "module_hash": [1, 171]
                }
            },
            {
                "pid": WORKER_TEXT,
                "role": "worker",
                "kind": "replica",
                "record": {
                    "pid": WORKER_TEXT,
                    "role": "worker",
                    "parent_pid": [APP_TEXT],
                    "module_hash": HASH
                }
            }
        ]
    })
    .to_string()
}

#[test]
fn registry_entries_parse_icp_response_bytes_json() {
    #[derive(CandidType)]
    struct FullSubnetRegistryEntryWire {
        pid: Principal,
        role: String,
        record: FullCanisterInfoWire,
    }

    #[derive(CandidType)]
    struct FullSubnetRegistryResponseWire(Vec<FullSubnetRegistryEntryWire>);

    #[derive(CandidType)]
    struct FullCanisterInfoWire {
        pid: Principal,
        role: String,
        parent_pid: Option<Principal>,
        module_hash: Option<Vec<u8>>,
        created_at: u64,
    }

    let response = Ok::<_, CanicErrorWire>(FullSubnetRegistryResponseWire(vec![
        FullSubnetRegistryEntryWire {
            pid: Principal::from_text(ROOT_TEXT).expect("root principal"),
            role: "root".to_string(),
            record: FullCanisterInfoWire {
                pid: Principal::from_text(ROOT_TEXT).expect("root principal"),
                role: "root".to_string(),
                parent_pid: None,
                module_hash: None,
                created_at: 1,
            },
        },
        FullSubnetRegistryEntryWire {
            pid: Principal::from_text(APP_TEXT).expect("app principal"),
            role: "app".to_string(),
            record: FullCanisterInfoWire {
                pid: Principal::from_text(APP_TEXT).expect("app principal"),
                role: "app".to_string(),
                parent_pid: Some(Principal::from_text(ROOT_TEXT).expect("root principal")),
                module_hash: Some(vec![1, 171]),
                created_at: 2,
            },
        },
    ]));
    let bytes = candid::Encode!(&response).expect("encode registry response");
    let payload = serde_json::json!({
        "response_bytes": hex_bytes(&bytes),
        "response_candid": "(variant { Ok = vec { ... } })"
    })
    .to_string();
    let entries = parse_registry_entries(&payload).expect("parse response bytes registry");

    assert_eq!(
        entries,
        vec![
            RegistryEntry {
                pid: ROOT_TEXT.to_string(),
                role: Some("root".to_string()),
                kind: None,
                parent_pid: None,
                module_hash: None,
            },
            RegistryEntry {
                pid: APP_TEXT.to_string(),
                role: Some("app".to_string()),
                kind: None,
                parent_pid: Some(ROOT_TEXT.to_string()),
                module_hash: Some("01ab".to_string()),
            },
        ]
    );
}

#[test]
fn registry_entries_reject_invalid_response_bytes_hex() {
    let payload = serde_json::json!({
        "response_bytes": "not-hex"
    })
    .to_string();
    let err = parse_registry_entries(&payload).expect_err("reject invalid response bytes");

    std::assert_matches!(err, RegistryParseError::InvalidResponseBytes);
}

#[test]
fn registry_entries_surface_response_bytes_rejection() {
    let response = Err::<SubnetRegistryResponseWire, _>(CanicErrorWire {
        message: "not ready".into(),
    });
    let bytes = candid::Encode!(&response).expect("encode registry rejection");
    let payload = serde_json::json!({
        "response_bytes": hex_bytes(&bytes)
    })
    .to_string();
    let err = parse_registry_entries(&payload).expect_err("surface registry rejection");

    std::assert_matches!(err, RegistryParseError::Rejected(message) if message == "not ready");
}
