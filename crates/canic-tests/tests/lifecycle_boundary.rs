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
use canic_internal::canister::{APP, SCALE_HUB, TEST, USER_HUB};
use canic_testkit::{
    artifacts::{
        WasmBuildProfile, build_wasm_canisters, prebuilt_wasm_dir, read_wasm, test_target_dir,
        wasm_artifacts_ready, workspace_root_for,
    },
    pic::pic,
};
use std::{
    path::{Path, PathBuf},
    sync::Once,
    time::Duration,
};

const INSTALL_CYCLES: u128 = 1_000_000_000_000;
const READY_TICK_LIMIT: usize = 120;
const INSTALL_CODE_RETRY_LIMIT: usize = 4;
const CANISTERS: [&str; 2] = ["canister_test", "intent_authority"];
const PREBUILT_WASM_DIR_ENV: &str = "CANIC_PREBUILT_WASM_DIR";
const INSTALL_CODE_COOLDOWN: Duration = Duration::from_secs(5 * 60);
static BUILD_ONCE: Once = Once::new();

const fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

#[test]
fn lifecycle_boundary_traps_are_phase_correct() {
    let workspace_root = workspace_root();
    let target_dir = test_target_dir(&workspace_root, "pic-wasm");
    build_canisters_once(&workspace_root);

    let canic_wasm = read_wasm(
        &target_dir,
        "canister_test",
        WasmBuildProfile::Release,
        PREBUILT_WASM_DIR_ENV,
    );
    let authority_wasm = read_wasm(
        &target_dir,
        "intent_authority",
        WasmBuildProfile::Release,
        PREBUILT_WASM_DIR_ENV,
    );
    let pic = pic();

    let canic_id = pic.create_canister();
    pic.add_cycles(canic_id, INSTALL_CYCLES);
    let init_args = encode_init_args(init_payload(canic_id));

    let install = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pic.install_canister(canic_id, canic_wasm.clone(), init_args, None);
    }));
    assert!(install.is_ok(), "install panicked for canic canister");
    wait_out_install_code_rate_limit(&pic);

    let reinstall_err = pic
        .reinstall_canister(canic_id, canic_wasm.clone(), invalid_init_args(), None)
        .map_err(|err| err.to_string());
    let reinstall_err = retry_install_code_err(&pic, reinstall_err, || {
        pic.reinstall_canister(canic_id, canic_wasm.clone(), invalid_init_args(), None)
            .map_err(|err| err.to_string())
    })
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
    wait_out_install_code_rate_limit(&pic);

    let upgrade_err = pic
        .upgrade_canister(
            authority_id,
            canic_wasm.clone(),
            encode_one(()).expect("encode upgrade"),
            None,
        )
        .map_err(|err| err.to_string());
    let upgrade_err = retry_install_code_err(&pic, upgrade_err, || {
        pic.upgrade_canister(
            authority_id,
            canic_wasm.clone(),
            encode_one(()).expect("encode upgrade"),
            None,
        )
        .map_err(|err| err.to_string())
    })
    .expect_err("upgrade should fail");
    assert_phase_error("post_upgrade", &upgrade_err);
}

#[test]
fn non_root_post_upgrade_remains_ready_across_repeated_upgrades() {
    let workspace_root = workspace_root();
    let target_dir = test_target_dir(&workspace_root, "pic-wasm");
    build_canisters_once(&workspace_root);

    let canic_wasm = read_wasm(
        &target_dir,
        "canister_test",
        WasmBuildProfile::Release,
        PREBUILT_WASM_DIR_ENV,
    );
    let pic = pic();

    let canic_id = pic.create_canister();
    pic.add_cycles(canic_id, INSTALL_CYCLES);
    let init_args = encode_init_args(init_payload(canic_id));

    pic.install_canister(canic_id, canic_wasm.clone(), init_args, None);
    pic.wait_for_ready(canic_id, READY_TICK_LIMIT, "install");
    wait_out_install_code_rate_limit(&pic);

    for attempt in 1..=3 {
        retry_install_code_ok(&pic, || {
            pic.upgrade_canister(
                canic_id,
                canic_wasm.clone(),
                encode_one(()).expect("encode upgrade"),
                None,
            )
            .map_err(|err| err.to_string())
        })
        .unwrap_or_else(|err| panic!("upgrade attempt {attempt} should succeed: {err}"));

        pic.wait_for_ready(canic_id, READY_TICK_LIMIT, "post_upgrade");
        wait_out_install_code_rate_limit(&pic);
    }
}

