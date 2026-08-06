//! Module: install_root::fleet_registry_activation_journal::tests
//!
//! Responsibility: prove exact activation authority, monotonic evidence, and retry recovery.
//! Does not own: live Coordinator transport or root final-mirror coverage.

use super::*;
use crate::{
    fleet_install_plan::{
        FleetInstallPlan, PersistedFleetInstallPlan, PlannedCanisterCreationFunding,
        PlannedFleetCoordinator, PlannedFleetSubnetRoot,
    },
    test_support::temp_dir,
};
use candid::Principal;
use canic_core::{
    bootstrap::parse_config_model,
    cdk::types::Cycles,
    dto::fleet_registry::{FleetSubnetRootEntry, FleetSubnetRootStatus},
    ids::{
        AppId, CanonicalNetworkId, ComponentSpecAdmission, CyclesFundingBudget, FleetBinding,
        FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetRootLimits,
        FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
    },
};

#[test]
fn journal_recovers_exact_atomic_registry_activation_evidence() {
    let root = temp_dir("fleet-registry-activation-journal");
    let (plan, topology, joining) = fixture(&root);
    let request = || PlanFleetRegistryActivationRequest {
        fleet_install_plan: &plan,
        component_topology: topology.clone(),
        joining_registry: joining.clone(),
    };
    let planned = plan_fleet_registry_activation(request()).expect("plan activation");
    let in_flight = begin_registry_activation(&planned).expect("begin activation");
    let response = FleetRegistryActivationResponse {
        previous_version: in_flight.journal.request.expected_registry.clone(),
        version: FleetRegistryOps::version(
            &in_flight.journal.active_registry.authority,
            &topology,
            &in_flight.journal.active_registry,
        )
        .expect("active version"),
    };
    let activated =
        record_registry_activated(&in_flight, response.clone()).expect("record activation");
    let manifest = FleetRegistryOps::manifest(
        &activated.journal.active_registry.authority,
        &topology,
        &activated.journal.active_registry,
    )
    .expect("active manifest");
    let verified =
        record_registry_activation_verified(&activated, manifest.clone(), response.version.clone())
            .expect("verify activation");

    assert_eq!(
        verified.journal.phase,
        FleetRegistryActivationPhase::Verified
    );
    assert_eq!(verified.journal.sequence, 3);
    assert_eq!(verified.journal.response, Some(response));
    assert_eq!(verified.journal.verified_manifest, Some(manifest));
    assert!(
        verified
            .journal
            .active_registry
            .fleet_subnet_roots
            .iter()
            .all(|entry| entry.status == FleetSubnetRootStatus::Active)
    );
    assert_eq!(
        plan_fleet_registry_activation(request())
            .expect("recover exact activation")
            .journal,
        verified.journal
    );
}

#[test]
fn journal_rejects_changed_source_or_response_authority() {
    let root = temp_dir("fleet-registry-activation-journal-conflict");
    let (plan, topology, joining) = fixture(&root);
    let planned = plan_fleet_registry_activation(PlanFleetRegistryActivationRequest {
        fleet_install_plan: &plan,
        component_topology: topology.clone(),
        joining_registry: joining.clone(),
    })
    .expect("plan activation");
    let in_flight = begin_registry_activation(&planned).expect("begin activation");
    let mut wrong_response = FleetRegistryActivationResponse {
        previous_version: in_flight.journal.request.expected_registry.clone(),
        version: FleetRegistryOps::version(
            &joining.authority,
            &topology,
            &in_flight.journal.active_registry,
        )
        .expect("active version"),
    };
    wrong_response.version.content_hash[0] ^= 1;
    assert!(matches!(
        record_registry_activated(&in_flight, wrong_response),
        Err(FleetRegistryActivationJournalError::InvalidDocument { .. })
    ));

    let mut changed = joining;
    changed.fleet_subnet_roots[0].limits.maximum_registry_bytes += 1;
    assert!(matches!(
        plan_fleet_registry_activation(PlanFleetRegistryActivationRequest {
            fleet_install_plan: &plan,
            component_topology: topology,
            joining_registry: changed,
        }),
        Err(FleetRegistryActivationJournalError::InvalidDocument { .. }
            | FleetRegistryActivationJournalError::ConflictingAuthority { .. })
    ));
}

