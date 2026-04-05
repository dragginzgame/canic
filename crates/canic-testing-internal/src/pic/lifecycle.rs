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
        WasmBuildProfile, build_wasm_canisters, read_wasm, test_target_dir, wasm_artifacts_ready,
        workspace_root_for,
    },
    pic::{Pic, PicSerialGuard, acquire_pic_serial_guard, pic},
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
const INSTALL_CODE_COOLDOWN: Duration = Duration::from_secs(5 * 60);
static BUILD_ONCE: Once = Once::new();

///
/// LifecycleBoundaryFixture
///

pub struct LifecycleBoundaryFixture {
    pub pic: Pic,
    pub canic_wasm: Vec<u8>,
    pub authority_wasm: Vec<u8>,
    _serial_guard: PicSerialGuard,
}

impl LifecycleBoundaryFixture {
    /// Install one fresh non-root Canic test canister with the standard valid init payload.
    #[must_use]
    pub fn install_canic_canister(&self) -> Principal {
        let canister_id = self.pic.create_canister();
        self.pic.add_cycles(canister_id, INSTALL_CYCLES);
        self.pic.install_canister(
            canister_id,
            self.canic_wasm.clone(),
            encode_init_args(init_payload(canister_id)),
            None,
        );
        canister_id
    }

    /// Install one fresh non-Canic authority canister for negative upgrade cases.
    #[must_use]
    pub fn install_authority_canister(&self) -> Principal {
        let canister_id = self.pic.create_canister();
        self.pic.add_cycles(canister_id, INSTALL_CYCLES);
        self.pic.install_canister(
            canister_id,
            self.authority_wasm.clone(),
            encode_one(Principal::anonymous()).expect("encode authority init"),
            None,
        );
        canister_id
    }

    /// Wait out the install_code cooldown window inside the same PocketIC instance.
    pub fn wait_out_install_code_rate_limit(&self) {
        self.pic.advance_time(INSTALL_CODE_COOLDOWN);
        self.pic.tick_n(2);
    }

    /// Retry one install_code-like operation while PocketIC still reports rate limiting.
    pub fn retry_install_code_ok<T, F>(&self, mut op: F) -> Result<T, String>
    where
        F: FnMut() -> Result<T, String>,
    {
        let mut last_err = None;

        for _ in 0..INSTALL_CODE_RETRY_LIMIT {
            match op() {
                Ok(value) => return Ok(value),
                Err(err) if is_install_code_rate_limited(&err) => {
                    last_err = Some(err);
                    self.wait_out_install_code_rate_limit();
                }
                Err(err) => return Err(err),
            }
        }

        Err(last_err.unwrap_or_else(|| "install_code retry loop exhausted".to_string()))
    }

    /// Retry one install_code-like failure path while PocketIC still reports rate limiting.
    pub fn retry_install_code_err<F>(
        &self,
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

        self.wait_out_install_code_rate_limit();

        for _ in 1..INSTALL_CODE_RETRY_LIMIT {
            match op() {
                Ok(()) => return Ok(()),
                Err(err) if is_install_code_rate_limited(&err) => {
                    self.wait_out_install_code_rate_limit();
                }
                Err(err) => return Err(err),
            }
        }

        op()
    }
}

/// Build the lifecycle-boundary canister pair once and install them into one fresh PocketIC.
#[must_use]
pub fn install_lifecycle_boundary_fixture() -> LifecycleBoundaryFixture {
    let workspace_root = workspace_root();
    let target_dir = test_target_dir(&workspace_root, "pic-wasm");
    build_canisters_once(&workspace_root);

    LifecycleBoundaryFixture {
        canic_wasm: read_wasm(&target_dir, "canister_test", WasmBuildProfile::Fast),
        authority_wasm: read_wasm(&target_dir, "intent_authority", WasmBuildProfile::Fast),
        _serial_guard: acquire_pic_serial_guard(),
        pic: pic(),
    }
}

/// Encode the intentionally invalid init payload used by lifecycle boundary checks.
#[must_use]
pub fn invalid_init_args() -> Vec<u8> {
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

/// Encode the empty tuple argument used for no-payload upgrades.
#[must_use]
pub fn upgrade_args() -> Vec<u8> {
    encode_one(()).expect("encode upgrade")
}

/// Wait until the installed non-root canister reports ready.
pub fn wait_for_ready(pic: &Pic, canister_id: Principal, phase: &str) {
    pic.wait_for_ready(canister_id, READY_TICK_LIMIT, phase);
}

// Build the dedicated lifecycle-boundary canisters once into the shared test target dir.
fn build_canisters_once(workspace_root: &Path) {
    BUILD_ONCE.call_once(|| {
        let target_dir = test_target_dir(workspace_root, "pic-wasm");

        if wasm_artifacts_ready(&target_dir, &CANISTERS, WasmBuildProfile::Fast) {
            return;
        }

        build_wasm_canisters(
            workspace_root,
            &target_dir,
            &CANISTERS,
            WasmBuildProfile::Fast,
            &[],
        );
    });
}

// Encode the standard valid non-root init payload for the lifecycle-boundary test canister.
fn init_payload(canister_id: Principal) -> CanisterInitPayload {
    let app_directory = app_directory_args();
    let subnet_directory = subnet_directory_args(canister_id);
    let root_pid = dummy_principal(1);

    let env = EnvBootstrapArgs {
        prime_root_pid: Some(root_pid),
        subnet_role: Some(SubnetRole::PRIME),
        subnet_pid: Some(dummy_principal(2)),
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

// Encode one init payload through the standard tuple boundary expected by Canic canisters.
fn encode_init_args(payload: CanisterInitPayload) -> Vec<u8> {
    encode_args::<(CanisterInitPayload, Option<Vec<u8>>)>((payload, None))
        .expect("encode init args")
}

// Build the minimal app-directory view used by lifecycle-boundary installs.
fn app_directory_args() -> AppDirectoryArgs {
    let roles = [USER_HUB, SCALE_HUB];
    AppDirectoryArgs(directory_entries(&roles, None, 10))
}

// Build the subnet directory view used by lifecycle-boundary installs.
fn subnet_directory_args(canister_id: Principal) -> SubnetDirectoryArgs {
    let roles = [APP, USER_HUB, SCALE_HUB, TEST];
    let override_role = Some((TEST, canister_id));
    SubnetDirectoryArgs(directory_entries(&roles, override_role, 20))
}

// Build deterministic directory entries with one optional explicit role override.
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
                let pid = dummy_principal(next_id);
                next_id = next_id.saturating_add(1);
                pid
            }
        } else {
            let pid = dummy_principal(next_id);
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

// Detect the PocketIC install_code rate-limit error string shape.
fn is_install_code_rate_limited(message: &str) -> bool {
    message.contains("CanisterInstallCodeRateLimited")
}

fn workspace_root() -> PathBuf {
    workspace_root_for(env!("CARGO_MANIFEST_DIR"))
}

const fn dummy_principal(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}
