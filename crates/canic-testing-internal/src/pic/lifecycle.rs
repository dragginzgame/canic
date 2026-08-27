use crate::canister::TEST;
use candid::{Principal, encode_args, encode_one};
use canic::{
    dto::{
        abi::v1::{CanisterInitAuthority, CanisterInitPayload},
        component_deployment::ProtectedComponentDeployment,
        component_registry::{
            ComponentDirectoryHead, ComponentDirectoryProvenance,
            ComponentRuntimeDirectoryAuthority, ComponentRuntimeDirectoryPreparationRequest,
        },
        fleet_registry::{
            FleetDirectoryProvenance, FleetDirectorySnapshot, FleetRegistryVersion,
            FleetSubnetRootDirectoryEntry, FleetSubnetRootStatus,
        },
    },
    ids::{
        ComponentBinding, ComponentInstanceId, ComponentSpecAdmission, ComponentSpecId,
        CyclesFundingBudget, FleetCoordinatorBinding, FleetRegistryAuthority,
        FleetSubnetCanisterPoolConfig, FleetSubnetRootBinding, FleetSubnetRootLimits, SubnetId,
    },
};
use canic_core::cdk::types::Cycles;
use canic_core::shared_support::fleet_admission_policy::{
    compile_fleet_admission_projection, compile_installed_fleet_admission_policy,
};
use canic_host::release_set::AppConfigSnapshot;
use ic_testkit::{
    Fake,
    artifacts::{read_wasm, test_target_dir, workspace_root_for},
    pic::{PocketIc, PocketIcBuilder},
};
use std::{
    path::{Path, PathBuf},
    sync::{Once, OnceLock},
};

use super::{
    artifacts::{
        CanicWasmBuildProfile, build_internal_test_wasm_canisters,
        build_internal_test_wasm_canisters_with_env,
    },
    canic::managed_test_init_identity,
    startup::start_pocket_ic,
};

const INSTALL_CYCLES: u128 = 1_000_000_000_000;
const CANISTERS: [&str; 3] = ["canister_test", "intent_authority", "runtime_probe"];
const LIFECYCLE_CANISTER_CONFIG_PATH: &str = "apps/test/test-configs/root-sharding.toml";
const COMBINED_LIFECYCLE_CONFIG_PATH: &str =
    "canisters/test/canic_icydb_lifecycle_probe/canic.toml";
static BUILD_ONCE: Once = Once::new();
static COMBINED_BUILD_ONCE: Once = Once::new();
const LIFECYCLE_PARTICIPANT_TRAP_ENV: (&str, &str) = ("CANIC_TEST_LIFECYCLE_PARTICIPANT_TRAP", "1");
const LIFECYCLE_PARTICIPANT_INIT_TRAP_ENV: (&str, &str) =
    ("CANIC_TEST_LIFECYCLE_PARTICIPANT_INIT_TRAP", "1");
const ICYDB_PARTICIPANT_TRAP_ENV: (&str, &str) = ("CANIC_TEST_ICYDB_PARTICIPANT_TRAP", "1");

///
/// LifecycleBoundaryFixture
///

pub struct LifecycleBoundaryFixture {
    pub pic: PocketIc,
    pub root: Principal,
    pub canic_wasm: Vec<u8>,
    pub runtime_probe_wasm: Vec<u8>,
    pub authority_wasm: Vec<u8>,
}

/// A funded empty canister and the identity-bound arguments for its Canic install.
pub struct UninstalledCanicFixture {
    pub canister_id: Principal,
    pub init_args: Vec<u8>,
    pub directory_request: ComponentRuntimeDirectoryPreparationRequest,
}

///
/// CanicIcydbLifecycleFixture
///

pub struct CanicIcydbLifecycleFixture {
    pub pic: PocketIc,
    pub root: Principal,
    pub wasm: Vec<u8>,
}

impl CanicIcydbLifecycleFixture {
    /// Install the exact managed Canic/IcyDB composition probe while it remains Prepared.
    #[must_use]
    pub fn install_composed_canister(
        &self,
    ) -> (Principal, ComponentRuntimeDirectoryPreparationRequest) {
        let canister_id = self.pic.create_canister();
        self.pic.add_cycles(canister_id, INSTALL_CYCLES);
        let payload =
            init_payload_for_config(canister_id, self.root, COMBINED_LIFECYCLE_CONFIG_PATH);
        let directory_request = directory_request(&payload);
        self.pic.install_canister(
            canister_id,
            self.wasm.clone(),
            encode_init_args(payload),
            None,
        );
        (canister_id, directory_request)
    }
}

