use super::*;
use crate::{
    fleet_install_plan::{FleetInstallPlan, PersistedFleetInstallPlan, PlannedFleetCoordinator},
    release_set::{
        CanicInfrastructureArtifactManifest, PersistedCanicInfrastructureArtifactManifest,
    },
    test_support::temp_dir,
};
use canic_core::{
    control_plane_support::ops::fleet_registry::FleetRegistryOps,
    ids::{
        AppId, CanonicalNetworkId, FleetCoordinatorBinding, FleetId, FleetKey,
        FleetRegistryAuthority, ReleaseBuildNonce,
    },
};

#[test]
fn journals_every_coordinator_effect_before_advancing() {
    let root = temp_dir("coordinator-install-journal");
    let plan = persisted_plan(&root);
    let manifest = persisted_manifest(&root, plan.plan.release_build_id);
    let planned = plan_fleet_coordinator_install(PlanFleetCoordinatorInstallRequest {
        fleet_install_plan: &plan,
        infrastructure_manifest: &manifest,
        component_topology: empty_topology(),
    })
    .expect("plan Coordinator install");
    assert_eq!(planned.journal.phase, FleetCoordinatorInstallPhase::Planned);

    let creating = begin_coordinator_creation(&planned).expect("begin creation");
    assert_eq!(
        creating.journal.phase,
        FleetCoordinatorInstallPhase::CreationInFlight
    );
    let coordinator = Principal::from_slice(&[91]);
    let created = record_coordinator_created(&creating, coordinator).expect("record created");
    assert_eq!(created.journal.coordinator, Some(coordinator));

    let installing = begin_coordinator_install(&created).expect("begin install");
    assert_eq!(
        installing.journal.phase,
        FleetCoordinatorInstallPhase::InstallInFlight
    );
    let installed =
        record_coordinator_installed(&installing, [7; 32]).expect("record exact module");
    assert_eq!(
        installed.journal.phase,
        FleetCoordinatorInstallPhase::Installed
    );
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: installed.journal.fleet.clone(),
            coordinator_subnet: installed.journal.coordinator_subnet,
            coordinator,
        },
        epoch: 1,
    };
    let registry = FleetRegistryOps::compile_genesis(
        &installed.journal.fleet.app,
        authority.clone(),
        &installed.journal.component_topology,
    )
    .expect("compile Registry genesis");
    let manifest =
        FleetRegistryOps::manifest(&authority, &installed.journal.component_topology, &registry)
            .expect("Registry manifest");
    let version =
        FleetRegistryOps::version(&authority, &installed.journal.component_topology, &registry)
            .expect("Registry version");
    let verified =
        record_coordinator_verified(&installed, manifest, version).expect("record verification");
    assert_eq!(
        verified.journal.phase,
        FleetCoordinatorInstallPhase::Verified
    );
    assert_eq!(verified.journal.sequence, 5);
}

#[test]
fn exact_retry_recovers_in_flight_without_advancing_again() {
    let root = temp_dir("coordinator-install-journal-retry");
    let plan = persisted_plan(&root);
    let manifest = persisted_manifest(&root, plan.plan.release_build_id);
    let planned = plan_fleet_coordinator_install(PlanFleetCoordinatorInstallRequest {
        fleet_install_plan: &plan,
        infrastructure_manifest: &manifest,
        component_topology: empty_topology(),
    })
    .expect("plan Coordinator install");
    let creating = begin_coordinator_creation(&planned).expect("begin creation");

    let recovered = plan_fleet_coordinator_install(PlanFleetCoordinatorInstallRequest {
        fleet_install_plan: &plan,
        infrastructure_manifest: &manifest,
        component_topology: empty_topology(),
    })
    .expect("recover Coordinator install");

    assert_eq!(recovered.journal, creating.journal);
    assert!(!recovered.advanced);
    let repeated = begin_coordinator_creation(&recovered).expect("recover in-flight intent");
    assert_eq!(repeated.journal, creating.journal);
    assert!(!repeated.advanced);
}

fn persisted_plan(root: &Path) -> PersistedFleetInstallPlan {
    let release_build_id =
        ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([6; 32]));
    PersistedFleetInstallPlan {
        plan: FleetInstallPlan {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([5; 32]),
                },
                app: AppId::from("demo"),
            },
            release_build_id,
            application_artifact_union_digest: [3; 32],
            coordinator: PlannedFleetCoordinator {
                coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[90])),
                creation_funding: PlannedCanisterCreationFunding::Cycles {
                    cycles: 2_000_000_000_000,
                },
            },
            fleet_subnet_roots: Vec::new(),
        },
        digest: [4; 32],
        path: root.join("fleet-install-plan.json"),
        root_release_sets: Vec::new(),
    }
}

fn persisted_manifest(
    root: &Path,
    release_build_id: ReleaseBuildId,
) -> PersistedCanicInfrastructureArtifactManifest {
    let artifact = CanicInfrastructureArtifactEntry {
        role: CanicInfrastructureRole::FleetCoordinator,
        package: "canic-coordinator".to_string(),
        release_build_id,
        wasm_relative_path: "coordinator.wasm".to_string(),
        wasm_size_bytes: 8,
        wasm_sha256_hex: "07".repeat(32),
        wasm_gz_relative_path: "coordinator.wasm.gz".to_string(),
        wasm_gz_size_bytes: 8,
        wasm_gz_sha256_hex: "08".repeat(32),
    };
    PersistedCanicInfrastructureArtifactManifest {
        manifest: CanicInfrastructureArtifactManifest {
            release_build_id,
            entries: vec![artifact],
        },
        digest: [9; 32],
        path: root.join("infrastructure-artifact-manifest.json"),
    }
}

fn empty_topology() -> ComponentTopology {
    ComponentTopology {
        component_specs: Vec::new(),
        provisioning_grants: Vec::new(),
    }
}
