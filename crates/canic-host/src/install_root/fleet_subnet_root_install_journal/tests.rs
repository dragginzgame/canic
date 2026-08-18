//! Module: install_root::fleet_subnet_root_install_journal::tests
//!
//! Responsibility: prove independent root effect ordering, exact retry, and authority checks.
//! Does not own: command execution or live Canister verification.
//! Boundary: fixtures contain one valid planned root and its exact topology/artifact authority.

use crate::{
    fleet_install_plan::{
        FleetInstallPlan, PersistedFleetInstallPlan, PlannedCanisterCreationFunding,
        PlannedFleetCoordinator, PlannedFleetSubnetRoot,
    },
    install_root::fleet_subnet_root_install_journal::{
        FleetSubnetRootInstallPhase, JOURNAL_SCHEMA_VERSION, PlanFleetSubnetRootInstallRequest,
        begin_component_registry_preparation, begin_registry_join,
        begin_registry_mirror_activation, begin_registry_sync, begin_root_creation,
        begin_root_install, begin_store_adoption, begin_store_bootstrap, begin_store_staging,
        begin_wasm_store_creation, begin_wasm_store_install, expected_root_authority,
        expected_wasm_store_authority, plan_fleet_subnet_root_install,
        record_component_registry_preparation_verified, record_component_registry_prepared,
        record_infrastructure_verified, record_registry_join_verified, record_registry_joined,
        record_registry_mirror_activated, record_registry_mirror_activation_verified,
        record_registry_sync_verified, record_registry_synchronized, record_root_created,
        record_root_installed, record_store_adopted, record_store_bootstrapped,
        record_store_staged, record_store_verified, record_wasm_store_created,
        record_wasm_store_installed,
    },
    release_set::{
        CanicInfrastructureArtifactEntry, CanicInfrastructureArtifactManifest,
        CanicInfrastructureRole, PersistedCanicInfrastructureArtifactManifest,
    },
    test_support::temp_dir,
};
use std::path::Path;

use candid::Principal;
use canic_core::{
    bootstrap::compiled::{ComponentLimits, ComponentSpec, ComponentTopology},
    cdk::types::Cycles,
    control_plane_support::ops::fleet_registry::FleetRegistryOps,
    dto::{
        component_registry::{
            RootComponentRegistryPreparationRequest, RootComponentRegistryStatusResponse,
        },
        fleet_registry::{
            FleetDirectoryProvenance, FleetDirectorySnapshot, FleetSubnetRootDirectoryEntry,
            FleetSubnetRootJoinResponse, FleetSubnetRootRegistryMirrorActivationRequest,
            FleetSubnetRootRegistryMirrorActivationResponse, FleetSubnetRootRegistrySyncRequest,
            FleetSubnetRootRegistrySyncResponse, FleetSubnetRootSnapshotAcknowledgement,
        },
        fleet_subnet_root::FleetSubnetWasmStoreAdoptionResponse,
        root_store::{
            RootStoreBootstrapRequest, RootStoreBootstrapResponse, RootStoreCatalogEntry,
        },
    },
    ids::{
        AppId, CanisterRole, CanonicalNetworkId, ComponentSpecAdmission, CyclesFundingBudget,
        FleetBinding, FleetId, FleetKey, FleetSubnetRootLimits, FleetSubnetRootReleaseSet,
        ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
    },
    role_contract::ProtocolProfileDigest,
};

#[test]
fn root_journal_keeps_the_single_current_pre_one_schema_identifier() {
    assert_eq!(JOURNAL_SCHEMA_VERSION, 1);
}

