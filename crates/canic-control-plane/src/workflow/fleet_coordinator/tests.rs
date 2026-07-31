//! Module: workflow::fleet_coordinator::tests
//!
//! Responsibility: qualify protected genesis commitment and canonical Coordinator queries.
//! Does not own: PocketIC installation or host effect-journal coverage.

use super::*;
use crate::storage::stable::fleet_coordinator::{
    FleetCoordinatorRegistryData, FleetCoordinatorRegistryStore,
};
use canic_core::{
    bootstrap::parse_config_model,
    cdk::types::Cycles,
    dto::{
        error::ErrorCode,
        fleet_registry::{
            FleetRegistryActivationRequest, FleetSubnetRootDrainingPublicationRequest,
            FleetSubnetRootEntry, FleetSubnetRootJoinRequest, FleetSubnetRootStatus,
        },
        fleet_subnet_root::FleetSubnetRootDrainingResponse,
    },
    ids::{
        AppId, CanonicalNetworkId, ComponentSpecAdmission, CyclesFundingBudget, FleetBinding,
        FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetRootLimits,
        FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
    },
};

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn init_args(coordinator: Principal) -> FleetCoordinatorInitArgs {
    let component_topology = parse_config_model(
        r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.project]
kind = "canister"
package = "project"

[component_specs.projects]
component_role = "project"
maximum_instances = 3
"#,
    )
    .expect("valid config")
    .compile_component_topology()
    .expect("Component Topology");
    FleetCoordinatorInitArgs {
        configured_app: AppId::from("demo"),
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                        fleet_id: FleetId::from_generated_bytes([7; 32]),
                    },
                    app: AppId::from("demo"),
                },
                coordinator_subnet: SubnetId::from_principal(principal(2)),
                coordinator,
            },
            epoch: 1,
        },
        component_topology,
    }
}

#[test]
fn protected_init_commits_exact_genesis_and_supports_exact_retry() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let coordinator = principal(3);
    let controller = principal(4);
    let args = init_args(coordinator);

    FleetCoordinatorWorkflow::initialize(args.clone(), controller, true, coordinator)
        .expect("commit genesis");
    FleetCoordinatorWorkflow::initialize(args, controller, true, coordinator)
        .expect("repeat exact genesis");

    let registry = FleetCoordinatorWorkflow::registry().expect("Registry");
    let manifest = FleetCoordinatorWorkflow::manifest().expect("manifest");
    let version = FleetCoordinatorWorkflow::version().expect("version");

    assert_eq!(registry.revision, 1);
    assert_eq!(registry.component_specs.len(), 1);
    assert!(registry.fleet_subnet_roots.is_empty());
    assert_eq!(manifest.revision, registry.revision);
    assert_eq!(version.content_hash, manifest.content_hash);

    let unauthorized = FleetCoordinatorWorkflow::initialize(
        init_args(coordinator),
        principal(5),
        false,
        coordinator,
    )
    .expect_err("reject non-controller init");
    assert_eq!(
        unauthorized.public_error().map(|error| error.code),
        Some(ErrorCode::Forbidden)
    );

    let wrong_canister = FleetCoordinatorWorkflow::initialize(
        init_args(principal(6)),
        controller,
        true,
        coordinator,
    )
    .expect_err("reject wrong Coordinator binding");
    assert_eq!(
        wrong_canister.public_error().map(|error| error.code),
        Some(ErrorCode::InvalidInput)
    );
}

