//! Module: install_root::fleet_catalog_publication::tests
//!
//! Responsibility: prove terminal evidence gates Coordinator-anchored catalog publication.
//! Does not own: live query transport or lifecycle execution fixtures.

use super::*;
use crate::{
    fleet_catalog::{FleetCatalogRequest, build_fleet_catalog_report},
    fleet_install_plan::{
        PlannedCanisterCreationFunding, PlannedFleetCoordinator, PlannedFleetSubnetRoot,
    },
    test_support::temp_dir,
};
use std::{fs, path::PathBuf};

use canic_core::{
    bootstrap::parse_config_model,
    cdk::types::Cycles,
    dto::fleet_registry::{FleetRegistry, FleetSubnetRootEntry},
    ids::{
        AppId, CanonicalNetworkId, ComponentSpecAdmission, CyclesFundingBudget, FleetBinding,
        FleetId, FleetKey, FleetSubnetRootLimits, FleetSubnetRootReleaseSet, ReleaseBuildId,
        ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
    },
};

#[test]
fn exact_terminal_evidence_publishes_one_coordinator_row_and_retries_exactly() {
    let fixture = fixture("exact");

    let committed =
        publish_terminal_fleet_catalog(fixture.request()).expect("publish terminal Fleet");
    let repeated =
        publish_terminal_fleet_catalog(fixture.request()).expect("repeat terminal Fleet");

    assert!(committed.advanced);
    assert!(!repeated.advanced);
    assert_eq!(repeated.entry, committed.entry);
    assert_eq!(repeated.catalog_hash, committed.catalog_hash);
    assert_eq!(
        committed.entry.coordinator_principal,
        fixture.coordinator.to_text()
    );
    let report = build_fleet_catalog_report(&FleetCatalogRequest {
        workspace_root: fixture.root.clone(),
        environment: "staging".to_string(),
        generated_at: "unix:54".to_string(),
    })
    .expect("read terminal Fleet catalog");
    assert_eq!(report.entries, vec![committed.entry]);
    fs::remove_dir_all(fixture.root).expect("remove fixture");
}

#[test]
fn publication_rejects_missing_or_contradictory_root_summaries_before_write() {
    let mut fixture = fixture("root-evidence");
    fixture.root_summaries.clear();
    assert!(matches!(
        publish_terminal_fleet_catalog(fixture.request()),
        Err(TerminalFleetCatalogPublicationError::RootSetMismatch)
    ));

    fixture.root_summaries = summaries(&fixture.registry);
    fixture.root_summaries[0].component_canisters = 3;
    assert!(matches!(
        publish_terminal_fleet_catalog(fixture.request()),
        Err(TerminalFleetCatalogPublicationError::RootSummaryMismatch { .. })
    ));
    assert!(!fixture.catalog_path().exists());
    fs::remove_dir_all(fixture.root).expect("remove fixture");
}

#[test]
fn publication_accepts_consistent_root_summaries_without_an_aggregate_physical_cap() {
    let mut fixture = fixture("uncapped-root-evidence");
    fixture.root_summaries[0].pooled_canisters = 20_000;
    fixture.root_summaries[0].total_canisters = 20_004;

    publish_terminal_fleet_catalog(fixture.request())
        .expect("publish consistent uncapped root evidence");

    assert!(fixture.catalog_path().exists());
    fs::remove_dir_all(fixture.root).expect("remove fixture");
}

#[test]
fn publication_rejects_coordinator_and_registry_evidence_drift() {
    let mut fixture = fixture("authority-drift");
    fixture.coordinator = principal(55);
    assert!(matches!(
        publish_terminal_fleet_catalog(fixture.request()),
        Err(TerminalFleetCatalogPublicationError::AuthorityMismatch)
    ));

    fixture.coordinator = principal(33);
    fixture.registry.version.content_hash = [99; 32];
    assert!(matches!(
        publish_terminal_fleet_catalog(fixture.request()),
        Err(TerminalFleetCatalogPublicationError::RegistryEvidenceMismatch)
    ));
    assert!(!fixture.catalog_path().exists());
    fs::remove_dir_all(fixture.root).expect("remove fixture");
}

#[test]
fn publication_rejects_reserved_coordinator_identity() {
    let mut fixture = fixture("reserved-coordinator");
    fixture.coordinator = Principal::management_canister();

    assert!(matches!(
        publish_terminal_fleet_catalog(fixture.request()),
        Err(TerminalFleetCatalogPublicationError::AuthorityMismatch)
    ));
    assert!(!fixture.catalog_path().exists());
    fs::remove_dir_all(fixture.root).expect("remove fixture");
}

struct Fixture {
    root: PathBuf,
    plan: FleetInstallPlan,
    topology: ComponentTopology,
    coordinator: Principal,
    registry: FleetRegistrySnapshotResponse,
    root_summaries: Vec<FleetSubnetRootCanisterSummary>,
}