#[test]
fn journals_each_root_effect_and_verifies_exact_protected_authority() {
    let root = temp_dir("fleet-subnet-root-install-journal");
    let fixture = fixture(&root);
    let planned = plan(&fixture).expect("plan root");
    assert_eq!(planned.journal.phase, FleetSubnetRootInstallPhase::Planned);
    assert_eq!(planned.journal.expected_root_module_hash, [7; 32]);
    assert_eq!(planned.journal.expected_wasm_store_module_hash, [9; 32]);
    assert_eq!(
        planned.journal.wasm_store_artifact.role,
        CanicInfrastructureRole::WasmStore
    );

    let installation_controller = Principal::from_slice(&[56]);
    let creating = begin_root_creation(&planned, installation_controller).expect("begin creation");
    assert_eq!(
        creating.journal.phase,
        FleetSubnetRootInstallPhase::RootCreationInFlight
    );
    assert_eq!(
        creating.journal.installation_controller,
        Some(installation_controller)
    );
    let root_canister = Principal::from_slice(&[44]);
    let created = record_root_created(&creating, root_canister).expect("record root");
    let creating_store = begin_wasm_store_creation(&created).expect("begin Store creation");
    let store_canister = Principal::from_slice(&[55]);
    let store_created =
        record_wasm_store_created(&creating_store, store_canister).expect("record Store");
    let store_installing = begin_wasm_store_install(&store_created).expect("begin Store install");
    let store_installed =
        record_wasm_store_installed(&store_installing, [9; 32]).expect("record Store module");
    let installing = begin_root_install(&store_installed).expect("begin root install");
    let installed = record_root_installed(&installing, [7; 32]).expect("record installed module");
    let authority = expected_root_authority(&installed.journal).expect("expected authority");
    let store_authority =
        expected_wasm_store_authority(&installed.journal).expect("expected Store authority");
    let verified =
        record_infrastructure_verified(&installed, authority.clone(), store_authority.clone())
            .expect("record exact authority");

    assert_eq!(
        verified.journal.phase,
        FleetSubnetRootInstallPhase::InfrastructureVerified
    );
    assert_eq!(verified.journal.sequence, 9);
    assert_eq!(verified.journal.verified_binding, Some(authority.binding));
    assert_eq!(
        verified.journal.verified_wasm_store_authority,
        Some(store_authority)
    );
}

#[test]
fn exact_retry_recovers_in_flight_root_without_advancing_it() {
    let root = temp_dir("fleet-subnet-root-install-journal-retry");
    let fixture = fixture(&root);
    let planned = plan(&fixture).expect("plan root");
    let installation_controller = Principal::from_slice(&[56]);
    let creating = begin_root_creation(&planned, installation_controller).expect("begin creation");

    let recovered = plan(&fixture).expect("recover root");
    assert_eq!(recovered.journal, creating.journal);
    assert!(!recovered.advanced);
    let repeated =
        begin_root_creation(&recovered, installation_controller).expect("recover creation intent");
    assert_eq!(repeated.journal, creating.journal);
    assert!(!repeated.advanced);
}

#[test]
fn journals_exact_store_bootstrap_and_rejects_a_catalog_outside_root_admissions() {
    let root = temp_dir("fleet-subnet-root-store-install-journal");
    let fixture = fixture(&root);
    let planned = plan(&fixture).expect("plan root");
    let root_canister = Principal::from_slice(&[44]);
    let verified = install_infrastructure(&planned, root_canister);
    let staging = begin_store_staging(&verified).expect("begin Store staging");
    let staged = record_store_staged(&staging).expect("record Store staging");
    let adopted = adopt_store(&staged);
    let bootstrapping = begin_store_bootstrap(&adopted).expect("begin Store bootstrap");
    let evidence = RootStoreBootstrapResponse {
        fleet_subnet_root: root_canister,
        wasm_store: Principal::from_slice(&[55]),
        release_set: fixture.plan.plan.fleet_subnet_roots[0].initial_release_set,
        catalog: vec![RootStoreCatalogEntry {
            role: CanisterRole::from("project_hub"),
            raw_module_hash: [8; 32],
            candid_sha256: [10; 32],
            protocol_profile_digest: ProtocolProfileDigest::from_bytes([11; 32]),
            payload_hash: [9; 32],
            payload_size_bytes: 1_024,
        }],
    };
    let bootstrapped =
        record_store_bootstrapped(&bootstrapping, evidence.clone()).expect("record Store");
    let complete =
        record_store_verified(&bootstrapped, evidence.clone()).expect("verify exact Store");

    assert_eq!(
        complete.journal.phase,
        FleetSubnetRootInstallPhase::StoreVerified
    );
    assert_eq!(complete.journal.sequence, 16);
    assert_eq!(complete.journal.store_bootstrap, Some(evidence));

    let mut inadmissible = complete
        .journal
        .store_bootstrap
        .clone()
        .expect("Store evidence");
    inadmissible.catalog.push(RootStoreCatalogEntry {
        role: CanisterRole::from("unplanned"),
        raw_module_hash: [11; 32],
        candid_sha256: [12; 32],
        protocol_profile_digest: ProtocolProfileDigest::from_bytes([13; 32]),
        payload_hash: [10; 32],
        payload_size_bytes: 1_024,
    });
    assert!(matches!(
        super::validate_store_bootstrap_evidence(&complete.path, &complete.journal, &inadmissible),
        Err(super::FleetSubnetRootInstallJournalError::InvalidDocument { .. })
    ));
}