fn fixture(root: &Path) -> (PersistedFleetInstallPlan, ComponentTopology, FleetRegistry) {
    let topology = parse_config_model(
        r#"
[app]
name = "toko"

[roles.root]
kind = "root"
package = "root"

[roles.project]
kind = "canister"
package = "project"

[component_specs.projects]
component_role = "project"
maximum_instances = 2
"#,
    )
    .expect("valid config")
    .compile_component_topology()
    .expect("Component Topology");
    let release_build_id =
        ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([6; 32]));
    let spec = topology
        .component_specs
        .first()
        .expect("one Component Spec");
    let admission = ComponentSpecAdmission {
        component_spec: spec.component_spec.clone(),
        spec_hash: spec.spec_hash,
        maximum_root_instances: 2,
    };
    let limits = root_limits();
    let fleet = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([4; 32]),
        },
        app: AppId::from("toko"),
    };
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: fleet.clone(),
            coordinator_subnet: subnet(1),
            coordinator: principal(33),
        },
        epoch: 1,
    };
    let release_set = FleetSubnetRootReleaseSet {
        release_build_id,
        manifest_digest: ReleaseSetDigest::from_bytes([5; 32]),
    };
    let topology_digest = topology
        .project_for_admissions(std::slice::from_ref(&admission))
        .expect("root topology")
        .digest()
        .expect("topology digest");
    let root_plan = PlannedFleetSubnetRoot {
        placement_subnet: subnet(2),
        component_admissions: vec![admission.clone()],
        component_topology_digest: topology_digest,
        initial_release_set: release_set,
        limits: limits.clone(),
        canister_pool_imports: Vec::new(),
        root_creation_funding: funding(),
        wasm_store_creation_funding: funding(),
    };
    let plan = PersistedFleetInstallPlan {
        plan: FleetInstallPlan {
            fleet: fleet.clone(),
            release_build_id,
            application_artifact_union_digest: [3; 32],
            coordinator: PlannedFleetCoordinator {
                coordinator_subnet: subnet(1),
                creation_funding: funding(),
            },
            fleet_subnet_roots: vec![root_plan],
        },
        digest: [9; 32],
        path: root.join("fleet-install-plan.json"),
        root_release_sets: Vec::new(),
    };
    let mut joining =
        FleetRegistryOps::compile_genesis(&fleet.app, authority, &topology).expect("genesis");
    joining = FleetRegistryOps::compile_joining(
        &joining.authority,
        &topology,
        &joining,
        FleetSubnetRootEntry {
            placement_subnet: subnet(2),
            fleet_subnet_root: principal(44),
            component_admissions: vec![admission],
            component_topology_digest: topology_digest,
            active_release_set: release_set,
            limits,
            status: FleetSubnetRootStatus::Joining,
        },
    )
    .expect("Joining root");
    (plan, topology, joining)
}

fn funding() -> PlannedCanisterCreationFunding {
    PlannedCanisterCreationFunding::Cycles {
        cycles: 2_000_000_000_000,
    }
}

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn subnet(byte: u8) -> SubnetId {
    SubnetId::from_principal(principal(byte))
}

fn root_limits() -> FleetSubnetRootLimits {
    FleetSubnetRootLimits {
        maximum_component_instances: 2,
        maximum_registry_bytes: 2_097_152,
        maximum_wasm_store_bytes: 268_435_456,
        maximum_group_placements: 16,
        canister_pool: canic_core::ids::FleetSubnetCanisterPoolConfig {
            minimum_size: 1,
            maximum_size: 10,
            canister_cycles: Cycles::new(5_000_000_000_000),
        },
        cycles_funding: CyclesFundingBudget {
            window_secs: 3_600,
            maximum_cycles: Cycles::new(2_000_000_000_000),
        },
    }
}