impl Fixture {
    fn request(&self) -> TerminalFleetCatalogPublicationRequest<'_> {
        TerminalFleetCatalogPublicationRequest {
            workspace_root: &self.root,
            fleet_name: "toko-production".parse().expect("Fleet name"),
            environment: "staging",
            deployed_at_unix_secs: 54,
            fleet_install_plan: &self.plan,
            component_topology: &self.topology,
            coordinator: self.coordinator,
            registry: &self.registry,
            root_summaries: &self.root_summaries,
        }
    }

    fn catalog_path(&self) -> PathBuf {
        self.root
            .join(".canic")
            .join("networks")
            .join(self.plan.fleet.fleet.canonical_network_id.to_string())
            .join("fleets/catalog.json")
    }
}

fn fixture(name: &str) -> Fixture {
    let root = temp_dir(&format!("terminal-fleet-catalog-{name}"));
    fs::create_dir_all(&root).expect("create fixture");
    fs::write(
        root.join("icp.yaml"),
        "environments:\n  - name: staging\n    network: ic\n",
    )
    .expect("write ICP config");
    let topology = topology();
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
    let release_set = FleetSubnetRootReleaseSet {
        release_build_id,
        manifest_digest: ReleaseSetDigest::from_bytes([5; 32]),
    };
    let topology_digest = topology
        .project_for_admissions(std::slice::from_ref(&admission))
        .expect("root topology")
        .digest()
        .expect("topology digest");
    let coordinator = principal(33);
    let fleet = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([4; 32]),
        },
        app: AppId::from("toko"),
    };
    let plan = FleetInstallPlan {
        fleet: fleet.clone(),
        fresh_fleet_plan_digest: "ab".repeat(32),
        release_build_id,
        application_artifact_union_digest: [3; 32],
        coordinator: PlannedFleetCoordinator {
            coordinator_subnet: subnet(1),
            creation_funding: funding(),
            root_funding: Some(crate::test_support::coordinator_root_funding_policy()),
        },
        fleet_subnet_roots: vec![PlannedFleetSubnetRoot {
            placement_subnet: subnet(2),
            component_group_placements: Vec::new(),
            component_admissions: vec![admission.clone()],
            component_topology_digest: topology_digest,
            initial_release_set: release_set,
            limits: limits.clone(),
            funding: crate::test_support::fleet_subnet_root_funding_authority(),
            canister_pool_imports: Vec::new(),
            root_creation_funding: funding(),
            wasm_store_creation_funding: funding(),
        }],
    };
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet,
            coordinator_subnet: subnet(1),
            coordinator,
        },
        epoch: 1,
    };
    let mut registry =
        FleetRegistryOps::compile_genesis(&plan.fleet.app, authority, &topology).expect("genesis");
    registry = FleetRegistryOps::compile_joining(
        &registry.authority,
        &topology,
        &registry,
        FleetSubnetRootEntry {
            placement_subnet: subnet(2),
            fleet_subnet_root: principal(44),
            component_admissions: vec![admission],
            component_topology_digest: topology_digest,
            active_release_set: release_set,
            limits,
            funding: crate::test_support::fleet_subnet_root_funding_authority(),
            status: FleetSubnetRootStatus::Joining,
        },
    )
    .expect("Joining root");
    registry = FleetRegistryOps::compile_active(&registry.authority, &topology, &registry)
        .expect("Active");
    let registry = snapshot(registry, &topology);
    let root_summaries = summaries(&registry);
    Fixture {
        root,
        plan,
        topology,
        coordinator,
        registry,
        root_summaries,
    }
}

fn topology() -> ComponentTopology {
    parse_config_model(
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
    .expect("Component Topology")
}

fn snapshot(
    registry: FleetRegistry,
    topology: &ComponentTopology,
) -> FleetRegistrySnapshotResponse {
    let manifest = FleetRegistryOps::manifest(&registry.authority, topology, &registry)
        .expect("Registry manifest");
    let version =
        FleetRegistryOps::version(&registry.authority, topology, &registry).expect("version");
    FleetRegistrySnapshotResponse {
        registry,
        manifest,
        version,
    }
}

fn summaries(registry: &FleetRegistrySnapshotResponse) -> Vec<FleetSubnetRootCanisterSummary> {
    registry
        .registry
        .fleet_subnet_roots
        .iter()
        .map(|root| FleetSubnetRootCanisterSummary {
            fleet_registry: registry.version.clone(),
            placement_subnet: root.placement_subnet,
            fleet_subnet_root: root.fleet_subnet_root,
            status: root.status,
            infrastructure_canisters: 2,
            component_canisters: 2,
            pooled_canisters: 2,
            total_canisters: 6,
        })
        .collect()
}

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn subnet(byte: u8) -> SubnetId {
    SubnetId::from_principal(principal(byte))
}

fn funding() -> PlannedCanisterCreationFunding {
    PlannedCanisterCreationFunding::Cycles {
        cycles: 2_000_000_000_000,
    }
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