#[test]
fn journals_exact_registry_join_sync_and_active_mirror_evidence() {
    let root = temp_dir("fleet-subnet-root-registry-join-journal");
    let fixture = fixture(&root);
    let planned = plan(&fixture).expect("plan root");
    let root_canister = Principal::from_slice(&[44]);
    let verified = install_infrastructure(&planned, root_canister);
    let staging = begin_store_staging(&verified).expect("begin Store staging");
    let staged = record_store_staged(&staging).expect("record Store staging");
    let adopted = adopt_store(&staged);
    let bootstrapping = begin_store_bootstrap(&adopted).expect("begin Store bootstrap");
    let store = RootStoreBootstrapResponse {
        fleet_subnet_root: root_canister,
        wasm_store: Principal::from_slice(&[55]),
        release_set: fixture.plan.plan.fleet_subnet_roots[0].initial_release_set,
        catalog: vec![RootStoreCatalogEntry {
            role: CanisterRole::from("project_hub"),
            raw_module_hash: [8; 32],
            candid_sha256: [10; 32],
            protocol_profile_digest: ProtocolProfileDigest::from_bytes([11; 32]),
            payload_hash: [9; 32],
            payload_size_bytes: 1_024,
        }],
    };
    let bootstrapped =
        record_store_bootstrapped(&bootstrapping, store.clone()).expect("record Store");
    let store_verified = record_store_verified(&bootstrapped, store).expect("verify exact Store");

    let genesis = FleetRegistryOps::compile_genesis(
        &fixture.plan.plan.fleet.app,
        store_verified.journal.authority.clone(),
        &fixture.topology,
    )
    .expect("genesis");
    let genesis_version = FleetRegistryOps::version(
        &store_verified.journal.authority,
        &fixture.topology,
        &genesis,
    )
    .expect("genesis version");
    let joining =
        begin_registry_join(&store_verified, genesis_version).expect("begin Registry join");
    assert_eq!(
        joining.journal.phase,
        FleetSubnetRootInstallPhase::RegistryJoinInFlight
    );
    assert_eq!(joining.journal.sequence, 17);
    let request = joining
        .journal
        .registry_join_request
        .clone()
        .expect("durable join request");
    let registry = FleetRegistryOps::compile_joining(
        &joining.journal.authority,
        &fixture.topology,
        &genesis,
        request.entry.clone(),
    )
    .expect("joined Registry");
    let manifest =
        FleetRegistryOps::manifest(&joining.journal.authority, &fixture.topology, &registry)
            .expect("joined manifest");
    let version =
        FleetRegistryOps::version(&joining.journal.authority, &fixture.topology, &registry)
            .expect("joined version");
    let response = FleetSubnetRootJoinResponse {
        entry: request.entry,
        version: version.clone(),
    };
    let joined = record_registry_joined(&joining, response.clone()).expect("record Registry join");
    let complete = record_registry_join_verified(&joined, &registry, &manifest, &version)
        .expect("verify joined Registry");

    assert_eq!(
        complete.journal.phase,
        FleetSubnetRootInstallPhase::RegistryJoinVerified
    );
    assert_eq!(complete.journal.sequence, 19);
    assert_eq!(complete.journal.registry_join_response, Some(response));

    assert_registry_sync_journal(&complete, root_canister, version);
}

