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
use pocket_ic::PocketIcBuilder;
use serde::de::DeserializeOwned;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Once,
};

const INSTALL_CYCLES: u128 = 2_000_000_000_000;
const CANISTER_PACKAGES: [&str; 2] = ["sharding_root_stub", "canister_shard_hub"];
const POOL_NAME: &str = "shards";
const PREBUILT_WASM_DIR_ENV: &str = "CANIC_PREBUILT_WASM_DIR";
static BUILD_ONCE: Once = Once::new();

#[test]
fn sharding_bootstraps_first_shard_when_active_empty() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);

    let root_wasm = read_wasm(&workspace_root, "sharding_root_stub");
    let shard_hub_wasm = read_wasm(&workspace_root, "canister_shard_hub");

    let pic = PocketIcBuilder::new().with_application_subnet().build();

    let root_id = pic.create_canister();
    pic.add_cycles(root_id, INSTALL_CYCLES);
    pic.install_canister(root_id, root_wasm, encode_args(()).unwrap(), None);

    let shard_hub_id = pic.create_canister();
    pic.add_cycles(shard_hub_id, INSTALL_CYCLES);
    pic.install_canister(
        shard_hub_id,
        shard_hub_wasm,
        shard_hub_init_args(root_id),
        None,
    );

    let partition_key = Principal::from_slice(&[10; 29]);
    let shard_pid: Result<Principal, Error> =
        update_call(&pic, shard_hub_id, "register_principal", (partition_key,));
    let shard_pid = shard_pid.expect("register_principal failed");

    let registry: Result<ShardingRegistryResponse, Error> =
        query_call(&pic, shard_hub_id, "canic_sharding_registry", ());
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
        shard_hub_id,
        "canic_sharding_partition_keys",
        (POOL_NAME.to_string(), shard_pid),
    );
    let partition_keys = partition_keys.expect("partition_keys query failed");
    assert_eq!(partition_keys.0, vec![partition_key.to_string()]);
}

#[test]
fn sharding_does_not_spawn_extra_shard_after_bootstrap() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);

    let root_wasm = read_wasm(&workspace_root, "sharding_root_stub");
    let shard_hub_wasm = read_wasm(&workspace_root, "canister_shard_hub");

    let pic = PocketIcBuilder::new().with_application_subnet().build();

    let root_id = pic.create_canister();
    pic.add_cycles(root_id, INSTALL_CYCLES);
    pic.install_canister(root_id, root_wasm, encode_args(()).unwrap(), None);

    let shard_hub_id = pic.create_canister();
    pic.add_cycles(shard_hub_id, INSTALL_CYCLES);
    pic.install_canister(
        shard_hub_id,
        shard_hub_wasm,
        shard_hub_init_args(root_id),
        None,
    );

    let partition_key_a = Principal::from_slice(&[10; 29]);
    let partition_key_b = Principal::from_slice(&[11; 29]);

    let first: Result<Principal, Error> =
        update_call(&pic, shard_hub_id, "register_principal", (partition_key_a,));
    let first = first.expect("register_principal partition_key_a failed");

    let second: Result<Principal, Error> =
        update_call(&pic, shard_hub_id, "register_principal", (partition_key_b,));
    let second = second.expect("register_principal partition_key_b failed");

    assert_eq!(first, second);

    let registry: Result<ShardingRegistryResponse, Error> =
        query_call(&pic, shard_hub_id, "canic_sharding_registry", ());
    let registry = registry.expect("registry query failed");

    let pool_count = registry
        .0
        .into_iter()
        .filter(|entry| entry.entry.pool == POOL_NAME)
        .count();

    assert_eq!(pool_count, 1);
}

fn shard_hub_init_args(root_pid: Principal) -> Vec<u8> {
    let env = EnvBootstrapArgs {
        prime_root_pid: Some(root_pid),
        subnet_role: Some(SubnetRole::PRIME),
        subnet_pid: Some(root_pid),
        root_pid: Some(root_pid),
        canister_role: Some(CanisterRole::from("shard_hub")),
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

fn build_canisters_once(workspace_root: &PathBuf) {
    BUILD_ONCE.call_once(|| {
        if prebuilt_wasm_dir().is_some() {
            return;
        }

        let mut cmd = Command::new("cargo");
        cmd.current_dir(workspace_root);
        cmd.env("DFX_NETWORK", "local");
        cmd.args(["build", "--target", "wasm32-unknown-unknown"]);
        for name in CANISTER_PACKAGES {
            cmd.args(["-p", name]);
        }

        let output = cmd.output().expect("failed to run cargo build");
        assert!(
            output.status.success(),
            "cargo build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    });
}

fn read_wasm(workspace_root: &Path, crate_name: &str) -> Vec<u8> {
    let wasm_path = wasm_path(workspace_root, crate_name);
    fs::read(&wasm_path).unwrap_or_else(|err| panic!("failed to read {crate_name} wasm: {err}"))
}

fn wasm_path(workspace_root: &Path, crate_name: &str) -> PathBuf {
    if let Some(dir) = prebuilt_wasm_dir() {
        return dir.join(format!("{crate_name}.wasm"));
    }

    let target_dir =
        env::var("CARGO_TARGET_DIR").map_or_else(|_| workspace_root.join("target"), PathBuf::from);

    target_dir
        .join("wasm32-unknown-unknown")
        .join("debug")
        .join(format!("{crate_name}.wasm"))
}

fn prebuilt_wasm_dir() -> Option<PathBuf> {
    env::var(PREBUILT_WASM_DIR_ENV).ok().map(PathBuf::from)
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}
