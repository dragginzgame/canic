// Category C - Artifact / deployment test (embedded static config).
// This test relies on embedded config by design (test stub).
//
// admin-only: not part of canonical delegation flow.
// used for tests / tooling due to PocketIC limitations.

use candid::{Principal, decode_one, encode_args};
use canic_core::{
    dto::{
        abi::v1::CanisterInitPayload,
        auth::{
            DelegationCert, DelegationProof, DelegationProvisionRequest,
            DelegationProvisionResponse,
        },
        env::EnvBootstrapArgs,
        error::{Error, ErrorCode},
        subnet::SubnetIdentity,
        topology::{
            AppDirectoryArgs, SubnetDirectoryArgs, SubnetRegistryEntry, SubnetRegistryResponse,
        },
    },
    ids::{CanisterRole, SubnetRole},
    protocol,
};
use pocket_ic::PocketIcBuilder;
use serde::de::DeserializeOwned;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Once, OnceLock},
    time::{Duration, Instant},
};

const INSTALL_CYCLES: u128 = 100_000_000_000_000;
const CANISTER_PACKAGES: [&str; 2] = ["delegation_root_stub", "delegation_signer_stub"];
const BOOTSTRAP_TICK_LIMIT: usize = 40;
const BOOTSTRAP_TIMEOUT_SECS: u64 = 20;
const PREBUILT_WASM_DIR_ENV: &str = "CANIC_PREBUILT_WASM_DIR";
static BUILD_ONCE: Once = Once::new();
static ROOT_WASM: OnceLock<Vec<u8>> = OnceLock::new();
static SIGNER_WASM: OnceLock<Vec<u8>> = OnceLock::new();

#[test]
fn delegation_provision_requires_root_caller() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);

    let root_wasm = root_wasm(&workspace_root);

    let pic = PocketIcBuilder::new().with_application_subnet().build();

    let root_id = pic.create_canister();
    pic.add_cycles(root_id, INSTALL_CYCLES);
    pic.install_canister(root_id, root_wasm, root_init_args(), None);

    wait_for_ready(&pic, root_id);
    let signer_pid = fetch_signer_pid(&pic, root_id);
    let request = provision_request(signer_pid);

    let non_root = Principal::from_slice(&[2; 29]);
    let denied: Result<DelegationProvisionResponse, Error> = update_call_as(
        &pic,
        root_id,
        non_root,
        "canic_delegation_provision",
        (request.clone(),),
    );
    let denied = denied.expect_err("expected unauthorized provision");
    assert_eq!(denied.code, ErrorCode::Unauthorized);

    let ok: Result<DelegationProvisionResponse, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "canic_delegation_provision",
        (request,),
    );
    match ok {
        Ok(ok) => {
            assert_eq!(ok.proof.cert.signer_pid, signer_pid);
            assert!(ok.results.is_empty());
        }
        Err(err) => {
            // PocketIC update calls do not provide certified data,
            // so canister signatures may be unavailable.
            // Canister signatures require certified data and cannot be produced in PocketIC update calls.
            // This test only enforces the root-caller gate under PocketIC.
            assert_eq!(err.code, ErrorCode::Internal);
        }
    }
}

#[test]
fn signer_proof_rejects_mismatched_signer_pid() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);

    let root_wasm = root_wasm(&workspace_root);
    let shard_wasm = signer_wasm(&workspace_root);

    let pic = PocketIcBuilder::new().with_application_subnet().build();

    let root_id = pic.create_canister();
    pic.add_cycles(root_id, INSTALL_CYCLES);
    pic.install_canister(root_id, root_wasm, root_init_args(), None);

    let shard_id = pic.create_canister();
    pic.add_cycles(shard_id, INSTALL_CYCLES);
    pic.install_canister(shard_id, shard_wasm, shard_init_args(root_id), None);

    let proof = mismatched_signer_proof();
    let denied: Result<(), Error> = update_call_as(
        &pic,
        shard_id,
        root_id,
        "canic_delegation_set_signer_proof",
        (proof,),
    );
    let denied = denied.expect_err("expected signer proof mismatch to fail");
    assert_eq!(denied.code, ErrorCode::InvalidInput);
}

fn provision_request(signer_pid: Principal) -> DelegationProvisionRequest {
    let cert = DelegationCert {
        v: 1,
        signer_pid,
        audiences: vec!["app".to_string()],
        scopes: vec!["scope".to_string()],
        issued_at: 100,
        expires_at: 200,
    };

    DelegationProvisionRequest {
        cert,
        signer_targets: Vec::new(),
        verifier_targets: Vec::new(),
    }
}

