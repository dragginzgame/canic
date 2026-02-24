// Category C - Artifact / deployment test (embedded static config).
// This test relies on embedded config by design (test stub).
//
// admin-only: not part of canonical delegation flow.

use candid::{Principal, decode_one, encode_args};
use canic_core::{
    dto::{
        abi::v1::CanisterInitPayload,
        auth::{
            DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProof,
            DelegationProvisionRequest, DelegationProvisionResponse,
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
const REQUIRE_THRESHOLD_KEYS_ENV: &str = "CANIC_REQUIRE_THRESHOLD_KEYS";
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
    let shard_pid = fetch_shard_pid(&pic, root_id);
    let request = provision_request(root_id, shard_pid);

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
    let ok = match ok {
        Ok(ok) => ok,
        Err(err) if threshold_key_unavailable(&err) => {
            assert!(
                !require_threshold_keys(),
                "threshold key unavailable while {REQUIRE_THRESHOLD_KEYS_ENV}=1: {}",
                err.message
            );
            eprintln!("skipping root provision success assertion: {}", err.message);
            return;
        }
        Err(err) => panic!("expected root caller provision to succeed: {err:?}"),
    };
    assert_eq!(ok.proof.cert.shard_pid, shard_pid);
    assert!(ok.results.is_empty());
}

#[test]
fn signer_proof_rejects_mismatched_shard_pid() {
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

#[test]
fn delegated_token_flow_enforces_subject_binding() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);

    let root_wasm = root_wasm(&workspace_root);
    let pic = PocketIcBuilder::new().with_application_subnet().build();

    let root_id = pic.create_canister();
    pic.add_cycles(root_id, INSTALL_CYCLES);
    pic.install_canister(root_id, root_wasm, root_init_args(), None);
    wait_for_ready(&pic, root_id);

    let signer_id = fetch_shard_pid(&pic, root_id);
    let now_secs = pic.get_time().as_nanos_since_unix_epoch() / 1_000_000_000;
    let scope = "test:verify".to_string();

    let provision = DelegationProvisionRequest {
        cert: DelegationCert {
            root_pid: root_id,
            shard_pid: signer_id,
            issued_at: now_secs.saturating_sub(5),
            expires_at: now_secs + 600,
            scopes: vec![scope.clone()],
            aud: vec![signer_id],
        },
        signer_targets: vec![signer_id],
        verifier_targets: Vec::new(),
    };

    if provision_or_skip(&pic, root_id, provision, "delegated token flow").is_none() {
        return;
    }

    let caller = p(9);
    let claims = DelegatedTokenClaims {
        sub: caller,
        shard_pid: signer_id,
        scopes: vec![scope],
        aud: vec![signer_id],
        iat: now_secs,
        exp: now_secs + 300,
    };

    let minted: Result<DelegatedToken, Error> = update_call_as(
        &pic,
        signer_id,
        Principal::anonymous(),
        "signer_mint_token",
        (claims,),
    );
    let token = minted.expect("expected token mint to succeed");

    let ok: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        caller,
        "signer_verify_token",
        (token.clone(),),
    );
    ok.expect("expected matching caller verification to succeed");

    let mismatch: Result<(), Error> =
        update_call_as(&pic, signer_id, p(10), "signer_verify_token", (token,));
    let mismatch = mismatch.expect_err("expected subject mismatch to fail");
    assert_eq!(mismatch.code, ErrorCode::Unauthorized);
}

#[test]
fn delegated_token_rejects_expired_certificate() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);

    let root_wasm = root_wasm(&workspace_root);
    let pic = PocketIcBuilder::new().with_application_subnet().build();

    let root_id = pic.create_canister();
    pic.add_cycles(root_id, INSTALL_CYCLES);
    pic.install_canister(root_id, root_wasm, root_init_args(), None);
    wait_for_ready(&pic, root_id);

    let signer_id = fetch_shard_pid(&pic, root_id);
    let now_secs = pic.get_time().as_nanos_since_unix_epoch() / 1_000_000_000;
    let scope = "test:verify".to_string();
    let cert_exp = now_secs + 2;

    let provision = DelegationProvisionRequest {
        cert: DelegationCert {
            root_pid: root_id,
            shard_pid: signer_id,
            issued_at: now_secs.saturating_sub(5),
            expires_at: cert_exp,
            scopes: vec![scope.clone()],
            aud: vec![signer_id],
        },
        signer_targets: vec![signer_id],
        verifier_targets: Vec::new(),
    };
    if provision_or_skip(&pic, root_id, provision, "expired certificate flow").is_none() {
        return;
    }

    let caller = p(11);
    let claims = DelegatedTokenClaims {
        sub: caller,
        shard_pid: signer_id,
        scopes: vec![scope],
        aud: vec![signer_id],
        iat: now_secs,
        exp: cert_exp,
    };

    let minted: Result<DelegatedToken, Error> = update_call_as(
        &pic,
        signer_id,
        Principal::anonymous(),
        "signer_mint_token",
        (claims,),
    );
    let token = minted.expect("expected token mint to succeed");

    let ok: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        caller,
        "signer_verify_token",
        (token.clone(),),
    );
    ok.expect("expected verification to succeed before expiry");

    pic.advance_time(Duration::from_secs(3));
    pic.tick();

    let expired: Result<(), Error> =
        update_call_as(&pic, signer_id, caller, "signer_verify_token", (token,));
    let expired = expired.expect_err("expected verification to fail after expiry");
    assert_eq!(expired.code, ErrorCode::Unauthorized);
}

