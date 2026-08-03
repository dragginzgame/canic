use candid::{decode_args, decode_one};
use canic::{
    dto::abi::v1::{CanisterInitAuthority, CanisterInitPayload},
    ids::{CanisterRole, ComponentSpecId},
};
use canic_testing_internal::{
    canister::{APP, SCALE_HUB, SCALE_REPLICA, TEST, USER_HUB, USER_SHARD, WASM_STORE},
    pic::{invalid_init_args, managed_test_init_identity, upgrade_args},
};
use ic_testkit::Fake;

// Verify canonical test role constants stay aligned with canister role names.
#[test]
fn canister_role_constants_have_expected_names() {
    let roles = [
        (APP, "app"),
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

// Verify the invalid lifecycle fixture binds a different managed Component canister.
#[test]
fn invalid_init_args_encode_mismatched_component_authority() {
    let (payload, user_payload): (CanisterInitPayload, Option<Vec<u8>>) =
        decode_args(&invalid_init_args()).expect("decode invalid init args");
    let identity = managed_test_init_identity();

    assert!(user_payload.is_none());
    assert_eq!(payload.install_id, identity.install_id);
    assert_eq!(payload.release_build_id, identity.release_build_id);
    let CanisterInitAuthority::Component { binding, .. } = payload.authority else {
        panic!("invalid fixture must carry Component authority");
    };
    assert_eq!(binding.authority.binding.fleet, identity.fleet);
    assert_eq!(binding.canister_id, Fake::principal(9));
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
    assert_eq!(
        ComponentSpecId::try_from(String::from("default"))
            .expect("default Component Spec ID")
            .as_str(),
        "default"
    );
}