fn root_init_args() -> Vec<u8> {
    encode_args((SubnetIdentity::Prime,)).expect("encode init args")
}

fn shard_init_args(root_pid: Principal) -> Vec<u8> {
    let env = EnvBootstrapArgs {
        prime_root_pid: Some(root_pid),
        subnet_role: Some(SubnetRole::PRIME),
        subnet_pid: Some(root_pid),
        root_pid: Some(root_pid),
        canister_role: Some(CanisterRole::from("user_shard")),
        parent_pid: Some(root_pid),
    };

    let payload = CanisterInitPayload {
        env,
        app_directory: AppDirectoryArgs(Vec::new()),
        subnet_directory: SubnetDirectoryArgs(Vec::new()),
    };

    encode_args((payload, None::<Vec<u8>>)).expect("encode init args")
}

fn mismatched_signer_proof() -> DelegationProof {
    let cert = DelegationCert {
        v: 1,
        signer_pid: Principal::from_slice(&[99; 29]),
        audiences: vec!["app".to_string()],
        scopes: vec!["scope".to_string()],
        issued_at: 100,
        expires_at: 200,
    };

    DelegationProof {
        cert,
        cert_sig: vec![1, 2, 3],
    }
}

fn update_call_as<T, A>(
    pic: &pocket_ic::PocketIc,
    canister_id: Principal,
    caller: Principal,
    method: &str,
    args: A,
) -> T
where
    T: candid::CandidType + DeserializeOwned,
    A: candid::utils::ArgumentEncoder,
{
    let payload = encode_args(args).expect("encode args");
    let result = pic
        .update_call(canister_id, caller, method, payload)
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

fn wait_for_ready(pic: &pocket_ic::PocketIc, canister_id: Principal) {
    let start = Instant::now();
    for _ in 0..BOOTSTRAP_TICK_LIMIT {
        assert!(
            start.elapsed() <= Duration::from_secs(BOOTSTRAP_TIMEOUT_SECS),
            "root did not signal readiness after {BOOTSTRAP_TIMEOUT_SECS}s"
        );
        pic.tick();
        if fetch_ready(pic, canister_id) {
            return;
        }
    }

    panic!("root did not signal readiness after {BOOTSTRAP_TICK_LIMIT} ticks");
}

fn fetch_ready(pic: &pocket_ic::PocketIc, canister_id: Principal) -> bool {
    query_call(pic, canister_id, protocol::CANIC_READY, ())
}

fn fetch_signer_pid(pic: &pocket_ic::PocketIc, root_id: Principal) -> Principal {
    let registry: Result<SubnetRegistryResponse, Error> =
        query_call(pic, root_id, protocol::CANIC_SUBNET_REGISTRY, ());
    let entries = registry.expect("query subnet registry application").0;

    entries
        .into_iter()
        .find(|entry: &SubnetRegistryEntry| !entry.role.is_root())
        .map(|entry| entry.pid)
        .expect("expected a non-root canister in subnet registry")
}

fn build_canisters_once(workspace_root: &PathBuf) {
    BUILD_ONCE.call_once(|| {
        if prebuilt_wasm_dir().is_some() {
            return;
        }

        let target_dir = workspace_root
            .join("target")
            .join("pic_delegation_provision");
        unsafe { env::set_var("CARGO_TARGET_DIR", &target_dir) };

        let mut cmd = Command::new("cargo");
        cmd.current_dir(workspace_root);
        cmd.env("CARGO_TARGET_DIR", &target_dir);
        cmd.env("DFX_NETWORK", "local");
        cmd.args(["build", "--release", "--target", "wasm32-unknown-unknown"]);
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

fn root_wasm(workspace_root: &Path) -> Vec<u8> {
    ROOT_WASM
        .get_or_init(|| read_wasm(workspace_root, "delegation_root_stub"))
        .clone()
}

fn signer_wasm(workspace_root: &Path) -> Vec<u8> {
    SIGNER_WASM
        .get_or_init(|| read_wasm(workspace_root, "delegation_signer_stub"))
        .clone()
}

fn wasm_path(workspace_root: &Path, crate_name: &str) -> PathBuf {
    if let Some(dir) = prebuilt_wasm_dir() {
        return dir.join(format!("{crate_name}.wasm"));
    }

    let target_dir =
        env::var("CARGO_TARGET_DIR").map_or_else(|_| workspace_root.join("target"), PathBuf::from);

    target_dir
        .join("wasm32-unknown-unknown")
        .join("release")
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