#[test]
fn non_root_post_upgrade_failure_reports_phase_error() {
    let workspace_root = workspace_root();
    let target_dir = test_target_dir(&workspace_root, "pic-wasm");
    build_canisters_once(&workspace_root);

    let canic_wasm = read_wasm(
        &target_dir,
        "canister_test",
        WasmBuildProfile::Release,
        PREBUILT_WASM_DIR_ENV,
    );
    let authority_wasm = read_wasm(
        &target_dir,
        "intent_authority",
        WasmBuildProfile::Release,
        PREBUILT_WASM_DIR_ENV,
    );
    let pic = pic();

    let authority_id = pic.create_canister();
    pic.add_cycles(authority_id, INSTALL_CYCLES);
    pic.install_canister(
        authority_id,
        authority_wasm,
        encode_one(Principal::anonymous()).expect("encode authority init"),
        None,
    );
    wait_out_install_code_rate_limit(&pic);

    let upgrade_err = pic
        .upgrade_canister(
            authority_id,
            canic_wasm.clone(),
            encode_one(()).expect("encode upgrade"),
            None,
        )
        .map_err(|err| err.to_string());
    let upgrade_err = retry_install_code_err(&pic, upgrade_err, || {
        pic.upgrade_canister(
            authority_id,
            canic_wasm.clone(),
            encode_one(()).expect("encode upgrade"),
            None,
        )
        .map_err(|err| err.to_string())
    })
    .expect_err("upgrade should fail for non-canic stable state");

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

fn wait_out_install_code_rate_limit(pic: &canic_testkit::pic::Pic) {
    pic.advance_time(INSTALL_CODE_COOLDOWN);
    pic.tick_n(2);
}

fn retry_install_code_ok<T, F>(pic: &canic_testkit::pic::Pic, mut op: F) -> Result<T, String>
where
    F: FnMut() -> Result<T, String>,
{
    let mut last_err = None;

    for _ in 0..INSTALL_CODE_RETRY_LIMIT {
        match op() {
            Ok(value) => return Ok(value),
            Err(err) if is_install_code_rate_limited(&err) => {
                last_err = Some(err);
                wait_out_install_code_rate_limit(pic);
            }
            Err(err) => return Err(err),
        }
    }

    Err(last_err.unwrap_or_else(|| "install_code retry loop exhausted".to_string()))
}

fn retry_install_code_err<F>(
    pic: &canic_testkit::pic::Pic,
    first: Result<(), String>,
    mut op: F,
) -> Result<(), String>
where
    F: FnMut() -> Result<(), String>,
{
    match first {
        Ok(()) => return Ok(()),
        Err(err) if !is_install_code_rate_limited(&err) => return Err(err),
        Err(_) => {}
    }

    wait_out_install_code_rate_limit(pic);

    for _ in 1..INSTALL_CODE_RETRY_LIMIT {
        match op() {
            Ok(()) => return Ok(()),
            Err(err) if is_install_code_rate_limited(&err) => {
                wait_out_install_code_rate_limit(pic);
            }
            Err(err) => return Err(err),
        }
    }

    op()
}

fn is_install_code_rate_limited(message: &str) -> bool {
    message.contains("CanisterInstallCodeRateLimited")
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
    let roles = [USER_HUB, SCALE_HUB];
    AppDirectoryArgs(directory_entries(&roles, None, 10))
}

fn subnet_directory_args(canister_id: Principal) -> SubnetDirectoryArgs {
    let roles = [APP, USER_HUB, SCALE_HUB, TEST];
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

fn build_canisters_once(workspace_root: &Path) {
    BUILD_ONCE.call_once(|| {
        let target_dir = test_target_dir(workspace_root, "pic-wasm");

        if prebuilt_wasm_dir(PREBUILT_WASM_DIR_ENV).is_some()
            || wasm_artifacts_ready(
                &target_dir,
                &CANISTERS,
                WasmBuildProfile::Release,
                PREBUILT_WASM_DIR_ENV,
            )
        {
            return;
        }

        build_wasm_canisters(
            workspace_root,
            &target_dir,
            &CANISTERS,
            WasmBuildProfile::Release,
            &[],
        );
    });
}

fn workspace_root() -> PathBuf {
    workspace_root_for(env!("CARGO_MANIFEST_DIR"))
}
