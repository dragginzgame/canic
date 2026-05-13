use candid::{decode_args, decode_one};
use canic::{
    dto::abi::v1::CanisterInitPayload,
    ids::{CanisterRole, SubnetRole},
};
use canic_testing_internal::{
    canister::{APP, MINIMAL, SCALE_HUB, SCALE_REPLICA, TEST, USER_HUB, USER_SHARD, WASM_STORE},
    pic::{invalid_init_args, upgrade_args},
};

// Verify canonical test role constants stay aligned with canister role names.
#[test]
fn canister_role_constants_have_expected_names() {
    let roles = [
        (APP, "app"),
        (MINIMAL, "minimal"),
        (SCALE_HUB, "scale_hub"),
        (SCALE_REPLICA, "scale_replica"),
        (TEST, "test"),
        (WASM_STORE, "wasm_store"),
        (USER_HUB, "user_hub"),
        (USER_SHARD, "user_shard"),
    ];

    for (role, expected) in roles {
        assert_eq!(role.as_str(), expected);
    }
}

// Verify the invalid lifecycle init fixture encodes the intended missing env.
#[test]
fn invalid_init_args_encode_missing_env_fields() {
    let (payload, user_payload): (CanisterInitPayload, Option<Vec<u8>>) =
        decode_args(&invalid_init_args()).expect("decode invalid init args");

    assert!(user_payload.is_none());
    assert!(payload.env.prime_root_pid.is_none());
    assert!(payload.env.subnet_role.is_none());
    assert!(payload.env.subnet_pid.is_none());
    assert!(payload.env.root_pid.is_none());
    assert!(payload.env.canister_role.is_none());
    assert!(payload.env.parent_pid.is_none());
    assert!(payload.app_index.0.is_empty());
    assert!(payload.subnet_index.0.is_empty());
}

// Verify the upgrade fixture is the empty tuple expected by no-payload upgrades.
#[test]
fn upgrade_args_encode_empty_tuple() {
    decode_one::<()>(&upgrade_args()).expect("decode upgrade args");
}

// Verify role value helpers used by fixture constants remain available.
#[test]
fn role_constants_match_core_role_helpers() {
    assert_eq!(CanisterRole::WASM_STORE, WASM_STORE);
    assert!(WASM_STORE.is_wasm_store());
    assert_eq!(SubnetRole::PRIME.as_str(), "prime");
}
