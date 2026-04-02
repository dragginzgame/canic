// Category C - Artifact / deployment test (embedded static config).
// This test relies on embedded config by design (test stub).

use candid::{Principal, decode_one, encode_args};
use canic_core::{
    dto::{
        abi::v1::CanisterInitPayload,
        env::EnvBootstrapArgs,
        error::Error,
        placement::sharding::{ShardingPartitionKeysResponse, ShardingRegistryResponse},
        topology::{AppDirectoryArgs, SubnetDirectoryArgs},
    },
    ids::{CanisterRole, SubnetRole},
};
use canic_testkit::{
    artifacts::{
        WasmBuildProfile, build_wasm_canisters, read_wasm, test_target_dir, wasm_artifacts_ready,
        workspace_root_for,
    },
    pic::{acquire_pic_serial_guard, pic},
};
use serde::de::DeserializeOwned;
use std::{
    path::{Path, PathBuf},
    sync::Once,
};

const ROOT_INSTALL_CYCLES: u128 = 80_000_000_000_000;
const USER_HUB_INSTALL_CYCLES: u128 = 20_000_000_000_000;
const CANISTER_PACKAGES: [&str; 2] = ["sharding_root_stub", "canister_user_hub"];
const POOL_NAME: &str = "user_shards";
static BUILD_ONCE: Once = Once::new();

#[test]
fn sharding_bootstraps_first_shard_when_active_empty() {
    let workspace_root = workspace_root();
    let target_dir = test_target_dir(&workspace_root, "pic-wasm");
    build_canisters_once(&workspace_root);

    let root_wasm = read_wasm(&target_dir, "sharding_root_stub", WasmBuildProfile::Release);
    let user_hub_wasm = read_wasm(&target_dir, "canister_user_hub", WasmBuildProfile::Release);

    let _serial_guard = acquire_pic_serial_guard();
    let pic = pic();

    let root_id = pic.create_canister();
    pic.add_cycles(root_id, ROOT_INSTALL_CYCLES);
    pic.install_canister(root_id, root_wasm, encode_args(()).unwrap(), None);

    let user_hub_id = pic.create_canister();
    pic.add_cycles(user_hub_id, USER_HUB_INSTALL_CYCLES);
    pic.install_canister(
        user_hub_id,
        user_hub_wasm,
        user_hub_init_args(root_id),
        None,
    );

    let partition_key = Principal::from_slice(&[10; 29]);
    let shard_pid: Result<Principal, Error> =
        update_call(&pic, user_hub_id, "create_account", (partition_key,));
    let shard_pid = shard_pid.expect("create_account failed");

    let registry: Result<ShardingRegistryResponse, Error> =
        query_call(&pic, user_hub_id, "canic_sharding_registry", ());
    let registry = registry.expect("registry query failed");

    let pool_entries: Vec<_> = registry
        .0
        .into_iter()
        .filter(|entry| entry.entry.pool == POOL_NAME)
        .collect();

    assert_eq!(pool_entries.len(), 1);
    assert_eq!(pool_entries[0].pid, shard_pid);

    let partition_keys: Result<ShardingPartitionKeysResponse, Error> = query_call(
        &pic,
        user_hub_id,
        "canic_sharding_partition_keys",
        (POOL_NAME.to_string(), shard_pid),
    );
    let partition_keys = partition_keys.expect("partition_keys query failed");
    assert_eq!(partition_keys.0, vec![partition_key.to_string()]);
}

#[test]
fn sharding_does_not_spawn_extra_shard_after_bootstrap() {
    let workspace_root = workspace_root();
    let target_dir = test_target_dir(&workspace_root, "pic-wasm");
    build_canisters_once(&workspace_root);

    let root_wasm = read_wasm(&target_dir, "sharding_root_stub", WasmBuildProfile::Release);
    let user_hub_wasm = read_wasm(&target_dir, "canister_user_hub", WasmBuildProfile::Release);

    let _serial_guard = acquire_pic_serial_guard();
    let pic = pic();

    let root_id = pic.create_canister();
    pic.add_cycles(root_id, ROOT_INSTALL_CYCLES);
    pic.install_canister(root_id, root_wasm, encode_args(()).unwrap(), None);

    let user_hub_id = pic.create_canister();
    pic.add_cycles(user_hub_id, USER_HUB_INSTALL_CYCLES);
    pic.install_canister(
        user_hub_id,
        user_hub_wasm,
        user_hub_init_args(root_id),
        None,
    );

    let partition_key_a = Principal::from_slice(&[10; 29]);
    let partition_key_b = Principal::from_slice(&[11; 29]);

    let first: Result<Principal, Error> =
        update_call(&pic, user_hub_id, "create_account", (partition_key_a,));
    let first = first.expect("create_account partition_key_a failed");

    let second: Result<Principal, Error> =
        update_call(&pic, user_hub_id, "create_account", (partition_key_b,));
    let second = second.expect("create_account partition_key_b failed");

    assert_eq!(first, second);

    let registry: Result<ShardingRegistryResponse, Error> =
        query_call(&pic, user_hub_id, "canic_sharding_registry", ());
    let registry = registry.expect("registry query failed");

    let pool_count = registry
        .0
        .into_iter()
        .filter(|entry| entry.entry.pool == POOL_NAME)
        .count();

    assert_eq!(pool_count, 1);
}

fn user_hub_init_args(root_pid: Principal) -> Vec<u8> {
    let env = EnvBootstrapArgs {
        prime_root_pid: Some(root_pid),
        subnet_role: Some(SubnetRole::PRIME),
        subnet_pid: Some(root_pid),
        root_pid: Some(root_pid),
        canister_role: Some(CanisterRole::from("user_hub")),
        parent_pid: Some(root_pid),
    };

    let payload = CanisterInitPayload {
        env,
        app_directory: AppDirectoryArgs(Vec::new()),
        subnet_directory: SubnetDirectoryArgs(Vec::new()),
    };

    encode_args((payload, None::<Vec<u8>>)).expect("encode init args")
}

fn update_call<T, A>(pic: &pocket_ic::PocketIc, canister_id: Principal, method: &str, args: A) -> T
where
    T: candid::CandidType + DeserializeOwned,
    A: candid::utils::ArgumentEncoder,
{
    let payload = encode_args(args).expect("encode args");
    let result = pic
        .update_call(canister_id, Principal::anonymous(), method, payload)
        .expect("update_call failed");

    decode_one(&result).expect("decode response")
}

fn query_call<T, A>(pic: &pocket_ic::PocketIc, canister_id: Principal, method: &str, args: A) -> T
where
    T: candid::CandidType + DeserializeOwned,
    A: candid::utils::ArgumentEncoder,
{
    let payload = encode_args(args).expect("encode args");
    let result = pic
        .query_call(canister_id, Principal::anonymous(), method, payload)
        .expect("query_call failed");

    decode_one(&result).expect("decode response")
}

fn build_canisters_once(workspace_root: &Path) {
    BUILD_ONCE.call_once(|| {
        let target_dir = test_target_dir(workspace_root, "pic-wasm");
        if wasm_artifacts_ready(&target_dir, &CANISTER_PACKAGES, WasmBuildProfile::Release) {
            return;
        }

        build_wasm_canisters(
            workspace_root,
            &target_dir,
            &CANISTER_PACKAGES,
            WasmBuildProfile::Release,
            &[],
        );
    });
}

fn workspace_root() -> PathBuf {
    workspace_root_for(env!("CARGO_MANIFEST_DIR"))
}
