use crate::canister::{APP, SCALE_HUB, TEST, USER_HUB};
use candid::{Principal, encode_args, encode_one};
use canic::{
    dto::{
        abi::v1::CanisterInitPayload,
        env::EnvBootstrapArgs,
        topology::{
            DirectoryEntryInput, DirectoryProvenance, FleetDirectoryInput, SubnetDirectoryInput,
        },
    },
    ids::{CanisterRole, ComponentSpecId},
};
use ic_testkit::{
    Fake,
    artifacts::{read_wasm, test_target_dir, workspace_root_for},
    pic::{Pic, PicSerialGuard, acquire_pic_serial_guard, pic},
};
use std::{
    path::{Path, PathBuf},
    sync::Once,
};

use super::{
    artifacts::{CanicWasmBuildProfile, build_internal_test_wasm_canisters},
    canic::managed_test_init_identity,
};

const INSTALL_CYCLES: u128 = 1_000_000_000_000;
const CANISTERS: [&str; 3] = ["canister_test", "intent_authority", "runtime_probe"];
static BUILD_ONCE: Once = Once::new();

///
/// LifecycleBoundaryFixture
///

pub struct LifecycleBoundaryFixture {
    pub pic: Pic,
    pub canic_wasm: Vec<u8>,
    pub runtime_probe_wasm: Vec<u8>,
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

    /// Install the standalone-local runtime probe used by timer behavior tests.
    ///
    /// # Panics
    ///
    /// Panics if the fixed standalone-local init argument cannot be encoded.
    #[must_use]
    pub fn install_runtime_probe_canister(&self) -> Principal {
        let canister_id = self.pic.create_canister();
        self.pic.add_cycles(canister_id, INSTALL_CYCLES);
        self.pic.install_canister(
            canister_id,
            self.runtime_probe_wasm.clone(),
            encode_one(None::<Vec<u8>>).expect("encode standalone-local init"),
            None,
        );
        canister_id
    }

