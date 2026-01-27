// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use candid::{Principal, encode_args, encode_one};
use canic::{
    dto::{
        abi::v1::CanisterInitPayload,
        env::EnvBootstrapArgs,
        topology::{AppDirectoryArgs, DirectoryEntryInput, SubnetDirectoryArgs},
    },
    ids::{CanisterRole, SubnetRole},
};
use canic_internal::canister::{APP, SCALE_HUB, SHARD_HUB, TEST, USER_HUB};
use canic_testkit::pic::pic;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

const INSTALL_CYCLES: u128 = 1_000_000_000_000;
const CANISTERS: [&str; 2] = ["canister_test", "intent_authority"];

const fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

#[test]
fn lifecycle_boundary_traps_are_phase_correct() {
    let workspace_root = workspace_root();
    build_canisters(&workspace_root);

    let canic_wasm = read_wasm(&workspace_root, "canister_test");
    let authority_wasm = read_wasm(&workspace_root, "intent_authority");
    let pic = pic();

    let canic_id = pic.create_canister();
    pic.add_cycles(canic_id, INSTALL_CYCLES);
    let init_args = encode_init_args(init_payload(canic_id));

    let install = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pic.install_canister(canic_id, canic_wasm.clone(), init_args, None);
    }));
    assert!(install.is_ok(), "install panicked for canic canister");

    let reinstall_err = pic
        .reinstall_canister(canic_id, canic_wasm.clone(), invalid_init_args(), None)
        .expect_err("reinstall should fail");
    assert_phase_error("init", &reinstall_err);

    let authority_id = pic.create_canister();
    pic.add_cycles(authority_id, INSTALL_CYCLES);
    pic.install_canister(
        authority_id,
        authority_wasm,
        encode_one(Principal::anonymous()).expect("encode authority init"),
        None,
    );

    let upgrade_err = pic
        .upgrade_canister(
            authority_id,
            canic_wasm,
            encode_one(()).expect("encode upgrade"),
            None,
        )
        .expect_err("upgrade should fail");
    assert_phase_error("post_upgrade", &upgrade_err);
}

fn assert_phase_error(phase: &str, err: &impl ToString) {
    let message = err.to_string();
    assert!(
        message.contains(&format!("{phase}:")),
        "missing {phase} prefix: {message}"
    );
    assert!(
        !message.contains("Internal"),
        "unexpected internal error: {message}"
    );
}

fn init_payload(canister_id: Principal) -> CanisterInitPayload {
    let app_directory = app_directory_args();
    let subnet_directory = subnet_directory_args(canister_id);
    let root_pid = p(1);

    let env = EnvBootstrapArgs {
        prime_root_pid: Some(root_pid),
        subnet_role: Some(SubnetRole::PRIME),
        subnet_pid: Some(p(2)),
        root_pid: Some(root_pid),
        canister_role: Some(TEST),
        parent_pid: Some(root_pid),
    };

    CanisterInitPayload {
        env,
        app_directory,
        subnet_directory,
    }
}

fn invalid_init_args() -> Vec<u8> {
    let payload = CanisterInitPayload {
        env: EnvBootstrapArgs {
            prime_root_pid: None,
            subnet_role: None,
            subnet_pid: None,
            root_pid: None,
            canister_role: None,
            parent_pid: None,
        },
        app_directory: AppDirectoryArgs(Vec::new()),
        subnet_directory: SubnetDirectoryArgs(Vec::new()),
    };

    encode_init_args(payload)
}

fn encode_init_args(payload: CanisterInitPayload) -> Vec<u8> {
    encode_args::<(CanisterInitPayload, Option<Vec<u8>>)>((payload, None))
        .expect("encode init args")
}

fn app_directory_args() -> AppDirectoryArgs {
    let roles = [USER_HUB, SCALE_HUB, SHARD_HUB];
    AppDirectoryArgs(directory_entries(&roles, None, 10))
}

fn subnet_directory_args(canister_id: Principal) -> SubnetDirectoryArgs {
    let roles = [APP, USER_HUB, SCALE_HUB, SHARD_HUB, TEST];
    let override_role = Some((TEST, canister_id));
    SubnetDirectoryArgs(directory_entries(&roles, override_role, 20))
}

fn directory_entries(
    roles: &[CanisterRole],
    override_role: Option<(CanisterRole, Principal)>,
    mut next_id: u8,
) -> Vec<DirectoryEntryInput> {
    let mut entries = Vec::new();

    for role in roles {
        let pid = if let Some((override_role, override_pid)) = &override_role {
            if role == override_role {
                *override_pid
            } else {
                let pid = p(next_id);
                next_id = next_id.saturating_add(1);
                pid
            }
        } else {
            let pid = p(next_id);
            next_id = next_id.saturating_add(1);
            pid
        };

        entries.push(DirectoryEntryInput {
            role: role.clone(),
            pid,
        });
    }

    entries
}

fn build_canisters(workspace_root: &PathBuf) {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(workspace_root);
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