fn assert_registry_sync_journal(
    joined: &super::ResolvedFleetSubnetRootInstall,
    root_canister: Principal,
    version: canic_core::dto::fleet_registry::FleetRegistryVersion,
) {
    let sync_request = FleetSubnetRootRegistrySyncRequest {
        operation_id: [8; 32],
        expected_registry: version.clone(),
        store_bootstrap: RootStoreBootstrapRequest {
            operation_id: [9; 32],
            manifest_payload_size_bytes: 1_024,
        },
    };
    let synchronizing =
        begin_registry_sync(joined, sync_request.clone()).expect("begin Registry sync");
    let acknowledgement = FleetSubnetRootSnapshotAcknowledgement {
        fleet_subnet_root: root_canister,
        version: version.clone(),
    };
    let sync_response = FleetSubnetRootRegistrySyncResponse {
        fleet_subnet_root: root_canister,
        version: version.clone(),
        acknowledgement,
    };
    let synchronized = record_registry_synchronized(&synchronizing, sync_response.clone())
        .expect("record Registry sync");
    let verified = record_registry_sync_verified(&synchronized, sync_response.clone())
        .expect("verify Registry sync");

    assert_eq!(
        verified.journal.phase,
        FleetSubnetRootInstallPhase::RegistrySyncVerified
    );
    assert_eq!(verified.journal.sequence, 22);
    assert_eq!(
        verified.journal.registry_sync_request,
        Some(sync_request.clone())
    );
    assert_eq!(verified.journal.registry_sync_response, Some(sync_response));

    assert_registry_mirror_and_component_registry_journal(
        &verified,
        root_canister,
        version,
        sync_request,
    );
}

fn assert_registry_mirror_and_component_registry_journal(
    verified: &super::ResolvedFleetSubnetRootInstall,
    root_canister: Principal,
    version: canic_core::dto::fleet_registry::FleetRegistryVersion,
    sync_request: FleetSubnetRootRegistrySyncRequest,
) {
    let active_version = canic_core::dto::fleet_registry::FleetRegistryVersion {
        authority: version.authority.clone(),
        revision: version.revision.checked_add(1).expect("next revision"),
        content_hash: [12; 32],
    };
    let directory = FleetDirectorySnapshot {
        provenance: FleetDirectoryProvenance {
            registry: active_version.clone(),
            source_fleet_subnet_root: root_canister,
        },
        fleet_subnet_roots: vec![FleetSubnetRootDirectoryEntry {
            placement_subnet: verified.journal.root_plan.placement_subnet,
            fleet_subnet_root: root_canister,
            status: canic_core::dto::fleet_registry::FleetSubnetRootStatus::Active,
        }],
        services: vec![],
    };
    let activation_request = FleetSubnetRootRegistryMirrorActivationRequest {
        previous_registry: version,
        expected_registry: active_version.clone(),
        expected_directory: directory.clone(),
        store_bootstrap: sync_request.store_bootstrap,
    };
    let activating = begin_registry_mirror_activation(verified, activation_request.clone())
        .expect("begin Registry mirror activation");
    let activation_response = FleetSubnetRootRegistryMirrorActivationResponse {
        fleet_subnet_root: root_canister,
        previous_registry: activation_request.previous_registry.clone(),
        version: active_version,
        directory,
    };
    let activated = record_registry_mirror_activated(&activating, activation_response.clone())
        .expect("record Registry mirror activation");
    let activation_verified =
        record_registry_mirror_activation_verified(&activated, activation_response.clone())
            .expect("verify Registry mirror activation");

    assert_eq!(
        activation_verified.journal.phase,
        FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified
    );
    assert_eq!(activation_verified.journal.sequence, 25);
    assert_eq!(
        activation_verified
            .journal
            .registry_mirror_activation_request,
        Some(activation_request.clone())
    );
    assert_eq!(
        activation_verified
            .journal
            .registry_mirror_activation_response,
        Some(activation_response.clone())
    );

    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: activation_request.store_bootstrap,
        expected_fleet_registry: activation_response.version.clone(),
    };
    let preparing =
        begin_component_registry_preparation(&activation_verified, preparation_request.clone())
            .expect("begin Component Registry preparation");
    let preparation_response = RootComponentRegistryStatusResponse {
        fleet_subnet_root: root_canister,
        prepared_against_registry: activation_response.version,
        release_set: preparing.journal.root_plan.initial_release_set,
        component_topology_digest: preparing.journal.root_plan.component_topology_digest,
        next_allocation_sequence: 1,
        reserved_component_instances: 0,
        committed_component_instances: 0,
        managed_descendants: 0,
        known_created_component_canisters: 0,
        encoded_bytes: 0,
        initial_inventory: None,
    };
    let prepared = record_component_registry_prepared(&preparing, preparation_response.clone())
        .expect("record Component Registry preparation");
    let preparation_verified =
        record_component_registry_preparation_verified(&prepared, preparation_response.clone())
            .expect("verify Component Registry preparation");

    assert_eq!(
        preparation_verified.journal.phase,
        FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified
    );
    assert_eq!(preparation_verified.journal.sequence, 28);
    assert_eq!(
        preparation_verified
            .journal
            .component_registry_preparation_request,
        Some(preparation_request)
    );
    assert_eq!(
        preparation_verified
            .journal
            .component_registry_preparation_response,
        Some(preparation_response)
    );
}