#[test]
fn root_join_compare_and_commit_retains_exact_response_receipts() {
    FleetCoordinatorRegistryStore::import(FleetCoordinatorRegistryData::default());
    let coordinator = principal(13);
    let args = init_args(coordinator);
    let topology = args.component_topology.clone();
    FleetCoordinatorWorkflow::initialize(args, principal(14), true, coordinator)
        .expect("commit genesis");

    let genesis = FleetCoordinatorWorkflow::version().expect("genesis version");
    let first_entry = joining_entry(&topology, 7, 15, 1);
    let first_request = FleetSubnetRootJoinRequest {
        expected_registry: genesis.clone(),
        entry: first_entry.clone(),
    };
    let first =
        FleetCoordinatorWorkflow::join_root(first_request.clone()).expect("join first root");
    assert_eq!(first.entry, first_entry);
    assert_eq!(first.version.revision, 2);
    assert_eq!(
        FleetCoordinatorWorkflow::join_root(first_request.clone()).expect("exact first retry"),
        first
    );

    let second_entry = joining_entry(&topology, 5, 16, 2);
    let second_request = FleetSubnetRootJoinRequest {
        expected_registry: first.version.clone(),
        entry: second_entry.clone(),
    };
    let second = FleetCoordinatorWorkflow::join_root(second_request).expect("join second root");
    assert_eq!(second.version.revision, 3);
    assert_eq!(
        FleetCoordinatorWorkflow::join_root(first_request).expect("late exact first retry"),
        first,
        "the original response must survive later Registry revisions"
    );

    let registry = FleetCoordinatorWorkflow::registry().expect("joined Registry");
    assert_eq!(registry.revision, 3);
    assert_eq!(
        registry
            .fleet_subnet_roots
            .iter()
            .map(|entry| entry.placement_subnet)
            .collect::<Vec<_>>(),
        vec![second_entry.placement_subnet, first_entry.placement_subnet]
    );

    let active_version =
        assert_snapshot_acknowledgements(&registry, &first_entry, &second_entry, &second.version);
    assert_root_draining_publication(&first_entry, &second_entry, &active_version);

    let stale = FleetCoordinatorWorkflow::join_root(FleetSubnetRootJoinRequest {
        expected_registry: genesis,
        entry: joining_entry(&topology, 9, 17, 1),
    })
    .expect_err("a new root cannot commit against stale Registry authority");
    assert_eq!(
        stale.public_error().map(|error| error.code),
        Some(ErrorCode::Conflict)
    );

    let mut conflicting_entry = first_entry;
    conflicting_entry.limits.maximum_managed_canisters += 1;
    let conflict = FleetCoordinatorWorkflow::join_root(FleetSubnetRootJoinRequest {
        expected_registry: second.version,
        entry: conflicting_entry,
    })
    .expect_err("an existing root identity cannot change authority");
    assert_eq!(
        conflict.public_error().map(|error| error.code),
        Some(ErrorCode::Conflict)
    );

    let mut corrupted = FleetCoordinatorRegistryStore::export();
    corrupted
        .current
        .as_mut()
        .expect("Coordinator state")
        .root_join_receipts[0]
        .version
        .content_hash[0] ^= 1;
    FleetCoordinatorRegistryStore::import(corrupted);
    let invalid = crate::api::fleet_coordinator::FleetCoordinatorApi::registry()
        .expect_err("reject corrupted historical receipt");
    assert_eq!(invalid.code, ErrorCode::InvariantViolation);
}

fn assert_snapshot_acknowledgements(
    registry: &FleetRegistry,
    first_entry: &FleetSubnetRootEntry,
    second_entry: &FleetSubnetRootEntry,
    version: &FleetRegistryVersion,
) -> FleetRegistryVersion {
    let snapshot = FleetCoordinatorWorkflow::snapshot_for_root(first_entry.fleet_subnet_root)
        .expect("registered root snapshot");
    assert_eq!(&snapshot.registry, registry);
    assert_eq!(&snapshot.version, version);
    let unauthorized_snapshot = FleetCoordinatorWorkflow::snapshot_for_root(principal(99))
        .expect_err("unregistered caller cannot fetch snapshot");
    assert_eq!(
        unauthorized_snapshot.public_error().map(|error| error.code),
        Some(ErrorCode::Forbidden)
    );

    let request = canic_core::dto::fleet_registry::FleetSubnetRootSnapshotAcknowledgementRequest {
        version: version.clone(),
    };
    let first_ack = FleetCoordinatorWorkflow::acknowledge_root_snapshot(
        first_entry.fleet_subnet_root,
        request.clone(),
    )
    .expect("first acknowledgement");
    assert_eq!(
        FleetCoordinatorWorkflow::acknowledge_root_snapshot(
            first_entry.fleet_subnet_root,
            request.clone(),
        )
        .expect("exact acknowledgement retry"),
        first_ack
    );
    let activation_request = FleetRegistryActivationRequest {
        expected_registry: version.clone(),
    };
    let incomplete = FleetCoordinatorWorkflow::activate_registry(activation_request.clone())
        .expect_err("activation requires every root acknowledgement");
    assert_eq!(
        incomplete.public_error().map(|error| error.code),
        Some(ErrorCode::Conflict)
    );
    FleetCoordinatorWorkflow::acknowledge_root_snapshot(second_entry.fleet_subnet_root, request)
        .expect("second acknowledgement");
    let acknowledgements =
        FleetCoordinatorWorkflow::root_snapshot_acknowledgements().expect("acknowledgements");
    assert_eq!(acknowledgements.len(), 2);
    assert!(acknowledgements.iter().all(|ack| &ack.version == version));

    let activated = FleetCoordinatorWorkflow::activate_registry(activation_request.clone())
        .expect("activate complete acknowledged Registry");
    assert_eq!(&activated.previous_version, version);
    assert_eq!(activated.version.revision, version.revision + 1);
    assert_eq!(
        FleetCoordinatorWorkflow::activate_registry(activation_request)
            .expect("exact activation retry"),
        activated
    );
    let registry = FleetCoordinatorWorkflow::registry().expect("active Registry");
    assert!(
        registry
            .fleet_subnet_roots
            .iter()
            .all(|entry| entry.status == FleetSubnetRootStatus::Active)
    );
    assert!(
        FleetCoordinatorWorkflow::root_snapshot_acknowledgements()
            .expect("cleared acknowledgements")
            .is_empty()
    );
    activated.version
}