    /// Install one fresh non-Canic authority canister for negative upgrade cases.
    ///
    /// # Panics
    ///
    /// Panics if the authority init argument cannot be encoded.
    #[must_use]
    pub fn install_authority_canister(&self) -> Principal {
        let canister_id = self.pic.create_canister();
        self.pic.add_cycles(canister_id, INSTALL_CYCLES);
        self.pic.install_canister(
            canister_id,
            self.authority_wasm.clone(),
            encode_one(()).expect("encode authority init"),
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
        canic_wasm: read_wasm(
            &target_dir,
            "canister_test",
            CanicWasmBuildProfile::Fast.target_dir_name(),
        ),
        runtime_probe_wasm: read_wasm(
            &target_dir,
            "runtime_probe",
            CanicWasmBuildProfile::Fast.target_dir_name(),
        ),
        authority_wasm: read_wasm(
            &target_dir,
            "intent_authority",
            CanicWasmBuildProfile::Fast.target_dir_name(),
        ),
        _serial_guard: acquire_pic_serial_guard(),
        pic: pic(),
    }
}

/// Encode the intentionally invalid init payload used by lifecycle boundary checks.
#[must_use]
pub fn invalid_init_args() -> Vec<u8> {
    let identity = managed_test_init_identity();
    let provenance = DirectoryProvenance {
        fleet: identity.fleet.clone(),
        source_root: Fake::principal(1),
    };
    let payload = CanisterInitPayload {
        fleet: identity.fleet,
        install_id: identity.install_id,
        release_build_id: identity.release_build_id,
        env: EnvBootstrapArgs {
            fleet_root_pid: None,
            component_spec: None,
            subnet_pid: None,
            root_pid: None,
            canister_role: None,
            parent_pid: None,
        },
        fleet_directory: FleetDirectoryInput {
            provenance: provenance.clone(),
            entries: Vec::new(),
        },
        subnet_directory: SubnetDirectoryInput {
            provenance,
            entries: Vec::new(),
        },
    };

    encode_init_args(payload)
}

/// Encode the empty tuple argument used for no-payload upgrades.
///
/// # Panics
///
/// Panics if the empty tuple upgrade argument cannot be encoded.
#[must_use]
pub fn upgrade_args() -> Vec<u8> {
    encode_one(()).expect("encode upgrade")
}

// Build the dedicated lifecycle-boundary canisters once into the shared test target dir.
fn build_canisters_once(workspace_root: &Path) {
    BUILD_ONCE.call_once(|| {
        let target_dir = test_target_dir(workspace_root, "pic-wasm");
        build_internal_test_wasm_canisters(
            workspace_root,
            &target_dir,
            &CANISTERS,
            CanicWasmBuildProfile::Fast,
        );
    });
}

// Encode the standard valid non-root init payload for the lifecycle-boundary test canister.
fn init_payload(canister_id: Principal) -> CanisterInitPayload {
    let root_pid = Fake::principal(1);
    let identity = managed_test_init_identity();
    let provenance = DirectoryProvenance {
        fleet: identity.fleet.clone(),
        source_root: root_pid,
    };
    let fleet_directory = fleet_directory_input(provenance.clone());
    let subnet_directory = subnet_directory_input(canister_id, provenance);

    let env = EnvBootstrapArgs {
        fleet_root_pid: Some(root_pid),
        component_spec: Some(
            ComponentSpecId::try_from(String::from("test")).expect("test Component Spec ID"),
        ),
        subnet_pid: Some(Fake::principal(2)),
        root_pid: Some(root_pid),
        canister_role: Some(TEST),
        parent_pid: Some(root_pid),
    };

    CanisterInitPayload {
        fleet: identity.fleet,
        install_id: identity.install_id,
        release_build_id: identity.release_build_id,
        env,
        fleet_directory,
        subnet_directory,
    }
}

// Encode one init payload through the standard tuple boundary expected by Canic canisters.
fn encode_init_args(payload: CanisterInitPayload) -> Vec<u8> {
    encode_args::<(CanisterInitPayload, Option<Vec<u8>>)>((payload, None))
        .expect("encode init args")
}

// Build the minimal Fleet Directory input used by lifecycle-boundary installs.
fn fleet_directory_input(provenance: DirectoryProvenance) -> FleetDirectoryInput {
    let roles = [USER_HUB, SCALE_HUB];
    FleetDirectoryInput {
        provenance,
        entries: directory_entries(&roles, None, 10),
    }
}

// Build the minimal Subnet Directory input used by lifecycle-boundary installs.
fn subnet_directory_input(
    canister_id: Principal,
    provenance: DirectoryProvenance,
) -> SubnetDirectoryInput {
    let roles = [APP, USER_HUB, SCALE_HUB, TEST];
    let override_role = Some((TEST, canister_id));
    SubnetDirectoryInput {
        provenance,
        entries: directory_entries(&roles, override_role, 20),
    }
}

// Build deterministic Directory entries with one optional explicit role override.
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::bootstrap::parse_config_model;

    const LIFECYCLE_CANISTER_CONFIG: &str =
        include_str!("../../../../apps/test/test-configs/root-sharding.toml");

    #[test]
    fn init_payload_component_spec_matches_embedded_canister_config() {
        let payload = init_payload(Fake::principal(3));
        let component_spec = payload
            .env
            .component_spec
            .as_ref()
            .expect("managed lifecycle Component Spec");
        let role = payload
            .env
            .canister_role
            .as_ref()
            .expect("managed lifecycle role");
        let config =
            parse_config_model(LIFECYCLE_CANISTER_CONFIG).expect("lifecycle canister config");
        let configured_spec = config
            .get_component_spec(component_spec)
            .expect("lifecycle Component Spec must be declared");

        assert_eq!(&configured_spec.component_role, role);
    }
}