fn install_infrastructure(
    planned: &super::ResolvedFleetSubnetRootInstall,
    root_canister: Principal,
) -> super::ResolvedFleetSubnetRootInstall {
    let creating =
        begin_root_creation(planned, Principal::from_slice(&[56])).expect("begin root creation");
    let root_created = record_root_created(&creating, root_canister).expect("record root");
    let creating_store =
        begin_wasm_store_creation(&root_created).expect("begin Wasm Store creation");
    let store_created = record_wasm_store_created(&creating_store, Principal::from_slice(&[55]))
        .expect("record Wasm Store");
    let installing_store =
        begin_wasm_store_install(&store_created).expect("begin Wasm Store install");
    let store_installed =
        record_wasm_store_installed(&installing_store, [9; 32]).expect("record Store module");
    let installing_root = begin_root_install(&store_installed).expect("begin root install");
    let root_installed =
        record_root_installed(&installing_root, [7; 32]).expect("record root module");
    let root_authority =
        expected_root_authority(&root_installed.journal).expect("expected root authority");
    let store_authority =
        expected_wasm_store_authority(&root_installed.journal).expect("expected Store authority");
    record_infrastructure_verified(&root_installed, root_authority, store_authority)
        .expect("record infrastructure authority")
}

fn adopt_store(
    verified: &super::ResolvedFleetSubnetRootInstall,
) -> super::ResolvedFleetSubnetRootInstall {
    let adopting = begin_store_adoption(verified).expect("begin Store adoption");
    let authority =
        expected_wasm_store_authority(&adopting.journal).expect("expected Store authority");
    let mut temporary_controllers = vec![
        authority.installation_controller,
        authority.fleet_subnet_root,
    ];
    temporary_controllers.sort();
    let evidence = FleetSubnetWasmStoreAdoptionResponse {
        operation_id: super::super::root_store_adoption_operation_id(
            adopting.journal.install_operation_id,
        ),
        authority: authority.clone(),
        temporary_controllers,
        final_controllers: vec![authority.fleet_subnet_root],
        adopted_at_ns: 10,
    };
    record_store_adopted(&adopting, evidence).expect("record Store adoption")
}

