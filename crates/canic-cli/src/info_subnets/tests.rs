//! Module: canic_cli::info_subnets::tests
//!
//! Responsibility: prove command parsing, complete evidence aggregation, and stable rendering.
//! Does not own: live PocketIC lifecycle coverage or ICP CLI compatibility.

use super::*;
use crate::info_subnets::{
    model::{SubnetInventoryError, SubnetInventoryPlan},
    render::text_report,
};
use candid::Principal;
use canic_core::{
    cdk::types::Cycles,
    dto::{
        fleet_registry::{
            FleetRegistry, FleetRegistryManifest, FleetRegistryVersion, FleetSubnetRootEntry,
            FleetSubnetRootStatus,
        },
        fleet_subnet_root::FleetSubnetRootCanisterSummary,
    },
    ids::{
        AppId, CanonicalNetworkId, ComponentTopologyDigest, CyclesFundingBudget, FleetBinding,
        FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetRootLimits,
        FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
    },
};
use canic_host::fleet_catalog::FleetCatalogEntryV1;

#[test]
fn parses_fleet_json_and_forwarded_global_options() {
    let options = InfoSubnetsOptions::parse([
        OsString::from("toko"),
        OsString::from("--json"),
        OsString::from("--__canic-environment"),
        OsString::from("staging"),
        OsString::from("--__canic-icp"),
        OsString::from("/opt/icp"),
    ])
    .expect("parse subnet inventory");

    assert_eq!(options.fleet, "toko");
    assert!(options.json);
    assert_eq!(options.environment, "staging");
    assert_eq!(options.icp, "/opt/icp");
}

#[test]
fn complete_evidence_groups_a_colocated_coordinator_and_root() {
    let fixture = fixture();
    let report = fixture
        .plan()
        .expect("compile plan")
        .complete(fixture.summaries())
        .expect("complete report");

    assert_eq!(report.schema_version, 1);
    assert_eq!(report.registry_revision, 4);
    assert_eq!(report.subnets.len(), 2);
    assert_eq!(report.total_canisters, 13);
    assert_eq!(report.subnets[0].coordinator_canisters, 1);
    assert_eq!(report.subnets[0].root_infrastructure_canisters, 2);
    assert_eq!(report.subnets[0].component_canisters, 2);
    assert_eq!(report.subnets[0].pooled_canisters, 2);
    assert_eq!(report.subnets[0].total_canisters, 7);
    assert_eq!(report.subnets[1].coordinator_canisters, 0);
    assert_eq!(report.subnets[1].total_canisters, 6);

    let json = serde_json::to_value(&report).expect("serialize report");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["coordinator_principal"], principal(30).to_text());
    assert_eq!(json["subnets"][0]["status"], "active");
    assert_eq!(json["subnets"][0]["pooled_canisters"], 2);
    assert_eq!(json["subnets"][0]["total_canisters"], 7);
    let report_keys = json
        .as_object()
        .expect("report object")
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(
        report_keys,
        [
            "schema_version",
            "canonical_network_id",
            "fleet_id",
            "fleet",
            "app",
            "coordinator_principal",
            "registry_revision",
            "total_canisters",
            "subnets",
        ]
    );
    let row_keys = json["subnets"][0]
        .as_object()
        .expect("row object")
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(
        row_keys,
        [
            "subnet",
            "coordinator_canisters",
            "root",
            "status",
            "root_infrastructure_canisters",
            "component_canisters",
            "pooled_canisters",
            "total_canisters",
        ]
    );
}

#[test]
fn complete_evidence_rejects_missing_or_stale_root_summaries() {
    let fixture = fixture();
    let mut summaries = fixture.summaries();
    summaries.pop();
    assert!(matches!(
        fixture.plan().expect("compile plan").complete(summaries),
        Err(SubnetInventoryError::MissingSummary { .. })
    ));

    let mut summaries = fixture.summaries();
    summaries[0].fleet_registry.revision += 1;
    assert!(matches!(
        fixture.plan().expect("compile plan").complete(summaries),
        Err(SubnetInventoryError::SummaryMismatch { .. })
    ));
}

