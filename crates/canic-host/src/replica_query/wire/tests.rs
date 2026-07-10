use super::{
    CanisterInfoWire, SubnetRegistryEntryWire, SubnetRegistryResponseWire,
    decode_bootstrap_status_response, decode_cycle_balance_response,
    decode_subnet_registry_response,
};
use candid::{Encode, Principal};

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
    let response: Result<SubnetRegistryResponseWire, canic_core::dto::error::Error> =
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