#[test]
fn delegated_token_rejects_cross_shard_token_reuse() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);

    let root_wasm = root_wasm(&workspace_root);
    let shard_wasm = signer_wasm(&workspace_root);
    let pic = PocketIcBuilder::new().with_application_subnet().build();

    let root_id = pic.create_canister();
    pic.add_cycles(root_id, INSTALL_CYCLES);
    pic.install_canister(root_id, root_wasm, root_init_args(), None);
    wait_for_ready(&pic, root_id);

    let signer_a = fetch_shard_pid(&pic, root_id);

    let signer_b = pic.create_canister();
    pic.add_cycles(signer_b, INSTALL_CYCLES);
    pic.install_canister(signer_b, shard_wasm, shard_init_args(root_id), None);

    let now_secs = pic.get_time().as_nanos_since_unix_epoch() / 1_000_000_000;
    let scope = "test:verify".to_string();

    let provision_a = DelegationProvisionRequest {
        cert: DelegationCert {
            root_pid: root_id,
            shard_pid: signer_a,
            issued_at: now_secs.saturating_sub(5),
            expires_at: now_secs + 600,
            scopes: vec![scope.clone()],
            aud: vec![signer_a, signer_b],
        },
        signer_targets: vec![signer_a],
        verifier_targets: Vec::new(),
    };
    if provision_or_skip(
        &pic,
        root_id,
        provision_a,
        "cross-shard signer A provisioning",
    )
    .is_none()
    {
        return;
    }

    let provision_b = DelegationProvisionRequest {
        cert: DelegationCert {
            root_pid: root_id,
            shard_pid: signer_b,
            issued_at: now_secs.saturating_sub(5),
            expires_at: now_secs + 600,
            scopes: vec![scope.clone()],
            aud: vec![signer_b],
        },
        signer_targets: vec![signer_b],
        verifier_targets: Vec::new(),
    };
    let Some(provisioned_b) = provision_or_skip(
        &pic,
        root_id,
        provision_b,
        "cross-shard signer B provisioning",
    ) else {
        return;
    };

    let caller = p(12);
    let claims_a = DelegatedTokenClaims {
        sub: caller,
        shard_pid: signer_a,
        scopes: vec![scope],
        aud: vec![signer_a, signer_b],
        iat: now_secs,
        exp: now_secs + 300,
    };

    let minted: Result<DelegatedToken, Error> = update_call_as(
        &pic,
        signer_a,
        Principal::anonymous(),
        "signer_mint_token",
        (claims_a,),
    );
    let token_a = minted.expect("expected signer A token mint to succeed");

    let ok: Result<(), Error> = update_call_as(
        &pic,
        signer_a,
        caller,
        "signer_verify_token",
        (token_a.clone(),),
    );
    ok.expect("expected signer A verification to succeed");

    let cross_shard: Result<(), Error> = update_call_as(
        &pic,
        signer_b,
        caller,
        "signer_verify_token",
        (token_a.clone(),),
    );
    let cross_shard = cross_shard.expect_err("expected signer A token to fail on signer B");
    assert_eq!(cross_shard.code, ErrorCode::Unauthorized);

    let mut remapped = token_a;
    remapped.proof = provisioned_b.proof;
    remapped.claims.shard_pid = signer_b;
    remapped.claims.aud = vec![signer_b];

    let wrong_key: Result<(), Error> =
        update_call_as(&pic, signer_b, caller, "signer_verify_token", (remapped,));
    let wrong_key = wrong_key.expect_err("expected remapped token to fail signature verification");
    assert_eq!(wrong_key.code, ErrorCode::Unauthorized);
}

fn provision_request(root_pid: Principal, shard_pid: Principal) -> DelegationProvisionRequest {
    let cert = DelegationCert {
        root_pid,
        shard_pid,
        aud: vec![Principal::from_slice(&[1; 29])],
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
        root_pid: Principal::from_slice(&[9; 29]),
        shard_pid: Principal::from_slice(&[99; 29]),
        aud: vec![Principal::from_slice(&[1; 29])],
        scopes: vec!["scope".to_string()],
        issued_at: 100,
        expires_at: 200,
    };

    DelegationProof {
        cert,
        cert_sig: vec![1, 2, 3],
    }
}

const fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
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

fn fetch_shard_pid(pic: &pocket_ic::PocketIc, root_id: Principal) -> Principal {
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

fn threshold_key_unavailable(err: &Error) -> bool {
    err.message.contains("Requested unknown threshold key")
        || err.message.contains("existing keys: []")
}

fn provision_or_skip(
    pic: &pocket_ic::PocketIc,
    root_id: Principal,
    request: DelegationProvisionRequest,
    context: &str,
) -> Option<DelegationProvisionResponse> {
    let result: Result<DelegationProvisionResponse, Error> = update_call_as(
        pic,
        root_id,
        root_id,
        "canic_delegation_provision",
        (request,),
    );

    match result {
        Ok(response) => Some(response),
        Err(err) if threshold_key_unavailable(&err) => {
            assert!(
                !require_threshold_keys(),
                "threshold key unavailable while {REQUIRE_THRESHOLD_KEYS_ENV}=1: {}",
                err.message
            );
            eprintln!("skipping {context}: {}", err.message);
            None
        }
        Err(err) => panic!("expected root caller provision to succeed ({context}): {err:?}"),
    }
}

fn require_threshold_keys() -> bool {
    env::var(REQUIRE_THRESHOLD_KEYS_ENV)
        .map(|value| {
            value.eq_ignore_ascii_case("1")
                || value.eq_ignore_ascii_case("true")
                || value.eq_ignore_ascii_case("yes")
        })
        .unwrap_or(false)
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}