#[test]
fn coordinator_evidence_must_match_the_terminal_catalog() {
    let mut fixture = fixture();
    fixture.registry.authority.binding.coordinator = principal(99);

    assert!(matches!(
        fixture.plan(),
        Err(SubnetInventoryError::CatalogAuthorityMismatch {
            field: "coordinator_principal"
        })
    ));
}

#[test]
fn removed_roots_are_excluded_from_queries_and_rows() {
    let mut fixture = fixture();
    fixture.registry.fleet_subnet_roots[1].status = FleetSubnetRootStatus::Removed;
    let plan = fixture.plan().expect("compile plan");

    assert_eq!(plan.root_principals(), vec![principal(40)]);
    let summary = fixture
        .summaries()
        .into_iter()
        .next()
        .expect("active root summary");
    let report = plan.complete(vec![summary]).expect("complete report");
    assert_eq!(report.subnets.len(), 1);
    assert_eq!(report.total_canisters, 7);
}

#[test]
fn text_output_contains_canonical_rows_and_exact_fleet_total() {
    let fixture = fixture();
    let report = fixture
        .plan()
        .expect("compile plan")
        .complete(fixture.summaries())
        .expect("complete report");
    let text = text_report(&report);

    assert!(text.contains("SUBNET"));
    assert!(text.contains("ROOT"));
    assert!(text.contains("STATUS"));
    assert!(text.contains("CANISTERS"));
    assert!(text.contains("ACTIVE"));
    assert!(text.contains("POOL"));
    assert!(text.contains("Fleet total: 13 Canisters"));
}

struct Fixture {
    catalog: FleetCatalogEntryV1,
    registry: FleetRegistry,
    manifest: FleetRegistryManifest,
    version: FleetRegistryVersion,
}

impl Fixture {
    fn plan(&self) -> Result<SubnetInventoryPlan, SubnetInventoryError> {
        SubnetInventoryPlan::compile(
            self.catalog.clone(),
            self.registry.clone(),
            self.manifest.clone(),
            self.version.clone(),
        )
    }

    fn summaries(&self) -> Vec<FleetSubnetRootCanisterSummary> {
        self.registry
            .fleet_subnet_roots
            .iter()
            .map(|root| FleetSubnetRootCanisterSummary {
                fleet_registry: self.version.clone(),
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
}

fn fixture() -> Fixture {
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
            coordinator: principal(30),
        },
        epoch: 1,
    };
    let registry = FleetRegistry {
        authority: authority.clone(),
        revision: 4,
        component_specs: Vec::new(),
        fleet_subnet_roots: vec![
            root(1, 40, FleetSubnetRootStatus::Active),
            root(2, 50, FleetSubnetRootStatus::Active),
        ],
    };
    let manifest = FleetRegistryManifest {
        authority: authority.clone(),
        revision: registry.revision,
        byte_length: 1_024,
        content_hash: [8; 32],
    };
    let version = FleetRegistryVersion {
        authority,
        revision: registry.revision,
        content_hash: manifest.content_hash,
    };
    Fixture {
        catalog: FleetCatalogEntryV1 {
            canonical_network_id: fleet.fleet.canonical_network_id,
            fleet_id: fleet.fleet.fleet_id,
            fleet_name: "toko-production".parse().expect("Fleet name"),
            app: fleet.app,
            environment: "staging".to_string(),
            deployed_at_unix_secs: 54,
            coordinator_principal: principal(30).to_text(),
        },
        registry,
        manifest,
        version,
    }
}

fn root(
    subnet_byte: u8,
    principal_byte: u8,
    status: FleetSubnetRootStatus,
) -> FleetSubnetRootEntry {
    FleetSubnetRootEntry {
        placement_subnet: subnet(subnet_byte),
        fleet_subnet_root: principal(principal_byte),
        component_admissions: Vec::new(),
        component_topology_digest: ComponentTopologyDigest::from_bytes([principal_byte; 32]),
        active_release_set: FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [6; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([5; 32]),
        },
        limits: FleetSubnetRootLimits {
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
        },
        status,
    }
}

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn subnet(byte: u8) -> SubnetId {
    SubnetId::from_principal(principal(byte))
}
