// Category C - Artifact / deployment test (embedded static config).
// This test relies on embedded config by design (test stub).

use candid::{Principal, decode_one, encode_args, encode_one, utils::ArgumentEncoder};
use canic_core::dto::{
    auth::{AttestationKeyStatus, SignedRoleAttestation},
    error::{Error, ErrorCode},
    subnet::SubnetIdentity,
};
use pocket_ic::PocketIcBuilder;
use serde::de::DeserializeOwned;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Once,
    time::Duration,
};

const ROOT_INSTALL_CYCLES: u128 = 80_000_000_000_000;
const CANISTER_PACKAGES: [&str; 1] = ["delegation_root_stub"];
const PREBUILT_WASM_DIR_ENV: &str = "CANIC_PREBUILT_WASM_DIR";
static BUILD_ONCE: Once = Once::new();

#[test]
fn role_attestation_issue_and_verify_happy_path() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let root_id = install_root_canister(&pic, root_wasm);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");

    let verified: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_verify_role_attestation",
        (issued, 0u64),
    );
    verified.expect("attestation verification failed");
}

#[test]
fn role_attestation_verify_rejects_mismatched_caller() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let root_id = install_root_canister(&pic, root_wasm);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");

    let verified: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        Principal::anonymous(),
        "root_verify_role_attestation",
        (issued, 0u64),
    );
    let err = verified.expect_err("verification must fail for mismatched caller");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("subject mismatch"),
        "expected subject mismatch error, got: {err:?}"
    );
}

#[test]
fn role_attestation_verify_rejects_expired() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let root_id = install_root_canister(&pic, root_wasm);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (1u64, Some(root_id), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");

    pic.advance_time(Duration::from_secs(2));
    pic.tick();

    let verified: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_verify_role_attestation",
        (issued, 0u64),
    );
    let err = verified.expect_err("verification must fail for expired attestation");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("expired"),
        "expected expired error, got: {err:?}"
    );
}

#[test]
fn role_attestation_verify_rejects_audience_mismatch() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let root_id = install_root_canister(&pic, root_wasm);
    let wrong_audience = Principal::from_slice(&[9; 29]);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(wrong_audience), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");

    let verified: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_verify_role_attestation",
        (issued, 0u64),
    );
    let err = verified.expect_err("verification must fail for audience mismatch");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("audience mismatch"),
        "expected audience mismatch error, got: {err:?}"
    );
}

#[test]
fn role_attestation_verify_handles_rotated_key_grace_window() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let root_id = install_root_canister(&pic, root_wasm);

    let previous_key_id = 1_001u32;
    let previous_key_seed = 3u8;
    let current_key_id = 1_002u32;
    let current_key_seed = 4u8;

    let previous_attestation: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test_with_key",
        (
            60u64,
            Some(root_id),
            0u64,
            previous_key_id,
            previous_key_seed,
        ),
    );
    let previous_attestation = previous_attestation.expect("previous-key attestation failed");
    let grace_until = previous_attestation.payload.issued_at.saturating_add(5);

    let set_keys: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_set_test_attestation_key_set",
        (vec![
            (
                previous_key_id,
                previous_key_seed,
                AttestationKeyStatus::Previous,
                None,
                Some(grace_until),
            ),
            (
                current_key_id,
                current_key_seed,
                AttestationKeyStatus::Current,
                Some(previous_attestation.payload.issued_at),
                None,
            ),
        ],),
    );
    set_keys.expect("seed key set failed");

    let verify_previous_in_grace: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_verify_role_attestation",
        (previous_attestation.clone(), 0u64),
    );
    verify_previous_in_grace.expect("previous key should verify during grace");

    pic.advance_time(Duration::from_secs(6));
    pic.tick();

    let verify_previous_after_grace: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_verify_role_attestation",
        (previous_attestation, 0u64),
    );
    let err = verify_previous_after_grace.expect_err("previous key must fail after grace");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("expired"),
        "expected key expiry error, got: {err:?}"
    );

    let current_attestation: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test_with_key",
        (60u64, Some(root_id), 0u64, current_key_id, current_key_seed),
    );
    let current_attestation = current_attestation.expect("current-key attestation failed");

    let verify_current_after_grace: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_verify_role_attestation",
        (current_attestation, 0u64),
    );
    verify_current_after_grace.expect("current key should verify after grace");
}

fn install_root_canister(pic: &pocket_ic::PocketIc, wasm: Vec<u8>) -> Principal {
    let root_id = pic.create_canister();
    pic.add_cycles(root_id, ROOT_INSTALL_CYCLES);
    pic.install_canister(
        root_id,
        wasm,
        encode_one(SubnetIdentity::Manual).expect("encode args"),
        None,
    );
    root_id
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
    A: ArgumentEncoder,
{
    let payload = encode_args(args).expect("encode args");
    let result = pic
        .update_call(canister_id, caller, method, payload)
        .expect("update_call failed");

    decode_one(&result).expect("decode response")
}

fn build_canisters_once(workspace_root: &PathBuf) {
    BUILD_ONCE.call_once_force(|_| {
        if prebuilt_wasm_dir().is_some() {
            return;
        }

        let target_dir = test_target_dir(workspace_root);
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

fn wasm_path(workspace_root: &Path, crate_name: &str) -> PathBuf {
    if let Some(dir) = prebuilt_wasm_dir() {
        return dir.join(format!("{crate_name}.wasm"));
    }

    let target_dir = test_target_dir(workspace_root);

    target_dir
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(format!("{crate_name}.wasm"))
}

fn prebuilt_wasm_dir() -> Option<PathBuf> {
    env::var(PREBUILT_WASM_DIR_ENV).ok().map(PathBuf::from)
}

fn test_target_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join("target").join("pic-wasm")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}