fn assert_root_draining_publication(
    first_entry: &FleetSubnetRootEntry,
    second_entry: &FleetSubnetRootEntry,
    active_version: &FleetRegistryVersion,
) {
    let request = FleetSubnetRootDrainingPublicationRequest {
        expected_registry: active_version.clone(),
        root_draining: FleetSubnetRootDrainingResponse {
            operation_id: [21; 32],
            fleet_subnet_root: first_entry.fleet_subnet_root,
            placement_subnet: first_entry.placement_subnet,
            active_registry: active_version.clone(),
            component_topology_digest: first_entry.component_topology_digest,
            active_release_set: first_entry.active_release_set,
            next_allocation_sequence: 3,
            reserved_component_instances: 1,
            committed_component_instances: 1,
            managed_descendants: 2,
            known_created_component_canisters: 3,
            root_registry_encoded_bytes: 1_024,
            started_at_ns: 22,
        },
    };
    let before_invalid = FleetCoordinatorRegistryStore::export();
    let mut oversized = request.clone();
    oversized.root_draining.root_registry_encoded_bytes =
        first_entry.limits.maximum_registry_bytes + 1;
    let invalid = FleetCoordinatorWorkflow::publish_root_draining(oversized)
        .expect_err("reject root draining receipt outside protected limits");
    assert_eq!(
        invalid.public_error().map(|error| error.code),
        Some(ErrorCode::InvalidInput)
    );
    assert_eq!(FleetCoordinatorRegistryStore::export(), before_invalid);

    let published = FleetCoordinatorWorkflow::publish_root_draining(request.clone())
        .expect("publish root Draining");
    assert_eq!(&published.previous_version, active_version);
    assert_eq!(published.version.revision, active_version.revision + 1);

    let durable = FleetCoordinatorRegistryStore::export();
    FleetCoordinatorRegistryStore::import(durable);
    assert_eq!(
        FleetCoordinatorWorkflow::publish_root_draining(request.clone())
            .expect("exact publication retry after restart"),
        published
    );
    let registry = FleetCoordinatorWorkflow::registry().expect("Draining Registry");
    assert_eq!(
        registry
            .fleet_subnet_roots
            .iter()
            .find(|entry| entry.fleet_subnet_root == first_entry.fleet_subnet_root)
            .expect("first root")
            .status,
        FleetSubnetRootStatus::Draining
    );
    assert_eq!(
        registry
            .fleet_subnet_roots
            .iter()
            .find(|entry| entry.fleet_subnet_root == second_entry.fleet_subnet_root)
            .expect("second root")
            .status,
        FleetSubnetRootStatus::Active
    );

    let mut conflicting = request;
    conflicting.root_draining.operation_id[0] ^= 1;
    let conflict = FleetCoordinatorWorkflow::publish_root_draining(conflicting)
        .expect_err("one root cannot publish different draining authority");
    assert_eq!(
        conflict.public_error().map(|error| error.code),
        Some(ErrorCode::Conflict)
    );

    let valid = FleetCoordinatorRegistryStore::export();
    let mut corrupted = valid.clone();
    corrupted
        .current
        .as_mut()
        .expect("Coordinator state")
        .root_draining_publication_receipts[0]
        .response
        .version
        .content_hash[0] ^= 1;
    FleetCoordinatorRegistryStore::import(corrupted);
    let invalid = crate::api::fleet_coordinator::FleetCoordinatorApi::registry()
        .expect_err("reject corrupted root Draining publication receipt");
    assert_eq!(invalid.code, ErrorCode::InvariantViolation);
    FleetCoordinatorRegistryStore::import(valid);
}

fn joining_entry(
    topology: &canic_core::control_plane_support::config::ComponentTopology,
    subnet_byte: u8,
    root_byte: u8,
    maximum_root_instances: u32,
) -> FleetSubnetRootEntry {
    let spec = topology
        .component_specs
        .first()
        .expect("one Component Spec");
    let component_admissions = vec![ComponentSpecAdmission {
        component_spec: spec.component_spec.clone(),
        spec_hash: spec.spec_hash,
        maximum_root_instances,
    }];
    let component_topology_digest = topology
        .project_for_admissions(&component_admissions)
        .expect("root topology")
        .digest()
        .expect("root topology digest");
    FleetSubnetRootEntry {
        placement_subnet: SubnetId::from_principal(principal(subnet_byte)),
        fleet_subnet_root: principal(root_byte),
        component_admissions,
        component_topology_digest,
        active_release_set: FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [18; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([root_byte; 32]),
        },
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 3,
            maximum_managed_canisters: 100,
            maximum_registry_bytes: 2_097_152,
            maximum_wasm_store_bytes: 268_435_456,
            cycles_funding: CyclesFundingBudget {
                window_secs: 3_600,
                maximum_cycles: Cycles::new(2_000_000_000_000),
            },
        },
        status: FleetSubnetRootStatus::Joining,
    }
}
