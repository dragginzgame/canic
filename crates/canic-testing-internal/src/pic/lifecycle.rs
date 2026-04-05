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
    Fake,
    artifacts::{
        WasmBuildProfile, build_wasm_canisters, read_wasm, test_target_dir, wasm_artifacts_ready,
        workspace_root_for,
    },
    pic::{Pic, PicSerialGuard, acquire_pic_serial_guard, pic},
};
use std::{
    path::{Path, PathBuf},
    sync::Once,
};

const INSTALL_CYCLES: u128 = 1_000_000_000_000;
const CANISTERS: [&str; 2] = ["canister_test", "intent_authority"];
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
    let root_pid = Fake::principal(1);

    let env = EnvBootstrapArgs {
        prime_root_pid: Some(root_pid),
        subnet_role: Some(SubnetRole::PRIME),
        subnet_pid: Some(Fake::principal(2)),
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
                let pid = Fake::principal(u32::from(next_id));
                next_id = next_id.saturating_add(1);
                pid
            }
        } else {
            let pid = Fake::principal(u32::from(next_id));
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

fn workspace_root() -> PathBuf {
    workspace_root_for(env!("CARGO_MANIFEST_DIR"))
}
