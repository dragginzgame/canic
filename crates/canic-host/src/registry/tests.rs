use super::{
    RegistryEntry, RegistryParseError, parse_registry_entries, registry_entries_from_response,
};
use candid::{CandidType, Encode, Principal};
use canic_core::{
    cdk::utils::hash::hex_bytes,
    dto::{
        canister::CanisterInfo,
        error::{Error as CanicError, ErrorCode},
        topology::{SubnetRegistryEntry, SubnetRegistryResponse},
    },
    ids::CanisterRole,
};

const ROOT_TEXT: &str = "aaaaa-aa";
const APP_TEXT: &str = "renrk-eyaaa-aaaaa-aaada-cai";

#[test]
fn parses_canonical_registry_response_bytes() {
    let root = Principal::from_text(ROOT_TEXT).expect("root principal");
    let app = Principal::from_text(APP_TEXT).expect("app principal");
    let response = Ok::<_, CanicError>(SubnetRegistryResponse(vec![
        registry_entry(root, "root", root, "root", None, None),
        registry_entry(app, "app", app, "app", Some(root), Some(vec![1, 171])),
    ]));

    let entries = parse_registry_entries(&response_json(&response)).expect("parse registry");

    assert_eq!(
        entries,
        vec![
            RegistryEntry {
                pid: ROOT_TEXT.to_string(),
                role: Some("root".to_string()),
                parent_pid: None,
                module_hash: None,
            },
            RegistryEntry {
                pid: APP_TEXT.to_string(),
                role: Some("app".to_string()),
                parent_pid: Some(ROOT_TEXT.to_string()),
                module_hash: Some("01ab".to_string()),
            },
        ]
    );
}

#[test]
fn rejects_redundant_principal_mismatch() {
    let root = Principal::from_text(ROOT_TEXT).expect("root principal");
    let app = Principal::from_text(APP_TEXT).expect("app principal");
    let response =
        SubnetRegistryResponse(vec![registry_entry(root, "root", app, "root", None, None)]);

    assert!(matches!(
        registry_entries_from_response(response),
        Err(RegistryParseError::PrincipalMismatch { entry_pid, record_pid })
            if entry_pid == ROOT_TEXT && record_pid == APP_TEXT
    ));
}

#[test]
fn rejects_redundant_role_mismatch() {
    let root = Principal::from_text(ROOT_TEXT).expect("root principal");
    let response = SubnetRegistryResponse(vec![registry_entry(
        root, "root", root, "worker", None, None,
    )]);

    assert!(matches!(
        registry_entries_from_response(response),
        Err(RegistryParseError::RoleMismatch {
            pid,
            entry_role,
            record_role,
        }) if pid == ROOT_TEXT && entry_role == "root" && record_role == "worker"
    ));
}

#[test]
fn preserves_typed_registry_rejection() {
    let response = Err::<SubnetRegistryResponse, _>(CanicError::unavailable("not ready"));
    let error = parse_registry_entries(&response_json(&response)).expect_err("reject response");

    let RegistryParseError::Response(crate::icp::IcpJsonResponseError::Rejected(error)) = error
    else {
        panic!("expected typed registry rejection");
    };
    assert_eq!(error.code, ErrorCode::Unavailable);
    assert_eq!(error.message, "not ready");
}

fn registry_entry(
    pid: Principal,
    role: &str,
    record_pid: Principal,
    record_role: &str,
    parent_pid: Option<Principal>,
    module_hash: Option<Vec<u8>>,
) -> SubnetRegistryEntry {
    SubnetRegistryEntry {
        pid,
        role: CanisterRole::owned(role.to_string()),
        record: CanisterInfo {
            pid: record_pid,
            role: CanisterRole::owned(record_role.to_string()),
            parent_pid,
            module_hash,
            created_at: 1,
        },
    }
}

fn response_json<T: CandidType>(response: &T) -> String {
    let bytes = Encode!(response).expect("encode response");
    serde_json::json!({ "response_bytes": hex_bytes(bytes) }).to_string()
}