#[test]
fn planner_rejects_zero_install_identity_and_a_root_outside_the_fleet_plan() {
    let root = temp_dir("fleet-subnet-root-install-journal-authority");
    let fixture = fixture(&root);
    let zero_identity = PlanFleetSubnetRootInstallRequest {
        fleet_install_plan: &fixture.plan,
        infrastructure_manifest: &fixture.manifest,
        coordinator: Principal::from_slice(&[33]),
        install_operation_id: [0; 32],
        component_topology: fixture.topology.clone(),
        root_plan: &fixture.plan.plan.fleet_subnet_roots[0],
    };
    assert!(matches!(
        plan_fleet_subnet_root_install(zero_identity),
        Err(super::FleetSubnetRootInstallJournalError::InvalidDocument { .. })
    ));

    let mut unplanned_root = fixture.plan.plan.fleet_subnet_roots[0].clone();
    unplanned_root.placement_subnet = subnet(3);
    let unplanned = PlanFleetSubnetRootInstallRequest {
        fleet_install_plan: &fixture.plan,
        infrastructure_manifest: &fixture.manifest,
        coordinator: Principal::from_slice(&[33]),
        install_operation_id: [11; 32],
        component_topology: fixture.topology.clone(),
        root_plan: &unplanned_root,
    };
    assert!(matches!(
        plan_fleet_subnet_root_install(unplanned),
        Err(super::FleetSubnetRootInstallJournalError::InvalidDocument { .. })
    ));
}

#[test]
fn planner_requires_the_exact_sibling_wasm_store_artifact() {
    let root = temp_dir("fleet-subnet-root-install-journal-store-artifact");
    let mut fixture = fixture(&root);
    fixture
        .manifest
        .manifest
        .entries
        .retain(|entry| entry.role != CanicInfrastructureRole::WasmStore);

    assert!(matches!(
        plan(&fixture),
        Err(super::FleetSubnetRootInstallJournalError::WasmStoreArtifactMissing)
    ));
}

struct Fixture {
    plan: PersistedFleetInstallPlan,
    manifest: PersistedCanicInfrastructureArtifactManifest,
    topology: ComponentTopology,
}

