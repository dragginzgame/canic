use candid::{Principal, decode_one, encode_args};
use canic_core::dto::{
    abi::v1::CanisterInitPayload,
    env::EnvBootstrapArgs,
    error::Error,
    placement::sharding::{ShardingRegistryResponse, ShardingTenantsResponse},
    topology::{AppDirectoryArgs, SubnetDirectoryArgs},
};
use canic_core::ids::{CanisterRole, SubnetRole};
use pocket_ic::PocketIcBuilder;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

const INSTALL_CYCLES: u128 = 2_000_000_000_000;
const CANISTERS: [&str; 2] = ["sharding_root_stub", "shard_hub"];
const POOL_NAME: &str = "shards";

#[test]
fn sharding_bootstraps_first_shard_when_active_empty() {
    let workspace_root = workspace_root();
    build_canisters(&workspace_root);

    let root_wasm = read_wasm(&workspace_root, "sharding_root_stub");
    let shard_hub_wasm = read_wasm(&workspace_root, "shard_hub");

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

    let tenant = Principal::from_slice(&[10; 29]);
    let shard_pid: Result<Principal, Error> =
        update_call(&pic, shard_hub_id, "register_principal", (tenant,));
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

    let tenants: Result<ShardingTenantsResponse, Error> = query_call(
        &pic,
        shard_hub_id,
        "canic_sharding_tenants",
        (POOL_NAME.to_string(), shard_pid),
    );
    let tenants = tenants.expect("tenants query failed");
    assert_eq!(tenants.0, vec![tenant.to_string()]);
}

#[test]
fn sharding_does_not_spawn_extra_shard_after_bootstrap() {
    let workspace_root = workspace_root();
    build_canisters(&workspace_root);

    let root_wasm = read_wasm(&workspace_root, "sharding_root_stub");
    let shard_hub_wasm = read_wasm(&workspace_root, "shard_hub");

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

    let tenant_a = Principal::from_slice(&[10; 29]);
    let tenant_b = Principal::from_slice(&[11; 29]);

    let first: Result<Principal, Error> =
        update_call(&pic, shard_hub_id, "register_principal", (tenant_a,));
    let first = first.expect("register_principal tenant_a failed");

    let second: Result<Principal, Error> =
        update_call(&pic, shard_hub_id, "register_principal", (tenant_b,));
    let second = second.expect("register_principal tenant_b failed");

    assert_eq!(first, second);

    let registry: Result<ShardingRegistryResponse, Error> =
        query_call(&pic, shard_hub_id, "canic_sharding_registry", ());
    let registry = registry.expect("registry query failed");

    let pool_entries: Vec<_> = registry
        .0
        .into_iter()
        .filter(|entry| entry.entry.pool == POOL_NAME)
        .collect();

    assert_eq!(pool_entries.len(), 1);
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

    encode_args((payload, None)).expect("encode init args")
}

fn update_call<T, A>(pic: &pocket_ic::PocketIc, canister_id: Principal, method: &str, args: A) -> T
where
    T: candid::CandidType + candid::de::DeserializeOwned,
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
    T: candid::CandidType + candid::de::DeserializeOwned,
    A: candid::utils::ArgumentEncoder,
{
    let payload = encode_args(args).expect("encode args");
    let result = pic
        .query_call(canister_id, Principal::anonymous(), method, payload)
        .expect("query_call failed");

    decode_one(&result).expect("decode response")
}

fn build_canisters(workspace_root: &PathBuf) {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(workspace_root);
    cmd.env("DFX_NETWORK", "local");
    cmd.args(["build", "--target", "wasm32-unknown-unknown"]);
    for name in CANISTERS {
        cmd.args(["-p", name]);
    }

    let output = cmd.output().expect("failed to run cargo build");
    assert!(
        output.status.success(),
        "cargo build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn read_wasm(workspace_root: &Path, crate_name: &str) -> Vec<u8> {
    let wasm_path = wasm_path(workspace_root, crate_name);
    fs::read(&wasm_path).unwrap_or_else(|err| panic!("failed to read {crate_name} wasm: {err}"))
}

fn wasm_path(workspace_root: &Path, crate_name: &str) -> PathBuf {
    let target_dir =
        env::var("CARGO_TARGET_DIR").map_or_else(|_| workspace_root.join("target"), PathBuf::from);

    target_dir
        .join("wasm32-unknown-unknown")
        .join("debug")
        .join(format!("{crate_name}.wasm"))
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}
