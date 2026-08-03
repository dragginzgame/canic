use crate::canister::TEST;
use candid::{Principal, encode_args, encode_one};
use canic::{
    dto::abi::v1::{CanisterInitAuthority, CanisterInitPayload},
    ids::{
        ComponentBinding, ComponentInstanceId, ComponentSpecAdmission, ComponentSpecId,
        CyclesFundingBudget, FleetCoordinatorBinding, FleetRegistryAuthority,
        FleetSubnetCanisterPoolConfig, FleetSubnetRootBinding, FleetSubnetRootLimits, SubnetId,
    },
};
use canic_core::cdk::types::Cycles;
use canic_host::release_set::AppConfigSnapshot;
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
const LIFECYCLE_CANISTER_CONFIG_PATH: &str = "apps/test/test-configs/root-sharding.toml";
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
    encode_init_args(init_payload(Fake::principal(9)))
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
    let component_spec =
        ComponentSpecId::try_from(String::from("test")).expect("test Component Spec ID");
    let config = AppConfigSnapshot::load(&workspace_root().join(LIFECYCLE_CANISTER_CONFIG_PATH))
        .expect("load lifecycle fixture config");
    let spec = config
        .component_topology()
        .get(&component_spec)
        .expect("test Component Spec");
    let admission = ComponentSpecAdmission {
        component_spec: component_spec.clone(),
        spec_hash: spec.spec_hash,
        maximum_root_instances: 1,
    };
    let admissions = vec![admission];
    let component_topology_digest = config
        .component_topology()
        .project_for_admissions(&admissions)
        .and_then(|topology| topology.digest())
        .expect("lifecycle Component topology projection");
    let placement_subnet = SubnetId::from_principal(Fake::principal(2));
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: identity.fleet,
            coordinator_subnet: SubnetId::from_principal(Fake::principal(3)),
            coordinator: Fake::principal(4),
        },
        epoch: 1,
    };
    let root = FleetSubnetRootBinding {
        authority: authority.clone(),
        placement_subnet,
        fleet_subnet_root: root_pid,
        component_admissions: admissions,
        component_topology_digest,
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 1,
            maximum_managed_canisters: 2,
            maximum_registry_bytes: 1_048_576,
            maximum_wasm_store_bytes: 1_048_576,
            canister_pool: FleetSubnetCanisterPoolConfig {
                minimum_size: 1,
                maximum_size: 1,
                canister_cycles: Cycles::new(1),
            },
            cycles_funding: CyclesFundingBudget {
                window_secs: 3_600,
                maximum_cycles: Cycles::new(1_000_000_000_000),
            },
        },
    };
    let binding = ComponentBinding {
        authority,
        component: ComponentInstanceId::from_generated_bytes([5; 32]),
        component_spec,
        spec_hash: spec.spec_hash,
        role: TEST,
        placement_subnet,
        fleet_subnet_root: root_pid,
        canister_id,
    };

    CanisterInitPayload {
        install_id: identity.install_id,
        release_build_id: identity.release_build_id,
        authority: CanisterInitAuthority::Component { root, binding },
    }
}

// Encode one init payload through the standard tuple boundary expected by Canic canisters.
fn encode_init_args(payload: CanisterInitPayload) -> Vec<u8> {
    encode_args::<(CanisterInitPayload, Option<Vec<u8>>)>((payload, None))
        .expect("encode init args")
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
    use canic_core::bootstrap::{compiled::ComponentTopology, parse_config_model};

    const LIFECYCLE_CANISTER_CONFIG: &str =
        include_str!("../../../../apps/test/test-configs/root-sharding.toml");

    #[test]
    fn init_payload_component_spec_matches_embedded_canister_config() {
        let payload = init_payload(Fake::principal(3));
        let CanisterInitAuthority::Component { root, binding } = payload.authority else {
            panic!("managed lifecycle Component authority");
        };
        let config =
            parse_config_model(LIFECYCLE_CANISTER_CONFIG).expect("lifecycle canister config");
        let topology =
            ComponentTopology::compile(&config).expect("compile lifecycle Component Topology");
        let configured_spec = topology
            .get(&binding.component_spec)
            .expect("lifecycle Component Spec must be declared");

        assert_eq!(configured_spec.component_role, binding.role);
        assert_eq!(configured_spec.spec_hash, binding.spec_hash);
        let admission = root
            .component_admissions
            .iter()
            .find(|admission| admission.component_spec == binding.component_spec)
            .expect("lifecycle Component Spec admission");
        assert_eq!(admission.spec_hash, configured_spec.spec_hash);
        assert_eq!(
            root.component_topology_digest,
            topology
                .project_for_admissions(&root.component_admissions)
                .and_then(|projection| projection.digest())
                .expect("lifecycle Component topology projection")
        );
    }
}