impl LifecycleBoundaryFixture {
    /// Create one funded but uninstalled canister with its valid Canic init arguments.
    #[must_use]
    pub fn create_uninstalled_canic_canister(&self) -> UninstalledCanicFixture {
        let canister_id = self.pic.create_canister();
        self.pic.add_cycles(canister_id, INSTALL_CYCLES);
        let payload = init_payload(canister_id, self.root);
        let directory_request = directory_request(&payload);
        UninstalledCanicFixture {
            canister_id,
            init_args: encode_init_args(payload),
            directory_request,
        }
    }

    /// Install one fresh non-root Canic test canister with the standard valid init payload.
    #[must_use]
    pub fn install_canic_canister(&self) -> Principal {
        self.install_canic_canister_with_directory().0
    }

    /// Install one fresh managed canister and retain its exact activation request.
    #[must_use]
    pub fn install_canic_canister_with_directory(
        &self,
    ) -> (Principal, ComponentRuntimeDirectoryPreparationRequest) {
        self.install_canic_canister_with_directory_for_root(self.root)
    }

    /// Install one fresh managed canister for an exact Fleet Root binding.
    #[must_use]
    pub fn install_canic_canister_with_directory_for_root(
        &self,
        root: Principal,
    ) -> (Principal, ComponentRuntimeDirectoryPreparationRequest) {
        let canister_id = self.pic.create_canister();
        self.pic.add_cycles(canister_id, INSTALL_CYCLES);
        let payload = init_payload(canister_id, root);
        let directory_request = directory_request(&payload);
        let init_args = encode_init_args(payload);
        self.pic
            .install_canister(canister_id, self.canic_wasm.clone(), init_args, None);
        (canister_id, directory_request)
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
    let target_dir = test_target_dir(&workspace_root, "pic-runtime-wasm");
    build_canisters_once(&workspace_root);

    LifecycleBoundaryFixture {
        root: Fake::principal(1),
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
        pic: start_pocket_ic(PocketIcBuilder::new().with_application_subnet()),
    }
}

/// Build the exact published-IcyDB composition probe and start one fresh PocketIC.
#[must_use]
pub fn install_canic_icydb_lifecycle_fixture() -> CanicIcydbLifecycleFixture {
    let workspace_root = workspace_root();
    let target_dir = test_target_dir(&workspace_root, "pic-canic-icydb-lifecycle-wasm");
    build_combined_canister_once(&workspace_root);

    CanicIcydbLifecycleFixture {
        root: Fake::principal(1),
        wasm: read_wasm(
            &target_dir,
            "canic_icydb_lifecycle_probe",
            CanicWasmBuildProfile::Fast.target_dir_name(),
        ),
        pic: start_pocket_ic(PocketIcBuilder::new().with_application_subnet()),
    }
}

/// Encode the intentionally invalid init payload used by lifecycle boundary checks.
#[must_use]
pub fn invalid_init_args() -> Vec<u8> {
    encode_init_args(init_payload(Fake::principal(9), Fake::principal(1)))
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

/// Build the managed lifecycle fixture whose post-upgrade participant traps.
#[must_use]
pub fn lifecycle_participant_trap_wasm() -> Vec<u8> {
    static WASM: OnceLock<Vec<u8>> = OnceLock::new();
    WASM.get_or_init(|| {
        let workspace_root = workspace_root();
        let target_dir = test_target_dir(&workspace_root, "pic-lifecycle-participant-trap-wasm");
        build_internal_test_wasm_canisters_with_env(
            &workspace_root,
            &target_dir,
            &["canister_test"],
            CanicWasmBuildProfile::Fast,
            &[LIFECYCLE_PARTICIPANT_TRAP_ENV],
        );
        read_wasm(
            &target_dir,
            "canister_test",
            CanicWasmBuildProfile::Fast.target_dir_name(),
        )
    })
    .clone()
}

/// Build the managed lifecycle fixture whose init participant traps.
#[must_use]
pub fn lifecycle_participant_init_trap_wasm() -> Vec<u8> {
    static WASM: OnceLock<Vec<u8>> = OnceLock::new();
    WASM.get_or_init(|| {
        let workspace_root = workspace_root();
        let target_dir =
            test_target_dir(&workspace_root, "pic-lifecycle-participant-init-trap-wasm");
        build_internal_test_wasm_canisters_with_env(
            &workspace_root,
            &target_dir,
            &["canister_test"],
            CanicWasmBuildProfile::Fast,
            &[LIFECYCLE_PARTICIPANT_INIT_TRAP_ENV],
        );
        read_wasm(
            &target_dir,
            "canister_test",
            CanicWasmBuildProfile::Fast.target_dir_name(),
        )
    })
    .clone()
}

/// Build the combined lifecycle fixture whose IcyDB participant path traps after restoration.
#[must_use]
pub fn icydb_participant_trap_wasm() -> Vec<u8> {
    static WASM: OnceLock<Vec<u8>> = OnceLock::new();
    WASM.get_or_init(|| {
        let workspace_root = workspace_root();
        let target_dir = test_target_dir(&workspace_root, "pic-icydb-participant-trap-wasm");
        build_internal_test_wasm_canisters_with_env(
            &workspace_root,
            &target_dir,
            &["canic_icydb_lifecycle_probe"],
            CanicWasmBuildProfile::Fast,
            &[ICYDB_PARTICIPANT_TRAP_ENV],
        );
        read_wasm(
            &target_dir,
            "canic_icydb_lifecycle_probe",
            CanicWasmBuildProfile::Fast.target_dir_name(),
        )
    })
    .clone()
}

// Build the dedicated lifecycle-boundary canisters once into the shared test target dir.
fn build_canisters_once(workspace_root: &Path) {
    BUILD_ONCE.call_once(|| {
        let target_dir = test_target_dir(workspace_root, "pic-runtime-wasm");
        build_internal_test_wasm_canisters(
            workspace_root,
            &target_dir,
            &CANISTERS,
            CanicWasmBuildProfile::Fast,
        );
    });
}

// Build the combined framework lifecycle probe once into its dedicated test target dir.
fn build_combined_canister_once(workspace_root: &Path) {
    COMBINED_BUILD_ONCE.call_once(|| {
        let target_dir = test_target_dir(workspace_root, "pic-canic-icydb-lifecycle-wasm");
        build_internal_test_wasm_canisters(
            workspace_root,
            &target_dir,
            &["canic_icydb_lifecycle_probe"],
            CanicWasmBuildProfile::Fast,
        );
    });
}

// Encode the standard valid non-root init payload for the lifecycle-boundary test canister.
fn init_payload(canister_id: Principal, root_pid: Principal) -> CanisterInitPayload {
    init_payload_for_config(canister_id, root_pid, LIFECYCLE_CANISTER_CONFIG_PATH)
}

// Construct one exact managed init payload from the fixture's embedded Canic config.
fn init_payload_for_config(
    canister_id: Principal,
    root_pid: Principal,
    config_path: &str,
) -> CanisterInitPayload {
    let identity = managed_test_init_identity();
    let component_spec =
        ComponentSpecId::try_from(String::from("test")).expect("test Component Spec ID");
    let config = AppConfigSnapshot::load(&workspace_root().join(config_path))
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
            fleet: identity.fleet.clone(),
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
        funding: super::root_funding_authority(),
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 1,
            maximum_registry_bytes: 1_048_576,
            maximum_wasm_store_bytes: 1_048_576,
            maximum_group_placements: 16,
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
    let target = canic::ids::ManagedCanisterBinding::Component(binding.clone());
    let policy = compile_installed_fleet_admission_policy(
        identity.fleet,
        1,
        vec![Fake::principal(15)],
        Vec::new(),
    )
    .expect("lifecycle Fleet admission policy");
    let fleet_admission = compile_fleet_admission_projection(&policy, target)
        .expect("lifecycle Fleet admission projection");

    CanisterInitPayload {
        install_id: identity.install_id,
        release_build_id: identity.release_build_id,
        component_deployment: Box::new(ProtectedComponentDeployment::UngroupedOrdinary {
            binding: binding.clone(),
        }),
        authority: CanisterInitAuthority::Component { root, binding },
        admission: Some(fleet_admission),
    }
}

// Derive the exact root-issued Directory authority for the just-installed managed binding.
fn directory_request(payload: &CanisterInitPayload) -> ComponentRuntimeDirectoryPreparationRequest {
    let CanisterInitAuthority::Component { root, binding } = &payload.authority else {
        panic!("combined lifecycle fixture requires Component authority");
    };
    ComponentRuntimeDirectoryPreparationRequest {
        operation_id: payload.install_id,
        authority: ComponentRuntimeDirectoryAuthority {
            fleet: FleetDirectorySnapshot {
                provenance: FleetDirectoryProvenance {
                    registry: FleetRegistryVersion {
                        authority: root.authority.clone(),
                        revision: 1,
                        content_hash: [0x71; 32],
                    },
                    source_fleet_subnet_root: root.fleet_subnet_root,
                },
                fleet_subnet_roots: vec![FleetSubnetRootDirectoryEntry {
                    placement_subnet: root.placement_subnet,
                    fleet_subnet_root: root.fleet_subnet_root,
                    status: FleetSubnetRootStatus::Active,
                }],
                services: Vec::new(),
            },
            component: ComponentDirectoryHead {
                provenance: ComponentDirectoryProvenance {
                    component: binding.clone(),
                    source_fleet_subnet_root: root.fleet_subnet_root,
                    component_registry_revision: 1,
                    component_registry_content_hash: [0x72; 32],
                    synchronized_at_ns: 1,
                },
                descendant_count: 0,
            },
            component_group: None,
        },
        direct_children: Vec::new(),
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
pub(super) use tests::{governed_fast_cases, governed_pocketic_cases};

#[cfg(test)]
mod tests {
    use super::*;
    use canic::{
        Error,
        dto::{
            fleet_admission::{
                FleetAdmissionActivateTargetRequest, FleetAdmissionOpenTargetRequest,
                FleetAdmissionPrepareTargetRequest, FleetAdmissionProjectionPhase,
                FleetAdmissionProjectionStatusResponse, FleetAdmissionTargetReceipt,
                FleetAdmissionTargetTransitionPhase,
            },
            page::PageRequest,
            role::OperationReceipt,
        },
        protocol::{CANIC_COMMAND, CANIC_STATUS},
    };
    use canic_core::bootstrap::{compiled::ComponentTopology, parse_config_model};
    use ic_testkit::pic::{CandidCallExt, CanisterInstallExt};
    use std::time::Duration;

    #[derive(candid::CandidType)]
    enum ManagedCommand {
        ActivateFleetAdmission(FleetAdmissionActivateTargetRequest),
        ConfigureRuntime(Box<ComponentRuntimeDirectoryPreparationRequest>),
        OpenFleetAdmission(FleetAdmissionOpenTargetRequest),
        PrepareFleetAdmission(Box<FleetAdmissionPrepareTargetRequest>),
    }

    #[derive(candid::CandidType, Debug, candid::Deserialize, Eq, PartialEq)]
    enum ManagedCommandResponse {
        ActivateFleetAdmission(FleetAdmissionTargetReceipt),
        OpenFleetAdmission(FleetAdmissionTargetReceipt),
        OperationAccepted(OperationReceipt),
        PrepareFleetAdmission(FleetAdmissionTargetReceipt),
    }

    #[derive(candid::CandidType)]
    enum ManagedStatusRequest {
        Admission(PageRequest),
    }

    #[derive(candid::CandidType, candid::Deserialize)]
    enum ManagedStatusResponse {
        Admission(FleetAdmissionProjectionStatusResponse),
    }

    #[derive(candid::CandidType, Debug, candid::Deserialize, Eq, PartialEq)]
    enum ProbeEvidence {
        Missing,
        Observed,
    }

    #[derive(candid::CandidType, Debug, candid::Deserialize, Eq, PartialEq)]
    struct ComposedFrameworkAdmissionReceipt {
        caller: Principal,
        workflow_runs: u32,
        icydb_request_session: ProbeEvidence,
    }

    const LIFECYCLE_CANISTER_CONFIG: &str =
        include_str!("../../../../apps/test/test-configs/root-sharding.toml");

    #[test]
    fn init_payload_component_spec_matches_embedded_canister_config() {
        let payload = init_payload(Fake::principal(3), Fake::principal(1));
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

    #[test]
    fn managed_projection_fences_then_opens_and_restores() {
        let fixture = install_lifecycle_boundary_fixture();
        let (canister, directory) = fixture.install_canic_canister_with_directory();
        let second_root = Fake::principal(2);
        let (second_canister, second_directory) =
            fixture.install_canic_canister_with_directory_for_root(second_root);
        let admitted = Fake::principal(15);

        let prepared = admission_status(&fixture.pic, canister, fixture.root);
        let second_prepared = admission_status(&fixture.pic, second_canister, second_root);
        assert_eq!(prepared.phase, FleetAdmissionProjectionPhase::Fenced);
        assert_eq!(second_prepared.phase, FleetAdmissionProjectionPhase::Fenced);
        assert_eq!(prepared.principals.entries, vec![admitted]);
        assert_eq!(second_prepared.principals.entries, vec![admitted]);
        assert_eq!(prepared.policy_digest, second_prepared.policy_digest);
        assert_ne!(prepared.target, second_prepared.target);
        assert_ne!(
            prepared.projection_digest,
            second_prepared.projection_digest
        );
        let denied: Result<Result<(), Error>, _> =
            fixture
                .pic
                .query_candid_as(canister, admitted, "test_fleet_admission_probe", ());
        assert!(denied.is_err());

        activate_projection(&fixture.pic, canister, fixture.root, &directory);
        activate_projection(
            &fixture.pic,
            second_canister,
            second_root,
            &second_directory,
        );

        let open = admission_status(&fixture.pic, canister, fixture.root);
        let second_open = admission_status(&fixture.pic, second_canister, second_root);
        assert_eq!(open.phase, FleetAdmissionProjectionPhase::Open);
        assert_eq!(second_open.phase, FleetAdmissionProjectionPhase::Open);
        assert_eq!(admission_probe(&fixture.pic, canister, admitted), Ok(()));
        assert_eq!(
            admission_probe(&fixture.pic, second_canister, admitted),
            Ok(())
        );
        assert!(admission_probe(&fixture.pic, canister, Fake::principal(16)).is_err());

        fixture
            .pic
            .wait_out_install_code_rate_limit(Duration::from_mins(5));
        fixture
            .pic
            .upgrade_canister(canister, fixture.canic_wasm.clone(), upgrade_args(), None)
            .expect("same-release projection restore");

        let restored = admission_status(&fixture.pic, canister, fixture.root);
        assert_eq!(restored.phase, FleetAdmissionProjectionPhase::Open);
        assert_eq!(restored.generation, open.generation);
        assert_eq!(restored.policy_digest, open.policy_digest);
        assert_eq!(restored.projection_digest, open.projection_digest);
        assert_eq!(admission_probe(&fixture.pic, canister, admitted), Ok(()));
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one direct-ingress journey proves public, fenced, denied and admitted framework paths"
    )]
    fn composed_framework_guard_matches_canic_endpoint_on_direct_ingress() {
        let fixture = install_canic_icydb_lifecycle_fixture();
        let (canister, directory) = fixture.install_composed_canister();
        let admitted = Fake::principal(15);
        let unlisted = Fake::principal(16);

        let prepared_call: Result<Result<ComposedFrameworkAdmissionReceipt, Error>, _> = fixture
            .pic
            .update_candid_as(canister, admitted, "composed_framework_admission_probe", ());
        assert!(prepared_call.is_err());
        activate_projection(&fixture.pic, canister, fixture.root, &directory);
        for _ in 0..3 {
            fixture.pic.advance_time(Duration::from_secs(1));
            fixture.pic.tick();
        }

        let public_caller: Result<Principal, Error> = fixture.pic.query_candid_as_or_panic(
            canister,
            unlisted,
            "composed_framework_public_probe",
            (),
        );
        assert_eq!(public_caller, Ok(unlisted));

        let composed: Result<ComposedFrameworkAdmissionReceipt, Error> =
            fixture.pic.update_candid_as_or_panic(
                canister,
                admitted,
                "composed_framework_admission_probe",
                (),
            );
        assert_eq!(
            composed.expect("admitted composed-framework caller"),
            ComposedFrameworkAdmissionReceipt {
                caller: admitted,
                workflow_runs: 1,
                icydb_request_session: ProbeEvidence::Observed,
            }
        );
        assert_eq!(composed_framework_workflow_runs(&fixture.pic, canister), 1);

        let canic: Result<Principal, Error> = fixture.pic.query_candid_as_or_panic(
            canister,
            admitted,
            "canic_fleet_admission_parity_probe",
            (),
        );
        assert_eq!(canic.expect("admitted Canic endpoint caller"), admitted);

        let application_owned: Result<Principal, Error> = fixture.pic.update_candid_as_or_panic(
            canister,
            admitted,
            "composed_framework_owned_probe",
            (),
        );
        assert_eq!(
            application_owned
                .expect_err("Fleet admission must not imply application ownership")
                .code(),
            canic::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
        );

        let denied: Result<ComposedFrameworkAdmissionReceipt, Error> =
            fixture.pic.update_candid_as_or_panic(
                canister,
                unlisted,
                "composed_framework_admission_probe",
                (),
            );
        assert!(denied.is_err());
        assert_eq!(composed_framework_workflow_runs(&fixture.pic, canister), 1);

        let canic_denied: Result<Result<Principal, Error>, _> = fixture.pic.query_candid_as(
            canister,
            unlisted,
            "canic_fleet_admission_parity_probe",
            (),
        );
        assert!(matches!(canic_denied, Ok(Err(_))));

        let initial = admission_status(&fixture.pic, canister, fixture.root);
        let successor_policy = compile_installed_fleet_admission_policy(
            initial.authority.fleet.clone(),
            initial.generation + 1,
            vec![admitted],
            Vec::new(),
        )
        .expect("successor admission policy");
        let successor =
            compile_fleet_admission_projection(&successor_policy, initial.target.clone())
                .expect("successor target projection");
        transition_target(
            &fixture.pic,
            canister,
            fixture.root,
            ManagedCommand::PrepareFleetAdmission(Box::new(FleetAdmissionPrepareTargetRequest {
                operation_id: [0x90; 32],
                expected_generation: initial.generation,
                expected_policy_digest: initial.policy_digest,
                successor,
            })),
        );
        let fenced_public: Result<Principal, Error> = fixture.pic.query_candid_as_or_panic(
            canister,
            unlisted,
            "composed_framework_public_probe",
            (),
        );
        assert_eq!(fenced_public, Ok(unlisted));
        let fenced: Result<ComposedFrameworkAdmissionReceipt, Error> =
            fixture.pic.update_candid_as_or_panic(
                canister,
                admitted,
                "composed_framework_admission_probe",
                (),
            );
        assert!(fenced.is_err());
        assert_eq!(composed_framework_workflow_runs(&fixture.pic, canister), 1);
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "target transition replay exercises every durable phase"
    )]
    fn managed_admission_target_transition_replays_and_recovers_forward() {
        let fixture = install_canic_icydb_lifecycle_fixture();
        let (canister, directory) = fixture.install_composed_canister();
        activate_projection(&fixture.pic, canister, fixture.root, &directory);

        let initial = admission_status(&fixture.pic, canister, fixture.root);
        let successor_policy = compile_installed_fleet_admission_policy(
            initial.authority.fleet.clone(),
            initial.generation + 1,
            vec![Fake::principal(15), Fake::principal(16)],
            Vec::new(),
        )
        .expect("successor admission policy");
        let successor =
            compile_fleet_admission_projection(&successor_policy, initial.target.clone())
                .expect("successor target projection");
        let operation_id = [0x91; 32];

        let assert_unchanged = || {
            let status = admission_status(&fixture.pic, canister, fixture.root);
            assert_eq!(status.phase, FleetAdmissionProjectionPhase::Open);
            assert_eq!(status.generation, initial.generation);
            assert_eq!(status.policy_digest, initial.policy_digest);
            assert_eq!(status.projection_digest, initial.projection_digest);
            assert!(status.prepared.is_none());
        };
        let valid_prepare = || FleetAdmissionPrepareTargetRequest {
            operation_id,
            expected_generation: initial.generation,
            expected_policy_digest: initial.policy_digest,
            successor: successor.clone(),
        };

        assert!(
            try_transition_target(
                &fixture.pic,
                canister,
                Fake::principal(99),
                ManagedCommand::PrepareFleetAdmission(Box::new(valid_prepare())),
            )
            .is_err()
        );
        assert_unchanged();

        let mut wrong_coordinator = valid_prepare();
        wrong_coordinator.successor.authority.coordinator = Fake::principal(98);
        assert!(
            try_transition_target(
                &fixture.pic,
                canister,
                fixture.root,
                ManagedCommand::PrepareFleetAdmission(Box::new(wrong_coordinator)),
            )
            .is_err()
        );
        assert_unchanged();

        let mut wrong_fleet = valid_prepare();
        wrong_fleet.successor.authority.fleet.app = canic_core::ids::AppId::from("foreign");
        assert!(
            try_transition_target(
                &fixture.pic,
                canister,
                fixture.root,
                ManagedCommand::PrepareFleetAdmission(Box::new(wrong_fleet)),
            )
            .is_err()
        );
        assert_unchanged();

        let mut wrong_target = valid_prepare();
        match &mut wrong_target.successor.target {
            canic::ids::ManagedCanisterBinding::Component(binding) => {
                binding.canister_id = Fake::principal(97);
            }
            canic::ids::ManagedCanisterBinding::ComponentChild(binding) => {
                binding.canister_id = Fake::principal(97);
            }
        }
        assert!(
            try_transition_target(
                &fixture.pic,
                canister,
                fixture.root,
                ManagedCommand::PrepareFleetAdmission(Box::new(wrong_target)),
            )
            .is_err()
        );
        assert_unchanged();

        let mut wrong_generation = valid_prepare();
        wrong_generation.expected_generation = initial.generation + 1;
        assert!(
            try_transition_target(
                &fixture.pic,
                canister,
                fixture.root,
                ManagedCommand::PrepareFleetAdmission(Box::new(wrong_generation)),
            )
            .is_err()
        );
        assert_unchanged();

        let mut wrong_digest = valid_prepare();
        wrong_digest.expected_policy_digest = [0x92; 32];
        assert!(
            try_transition_target(
                &fixture.pic,
                canister,
                fixture.root,
                ManagedCommand::PrepareFleetAdmission(Box::new(wrong_digest)),
            )
            .is_err()
        );
        assert_unchanged();

        let prepare = valid_prepare();
        let prepared = transition_target(
            &fixture.pic,
            canister,
            fixture.root,
            ManagedCommand::PrepareFleetAdmission(Box::new(prepare.clone())),
        );
        let ManagedCommandResponse::PrepareFleetAdmission(prepare_receipt) = prepared else {
            panic!("prepare target receipt");
        };
        assert_eq!(
            prepare_receipt.phase,
            FleetAdmissionTargetTransitionPhase::Prepare
        );
        fixture
            .pic
            .stop_canister(canister, None)
            .expect("stop target after retained prepare receipt");
        fixture
            .pic
            .start_canister(canister, None)
            .expect("restart target after retained prepare receipt");
        assert_eq!(
            transition_target(
                &fixture.pic,
                canister,
                fixture.root,
                ManagedCommand::PrepareFleetAdmission(Box::new(prepare)),
            ),
            ManagedCommandResponse::PrepareFleetAdmission(prepare_receipt)
        );
        let fenced = admission_status(&fixture.pic, canister, fixture.root);
        assert_eq!(fenced.phase, FleetAdmissionProjectionPhase::Fenced);
        assert!(fenced.prepared.is_some());
        assert!(combined_admission_probe(&fixture.pic, canister, Fake::principal(15)).is_err());

        let activate = FleetAdmissionActivateTargetRequest {
            operation_id,
            expected_generation: initial.generation,
            expected_policy_digest: initial.policy_digest,
            successor_generation: successor.generation,
            successor_policy_digest: successor.policy_digest,
            successor_projection_digest: successor.projection_digest,
        };
        let activated = transition_target(
            &fixture.pic,
            canister,
            fixture.root,
            ManagedCommand::ActivateFleetAdmission(activate.clone()),
        );
        let ManagedCommandResponse::ActivateFleetAdmission(activate_receipt) = activated else {
            panic!("activate target receipt");
        };
        assert_eq!(
            activate_receipt.phase,
            FleetAdmissionTargetTransitionPhase::Activate
        );
        fixture
            .pic
            .stop_canister(canister, None)
            .expect("stop target after retained activate receipt");
        fixture
            .pic
            .start_canister(canister, None)
            .expect("restart target after retained activate receipt");
        assert_eq!(
            transition_target(
                &fixture.pic,
                canister,
                fixture.root,
                ManagedCommand::ActivateFleetAdmission(activate),
            ),
            ManagedCommandResponse::ActivateFleetAdmission(activate_receipt)
        );
        let activated_status = admission_status(&fixture.pic, canister, fixture.root);
        assert_eq!(
            activated_status.phase,
            FleetAdmissionProjectionPhase::Fenced
        );
        assert_eq!(activated_status.generation, successor.generation);
        assert!(activated_status.prepared.is_none());

        let open = FleetAdmissionOpenTargetRequest {
            operation_id,
            generation: successor.generation,
            policy_digest: successor.policy_digest,
            projection_digest: successor.projection_digest,
        };
        let opened = transition_target(
            &fixture.pic,
            canister,
            fixture.root,
            ManagedCommand::OpenFleetAdmission(open.clone()),
        );
        let ManagedCommandResponse::OpenFleetAdmission(open_receipt) = opened else {
            panic!("open target receipt");
        };
        assert_eq!(
            open_receipt.phase,
            FleetAdmissionTargetTransitionPhase::Open
        );
        assert_eq!(
            transition_target(
                &fixture.pic,
                canister,
                fixture.root,
                ManagedCommand::OpenFleetAdmission(open.clone()),
            ),
            ManagedCommandResponse::OpenFleetAdmission(open_receipt.clone())
        );
        assert_eq!(
            combined_admission_probe(&fixture.pic, canister, Fake::principal(16)),
            Ok(Fake::principal(16))
        );

        fixture
            .pic
            .wait_out_install_code_rate_limit(Duration::from_mins(5));
        fixture
            .pic
            .upgrade_canister(canister, fixture.wasm.clone(), upgrade_args(), None)
            .expect("same-release target transition restore");
        assert_eq!(
            transition_target(
                &fixture.pic,
                canister,
                fixture.root,
                ManagedCommand::OpenFleetAdmission(open),
            ),
            ManagedCommandResponse::OpenFleetAdmission(open_receipt)
        );
    }

    #[test]
    fn published_managed_app_support_drives_composed_lifecycle() {
        let workspace_root = workspace_root();
        let target_dir = test_target_dir(&workspace_root, "pic-canic-icydb-lifecycle-wasm");
        build_combined_canister_once(&workspace_root);
        let wasm = read_wasm(
            &target_dir,
            "canic_icydb_lifecycle_probe",
            CanicWasmBuildProfile::Fast.target_dir_name(),
        );
        let admitted = Fake::principal(15);
        let input = canic::testing::ManagedAppQualificationInput::new(
            include_str!("../../../../canisters/test/canic_icydb_lifecycle_probe/canic.toml"),
            "test",
            "test",
            super::super::artifacts::INTERNAL_TEST_RELEASE_BUILD_ID.1,
            vec![admitted],
            wasm,
        );
        let fixture = canic::testing::install_managed_app(input)
            .expect("install through published managed-App support");

        assert_eq!(
            fixture
                .admission_status()
                .expect("prepared admission status")
                .phase,
            FleetAdmissionProjectionPhase::Fenced
        );
        let prepared: Result<Result<ComposedFrameworkAdmissionReceipt, Error>, _> =
            fixture.pic().update_candid_as(
                fixture.app(),
                admitted,
                "composed_framework_admission_probe",
                (),
            );
        assert!(prepared.is_err());

        fixture
            .configure_and_wait_until_active(30)
            .expect("activate through published managed-App support");
        for _ in 0..3 {
            fixture.pic().advance_time(Duration::from_secs(1));
            fixture.pic().tick();
        }
        assert_eq!(
            fixture
                .admission_status()
                .expect("open admission status")
                .phase,
            FleetAdmissionProjectionPhase::Open
        );
        let admitted_result: Result<ComposedFrameworkAdmissionReceipt, Error> =
            fixture.pic().update_candid_as_or_panic(
                fixture.app(),
                admitted,
                "composed_framework_admission_probe",
                (),
            );
        assert!(admitted_result.is_ok());

        fixture
            .upgrade_same_release(Duration::from_mins(5))
            .expect("same-release managed-App upgrade");
        assert_eq!(
            fixture
                .admission_status()
                .expect("restored admission status")
                .phase,
            FleetAdmissionProjectionPhase::Open
        );
        fixture
            .prepare_admission_successor([0x94; 32], vec![admitted, Fake::principal(16)])
            .expect("fence successor through published managed-App support");
        assert_eq!(
            fixture
                .admission_status()
                .expect("fenced successor status")
                .phase,
            FleetAdmissionProjectionPhase::Fenced
        );
    }

    fn activate_projection(
        pic: &PocketIc,
        canister: Principal,
        root: Principal,
        directory: &ComponentRuntimeDirectoryPreparationRequest,
    ) {
        let response: Result<ManagedCommandResponse, Error> = pic.update_candid_as_or_panic(
            canister,
            root,
            CANIC_COMMAND,
            (ManagedCommand::ConfigureRuntime(Box::new(
                directory.clone(),
            )),),
        );
        let ManagedCommandResponse::OperationAccepted(receipt) =
            response.expect("Root activates the exact prepared runtime")
        else {
            panic!("runtime activation receipt");
        };
        assert_eq!(receipt.operation_id, directory.operation_id);
    }

    fn admission_status(
        pic: &PocketIc,
        canister: Principal,
        root: Principal,
    ) -> FleetAdmissionProjectionStatusResponse {
        let response: Result<ManagedStatusResponse, Error> = pic.query_candid_as_or_panic(
            canister,
            root,
            CANIC_STATUS,
            (ManagedStatusRequest::Admission(PageRequest {
                offset: 0,
                limit: u64::MAX,
            }),),
        );
        let ManagedStatusResponse::Admission(status) =
            response.expect("protected admission status");
        status
    }

    fn admission_probe(
        pic: &PocketIc,
        canister: Principal,
        caller: Principal,
    ) -> Result<(), Error> {
        pic.query_candid_as_or_panic(canister, caller, "test_fleet_admission_probe", ())
    }

    fn combined_admission_probe(
        pic: &PocketIc,
        canister: Principal,
        caller: Principal,
    ) -> Result<Principal, Error> {
        pic.query_candid_as_or_panic(canister, caller, "canic_fleet_admission_parity_probe", ())
    }

    fn transition_target(
        pic: &PocketIc,
        canister: Principal,
        root: Principal,
        command: ManagedCommand,
    ) -> ManagedCommandResponse {
        let response: Result<ManagedCommandResponse, Error> =
            pic.update_candid_as_or_panic(canister, root, CANIC_COMMAND, (command,));
        response.expect("accepted target admission transition")
    }

    fn try_transition_target(
        pic: &PocketIc,
        canister: Principal,
        root: Principal,
        command: ManagedCommand,
    ) -> Result<ManagedCommandResponse, Error> {
        pic.update_candid_as_or_panic(canister, root, CANIC_COMMAND, (command,))
    }

    fn composed_framework_workflow_runs(pic: &PocketIc, canister: Principal) -> u32 {
        let result: Result<u32, Error> =
            pic.query_candid_or_panic(canister, "composed_framework_workflow_runs", ());
        result.expect("public workflow-run observation")
    }

    pub fn governed_fast_cases() -> Vec<crate::pic::GovernedTestCase> {
        vec![(
            "lifecycle embedded Component Spec",
            init_payload_component_spec_matches_embedded_canister_config,
        )]
    }

    pub fn governed_pocketic_cases() -> Vec<crate::pic::GovernedTestCase> {
        vec![
            (
                "composed-framework direct ingress",
                composed_framework_guard_matches_canic_endpoint_on_direct_ingress,
            ),
            (
                "managed admission target transition",
                managed_admission_target_transition_replays_and_recovers_forward,
            ),
            (
                "published managed-App support",
                published_managed_app_support_drives_composed_lifecycle,
            ),
        ]
    }
}