fn plan(
    fixture: &Fixture,
) -> Result<super::ResolvedFleetSubnetRootInstall, super::FleetSubnetRootInstallJournalError> {
    plan_fleet_subnet_root_install(PlanFleetSubnetRootInstallRequest {
        fleet_install_plan: &fixture.plan,
        infrastructure_manifest: &fixture.manifest,
        coordinator: Principal::from_slice(&[33]),
        install_operation_id: [11; 32],
        component_topology: fixture.topology.clone(),
        root_plan: &fixture.plan.plan.fleet_subnet_roots[0],
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "one fixture assembles the complete correlated install-plan authority"
)]
fn fixture(root: &Path) -> Fixture {
    let release_build_id =
        ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([6; 32]));
    let topology = topology();
    let admission = ComponentSpecAdmission {
        component_spec: "projects".parse().expect("Component Spec"),
        spec_hash: [8; 32],
        maximum_root_instances: 10,
    };
    let topology_digest = topology
        .project_for_admissions(std::slice::from_ref(&admission))
        .expect("project topology")
        .digest()
        .expect("topology digest");
    let root_plan = PlannedFleetSubnetRoot {
        placement_subnet: subnet(2),
        component_group_placements: Vec::new(),
        component_admissions: vec![admission],
        component_topology_digest: topology_digest,
        initial_release_set: FleetSubnetRootReleaseSet {
            release_build_id,
            manifest_digest: ReleaseSetDigest::from_bytes([5; 32]),
        },
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 100,
            maximum_registry_bytes: 2_097_152,
            maximum_wasm_store_bytes: 268_435_456,
            maximum_group_placements: 16,
            canister_pool: canic_core::ids::FleetSubnetCanisterPoolConfig {
                minimum_size: 1,
                maximum_size: 10,
                canister_cycles: Cycles::new(5_000_000_000_000),
            },
            cycles_funding: cycles_budget(),
        },
        canister_pool_imports: Vec::new(),
        root_creation_funding: PlannedCanisterCreationFunding::Cycles {
            cycles: 2_000_000_000_000,
        },
        wasm_store_creation_funding: PlannedCanisterCreationFunding::Cycles {
            cycles: 2_000_000_000_000,
        },
    };
    let plan = PersistedFleetInstallPlan {
        plan: FleetInstallPlan {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([4; 32]),
                },
                app: AppId::from("toko"),
            },
            release_build_id,
            application_artifact_union_digest: [3; 32],
            coordinator: PlannedFleetCoordinator {
                coordinator_subnet: subnet(1),
                creation_funding: PlannedCanisterCreationFunding::Cycles {
                    cycles: 2_000_000_000_000,
                },
            },
            fleet_subnet_roots: vec![root_plan],
        },
        digest: [9; 32],
        path: root.join("fleet-install-plan.json"),
        root_release_sets: Vec::new(),
    };
    let root_artifact = CanicInfrastructureArtifactEntry {
        role: CanicInfrastructureRole::FleetSubnetRoot,
        package: "canic-fleet-subnet-root".to_string(),
        protocol_release_identity: "0.103.0".to_string(),
        protocol_role: canic_core::ids::CanisterRole::new("fleet_subnet_root"),
        protocol_capabilities: std::collections::BTreeSet::new(),
        release_build_id,
        wasm_relative_path: "root.wasm".to_string(),
        wasm_size_bytes: 8,
        wasm_sha256_hex: "07".repeat(32),
        wasm_gz_relative_path: "root.wasm.gz".to_string(),
        wasm_gz_size_bytes: 8,
        wasm_gz_sha256_hex: "08".repeat(32),
        candid_sha256: [3; 32],
        protocol_profile_digest: canic_core::role_contract::ProtocolProfileDigest::from_bytes(
            [4; 32],
        ),
    };
    let wasm_store_artifact = CanicInfrastructureArtifactEntry {
        role: CanicInfrastructureRole::WasmStore,
        package: "canic-wasm-store".to_string(),
        protocol_release_identity: "0.103.0".to_string(),
        protocol_role: canic_core::ids::CanisterRole::new("wasm_store"),
        protocol_capabilities: std::collections::BTreeSet::new(),
        release_build_id,
        wasm_relative_path: "wasm_store.wasm".to_string(),
        wasm_size_bytes: 8,
        wasm_sha256_hex: "09".repeat(32),
        wasm_gz_relative_path: "wasm_store.wasm.gz".to_string(),
        wasm_gz_size_bytes: 8,
        wasm_gz_sha256_hex: "0a".repeat(32),
        candid_sha256: [5; 32],
        protocol_profile_digest: canic_core::role_contract::ProtocolProfileDigest::from_bytes(
            [6; 32],
        ),
    };
    let manifest = PersistedCanicInfrastructureArtifactManifest {
        manifest: CanicInfrastructureArtifactManifest {
            release_build_id,
            entries: vec![root_artifact, wasm_store_artifact],
        },
        digest: [10; 32],
        path: root.join("infrastructure-artifact-manifest.json"),
    };
    Fixture {
        plan,
        manifest,
        topology,
    }
}

fn topology() -> ComponentTopology {
    ComponentTopology {
        component_specs: vec![ComponentSpec {
            component_spec: "projects".parse().expect("Component Spec"),
            spec_hash: [8; 32],
            component_role: CanisterRole::from("project_hub"),
            maximum_fleet_instances: 10,
            limits: ComponentLimits {
                maximum_descendants: 20_000,
                maximum_registry_bytes: 2_097_152,
                cycles_funding: cycles_budget(),
            },
            children: Vec::new(),
            spawn_grants: Vec::new(),
        }],
        provisioning_grants: Vec::new(),
    }
}

fn cycles_budget() -> CyclesFundingBudget {
    CyclesFundingBudget {
        window_secs: 3_600,
        maximum_cycles: Cycles::new(2_000_000_000_000),
    }
}

fn subnet(byte: u8) -> SubnetId {
    SubnetId::from_principal(Principal::from_slice(&[byte; 29]))
}
