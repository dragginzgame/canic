// Category C - Artifact / deployment test (embedded config).
// This test helper relies on embedded production config by design.

use crate::root_cached_support::RootSetup;
use candid::{Principal, encode_args};
use canic::{
    dto::{
        abi::v1::CanisterInitPayload,
        env::EnvBootstrapArgs,
        topology::{AppIndexArgs, IndexEntryInput, SubnetIndexArgs},
    },
    ids::{CanisterRole, SubnetRole},
};
use canic_reference_support::canister;
use canic_testing_internal::pic::upgrade_args;
use canic_testkit::artifacts::workspace_root_for;
use std::{fs, path::PathBuf, time::Duration};

const READY_TICK_LIMIT: usize = 120;
const INSTALL_CODE_RETRY_LIMIT: usize = 4;
const INSTALL_CODE_COOLDOWN: Duration = Duration::from_mins(5);
const TEST_WASM_RELATIVE: &str = ".dfx/local/canisters/test/test.wasm.gz";
const USER_SHARD_WASM_RELATIVE: &str = ".dfx/local/canisters/user_shard/user_shard.wasm.gz";

/// Reinstall the verifier at its existing principal to simulate local verifier
/// proof-cache loss while preserving topology identity.
pub fn reinstall_test_verifier(setup: &RootSetup, test_pid: Principal) {
    log_step(&format!("reinstall verifier test={test_pid}"));
    setup
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    setup
        .pic
        .retry_install_code_ok(INSTALL_CODE_RETRY_LIMIT, INSTALL_CODE_COOLDOWN, || {
            setup
                .pic
                .reinstall_canister(
                    test_pid,
                    read_release_wasm(TEST_WASM_RELATIVE),
                    encode_test_reinstall_args(setup),
                    Some(setup.root_id),
                )
                .map_err(|err| err.to_string())
        })
        .expect("test verifier reinstall should succeed");
    setup
        .pic
        .wait_for_ready(test_pid, READY_TICK_LIMIT, "test verifier reinstall");
}

/// Upgrade the signer shard after verifier reinstall while preserving topology identity.
pub fn upgrade_user_shard_signer(setup: &RootSetup, shard_pid: Principal) {
    log_step(&format!("upgrade signer shard={shard_pid}"));
    setup
        .pic
        .wait_out_install_code_rate_limit(INSTALL_CODE_COOLDOWN);
    setup
        .pic
        .retry_install_code_ok(INSTALL_CODE_RETRY_LIMIT, INSTALL_CODE_COOLDOWN, || {
            setup
                .pic
                .upgrade_canister(
                    shard_pid,
                    read_release_wasm(USER_SHARD_WASM_RELATIVE),
                    upgrade_args(),
                    Some(setup.root_id),
                )
                .map_err(|err| err.to_string())
        })
        .expect("user shard upgrade should succeed");
    setup
        .pic
        .wait_for_ready(shard_pid, READY_TICK_LIMIT, "user shard post-upgrade");
}

// Emit one delegation lifecycle harness progress line.
fn log_step(step: &str) {
    canic::cdk::println!("[delegation_flow] {step}");
}

// Build the standard non-root init payload for reinstalling the test verifier.
fn encode_test_reinstall_args(setup: &RootSetup) -> Vec<u8> {
    let payload = CanisterInitPayload {
        env: EnvBootstrapArgs {
            prime_root_pid: Some(setup.root_id),
            subnet_role: Some(SubnetRole::PRIME),
            subnet_pid: Some(setup.root_id),
            root_pid: Some(setup.root_id),
            canister_role: Some(canister::TEST),
            parent_pid: Some(setup.root_id),
        },
        app_index: AppIndexArgs(vec![IndexEntryInput {
            role: canister::USER_HUB,
            pid: role_pid(setup, &canister::USER_HUB),
        }]),
        subnet_index: SubnetIndexArgs(
            setup
                .subnet_index
                .iter()
                .map(|(role, pid)| IndexEntryInput {
                    role: role.clone(),
                    pid: *pid,
                })
                .collect(),
        ),
    };

    encode_args::<(CanisterInitPayload, Option<Vec<u8>>)>((payload, None))
        .expect("encode test verifier reinstall args")
}

// Read one DFX-built release wasm artifact by workspace-relative path.
fn read_release_wasm(relative: &str) -> Vec<u8> {
    let path = workspace_root().join(relative);
    fs::read(&path).unwrap_or_else(|err| panic!("read {} failed: {err}", path.display()))
}

// Return the workspace root for this integration-test crate.
fn workspace_root() -> PathBuf {
    workspace_root_for(env!("CARGO_MANIFEST_DIR"))
}

// Resolve one role principal from the setup's cached subnet index.
fn role_pid(setup: &RootSetup, role: &CanisterRole) -> Principal {
    setup
        .subnet_index
        .get(role)
        .copied()
        .unwrap_or_else(|| panic!("{role} must exist in subnet index"))
}
