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
        FleetSubnetRootInstallPhase, PlanFleetSubnetRootInstallRequest, begin_registry_join,
        begin_root_creation, begin_root_install, begin_store_bootstrap, begin_store_staging,
        expected_root_authority, plan_fleet_subnet_root_install, record_registry_join_verified,
        record_registry_joined, record_root_created, record_root_installed, record_root_verified,
        record_store_bootstrapped, record_store_staged, record_store_verified,
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
        fleet_registry::FleetSubnetRootJoinResponse,
        root_store::{RootStoreBootstrapResponse, RootStoreCatalogEntry},
    },
    ids::{
        AppId, CanisterRole, CanonicalNetworkId, ComponentSpecAdmission, CyclesFundingBudget,
        FleetBinding, FleetId, FleetKey, FleetSubnetRootLimits, FleetSubnetRootReleaseSet,
        ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
    },
};

#[test]
fn journals_each_root_effect_and_verifies_exact_protected_authority() {
    let root = temp_dir("fleet-subnet-root-install-journal");
    let fixture = fixture(&root);
    let planned = plan(&fixture).expect("plan root");
    assert_eq!(planned.journal.phase, FleetSubnetRootInstallPhase::Planned);

    let creating = begin_root_creation(&planned).expect("begin creation");
    assert_eq!(
        creating.journal.phase,
        FleetSubnetRootInstallPhase::CreationInFlight
    );
    let root_canister = Principal::from_slice(&[44]);
    let created = record_root_created(&creating, root_canister).expect("record root");
    let installing = begin_root_install(&created).expect("begin install");
    let installed = record_root_installed(&installing, [7; 32]).expect("record installed module");
    let authority = expected_root_authority(&installed.journal).expect("expected authority");
    let verified =
        record_root_verified(&installed, authority.clone()).expect("record exact authority");

    assert_eq!(
        verified.journal.phase,
        FleetSubnetRootInstallPhase::Verified
    );
    assert_eq!(verified.journal.sequence, 5);
    assert_eq!(verified.journal.verified_binding, Some(authority.binding));
}

#[test]
fn exact_retry_recovers_in_flight_root_without_advancing_it() {
    let root = temp_dir("fleet-subnet-root-install-journal-retry");
    let fixture = fixture(&root);
    let planned = plan(&fixture).expect("plan root");
    let creating = begin_root_creation(&planned).expect("begin creation");

    let recovered = plan(&fixture).expect("recover root");
    assert_eq!(recovered.journal, creating.journal);
    assert!(!recovered.advanced);
    let repeated = begin_root_creation(&recovered).expect("recover creation intent");
    assert_eq!(repeated.journal, creating.journal);
    assert!(!repeated.advanced);
}

#[test]
fn journals_exact_store_bootstrap_and_rejects_a_catalog_outside_root_admissions() {
    let root = temp_dir("fleet-subnet-root-store-install-journal");
    let fixture = fixture(&root);
    let planned = plan(&fixture).expect("plan root");
    let creating = begin_root_creation(&planned).expect("begin creation");
    let root_canister = Principal::from_slice(&[44]);
    let created = record_root_created(&creating, root_canister).expect("record root");
    let installing = begin_root_install(&created).expect("begin install");
    let installed = record_root_installed(&installing, [7; 32]).expect("record installed module");
    let authority = expected_root_authority(&installed.journal).expect("expected authority");
    let verified = record_root_verified(&installed, authority).expect("record exact authority");
    let staging = begin_store_staging(&verified).expect("begin Store staging");
    let staged = record_store_staged(&staging).expect("record Store staging");
    let bootstrapping = begin_store_bootstrap(&staged).expect("begin Store bootstrap");
    let evidence = RootStoreBootstrapResponse {
        fleet_subnet_root: root_canister,
        wasm_store: Principal::from_slice(&[55]),
        release_set: fixture.plan.plan.fleet_subnet_roots[0].initial_release_set,
        catalog: vec![RootStoreCatalogEntry {
            role: CanisterRole::from("project_hub"),
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
    assert_eq!(complete.journal.sequence, 10);
    assert_eq!(complete.journal.store_bootstrap, Some(evidence));

    let mut inadmissible = complete
        .journal
        .store_bootstrap
        .clone()
        .expect("Store evidence");
    inadmissible.catalog.push(RootStoreCatalogEntry {
        role: CanisterRole::from("unplanned"),
        payload_hash: [10; 32],
        payload_size_bytes: 1_024,
    });
    assert!(matches!(
        super::validate_store_bootstrap_evidence(&complete.path, &complete.journal, &inadmissible),
        Err(super::FleetSubnetRootInstallJournalError::InvalidDocument { .. })
    ));
}

#[test]
fn journals_exact_registry_join_request_response_and_independent_verification() {
    let root = temp_dir("fleet-subnet-root-registry-join-journal");
    let fixture = fixture(&root);
    let planned = plan(&fixture).expect("plan root");
    let creating = begin_root_creation(&planned).expect("begin creation");
    let root_canister = Principal::from_slice(&[44]);
    let created = record_root_created(&creating, root_canister).expect("record root");
    let installing = begin_root_install(&created).expect("begin install");
    let installed = record_root_installed(&installing, [7; 32]).expect("record installed module");
    let authority = expected_root_authority(&installed.journal).expect("expected authority");
    let verified = record_root_verified(&installed, authority).expect("record exact authority");
    let staging = begin_store_staging(&verified).expect("begin Store staging");
    let staged = record_store_staged(&staging).expect("record Store staging");
    let bootstrapping = begin_store_bootstrap(&staged).expect("begin Store bootstrap");
    let store = RootStoreBootstrapResponse {
        fleet_subnet_root: root_canister,
        wasm_store: Principal::from_slice(&[55]),
        release_set: fixture.plan.plan.fleet_subnet_roots[0].initial_release_set,
        catalog: vec![RootStoreCatalogEntry {
            role: CanisterRole::from("project_hub"),
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
    assert_eq!(joining.journal.sequence, 11);
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
    assert_eq!(complete.journal.sequence, 13);
    assert_eq!(complete.journal.registry_join_response, Some(response));
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
        component_admissions: vec![admission],
        component_topology_digest: topology_digest,
        initial_release_set: FleetSubnetRootReleaseSet {
            release_build_id,
            manifest_digest: ReleaseSetDigest::from_bytes([5; 32]),
        },
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 100,
            maximum_managed_canisters: 20_000,
            maximum_registry_bytes: 2_097_152,
            maximum_wasm_store_bytes: 268_435_456,
            cycles_funding: cycles_budget(),
        },
        creation_funding: PlannedCanisterCreationFunding::Cycles {
            cycles: 2_000_000_000_000,
        },
    };
    let plan = PersistedFleetInstallPlan {
        plan: FleetInstallPlan {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::public_ic(),
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
    let artifact = CanicInfrastructureArtifactEntry {
        role: CanicInfrastructureRole::FleetSubnetRoot,
        package: "canic-fleet-subnet-root".to_string(),
        release_build_id,
        wasm_relative_path: "root.wasm".to_string(),
        wasm_size_bytes: 8,
        wasm_sha256_hex: "07".repeat(32),
        wasm_gz_relative_path: "root.wasm.gz".to_string(),
        wasm_gz_size_bytes: 8,
        wasm_gz_sha256_hex: "08".repeat(32),
    };
    let manifest = PersistedCanicInfrastructureArtifactManifest {
        manifest: CanicInfrastructureArtifactManifest {
            release_build_id,
            entries: vec![artifact],
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
