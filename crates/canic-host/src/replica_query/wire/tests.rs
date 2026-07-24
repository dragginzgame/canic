use super::{decode_cycle_balance_response, decode_subnet_registry_response};
use candid::{Encode, Principal};
use canic_core::{
    dto::{
        canister::CanisterInfo,
        error::Error as CanicError,
        topology::{SubnetRegistryEntry, SubnetRegistryResponse},
    },
    ids::CanisterRole,
};

#[test]
fn decodes_cycle_balance_response_bytes() {
    let response: Result<u128, CanicError> = Ok(99_999_000_000_000);
    let bytes = Encode!(&response).expect("encode cycle balance response");

    assert_eq!(
        decode_cycle_balance_response(&bytes).expect("decode cycle balance"),
        99_999_000_000_000
    );
}

#[test]
fn decodes_canonical_subnet_registry_response() {
    let root = Principal::from_text("aaaaa-aa").expect("root principal");
    let child = Principal::anonymous();
    let response: Result<SubnetRegistryResponse, CanicError> = Ok(SubnetRegistryResponse(vec![
        registry_entry(root, "root", None, None, 1),
        registry_entry(child, "worker", Some(root), Some(vec![0xab, 0xcd]), 2),
    ]));
    let bytes = Encode!(&response).expect("encode subnet registry response");

    let decoded = decode_subnet_registry_response(&bytes).expect("decode subnet registry");

    assert_eq!(decoded.0.len(), 2);
    assert_eq!(decoded.0[0].pid, root);
    assert_eq!(decoded.0[1].role.as_str(), "worker");
    assert_eq!(decoded.0[1].record.parent_pid, Some(root));
}

fn registry_entry(
    pid: Principal,
    role: &str,
    parent_pid: Option<Principal>,
    module_hash: Option<Vec<u8>>,
    created_at: u64,
) -> SubnetRegistryEntry {
    SubnetRegistryEntry {
        pid,
        role: CanisterRole::owned(role.to_string()),
        record: CanisterInfo {
            pid,
            role: CanisterRole::owned(role.to_string()),
            parent_pid,
            module_hash,
            created_at,
        },
    }
}
