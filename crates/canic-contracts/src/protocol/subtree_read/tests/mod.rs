//! Full native phase evidence and exact query identity.
use super::*;
use crate::{
    dto::{
        component_registry::{
            ComponentLifecycleStatus, ComponentRegistryHead, ComponentRuntimeActivationEvidence,
            RootComponentSubtreeRemovalResponse,
        },
        subtree_read::*,
    },
    ids::ComponentInstanceId,
};

#[test]
fn every_subtree_boundary_round_trips_complete_evidence() {
    let component = ComponentInstanceId::from_generated_bytes([1; 32]);
    let registry = ComponentRegistryHead {
        component,
        revision: u64::MAX,
        content_hash: [2; 32],
    };
    let node = Node {
        canister_id: ic_platform::Principal::from_slice(&[3]),
        parent_canister_id: ic_platform::Principal::from_slice(&[4]),
        role: "child".into(),
        kind: crate::deployment::ComponentChildKind::Shard,
        installed_artifact_hash: [5; 32],
        status: ComponentLifecycleStatus::Draining,
    };
    let stop = StopIntent {
        leaf: node.clone(),
        controller: ic_platform::Principal::from_slice(&[6]),
    };
    let stopped = Stopped {
        stop: stop.clone(),
        observed_module_hash: [7; 32],
    };
    let deletion = DeleteIntent {
        stopped: stopped.clone(),
    };
    let deleted = Deleted {
        deletion: deletion.clone(),
    };
    let membership = MembershipRemoved {
        deleted: deleted.clone(),
        removed_from_registry: registry.clone(),
        previous_descendant_content_hash: [8; 32],
        previous_committed_descendants: u32::MAX,
        registry: registry.clone(),
        descendant_content_hash: [9; 32],
        registry_encoded_bytes: u64::MAX,
        reserved_descendants: 10,
        committed_descendants: 11,
        directory_synchronized_at_ns: u64::MAX,
        directory_authority_hash: [12; 32],
        parent_role_instances: 13,
        root_managed_descendants: 14,
        root_known_created_component_canisters: 15,
    };
    let convergence = DirectoryConvergence {
        operation_id: [16; 32],
        canister_id: node.parent_canister_id,
        activation: ComponentRuntimeActivationEvidence {
            directory_authority_hash: [17; 32],
            activated_at_ns: u64::MAX,
        },
    };
    let synchronized = DirectorySynchronized {
        membership_removed: membership.clone(),
        covered_fleet_registry_revision: u64::MAX,
        covered_fleet_registry_content_hash: [18; 32],
        covered_component_registry: registry.clone(),
        covered_authority_hash: [19; 32],
        owning_component: Some(convergence.clone()),
        parent: Some(convergence),
    };
    let mut quiescent = synchronized.clone();
    quiescent.owning_component = None;
    quiescent.parent = None;
    for phase in [
        Phase::Fenced,
        Phase::Traversing(Box::new(node.clone())),
        Phase::LeafSelected(Box::new(node.clone())),
        Phase::StopIntent(Box::new(stop)),
        Phase::Stopped(Box::new(stopped)),
        Phase::DeleteIntent(Box::new(deletion)),
        Phase::Deleted(Box::new(deleted)),
        Phase::MembershipRemoved(Box::new(membership)),
        Phase::DirectorySynchronized(Box::new(synchronized)),
        Phase::DirectorySynchronized(Box::new(quiescent)),
        Phase::Completed(Box::new(Completed {
            registry: registry.clone(),
            directory_authority_hash: [20; 32],
        })),
    ] {
        let status = Status {
            operation_id: [21; 32],
            component,
            target_canister_id: node.canister_id,
            target_parent_canister_id: node.parent_canister_id,
            target_role: node.role.clone(),
            target_status: node.status,
            reserved_against_registry: registry.clone(),
            maximum_completed_leaves: u32::MAX,
            completed_leaves: 22,
            traversal_steps: 23,
            phase,
        };
        assert_round_trip(status);
    }
}

#[test]
fn subtree_query_rejects_other_operation_frames() {
    let operation = [24_u8; 32];
    let bytes = ic_wire::native::encode_request(METHOD_ID, &operation, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<[u8; 32]>(METHOD_ID, &bytes, REQUEST_LIMITS).unwrap(),
        operation
    );
    assert!(matches!(
        ic_wire::native::decode_request::<[u8; 32]>(
            crate::protocol::component_read::allocation::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn subtree_schema_covers_every_retained_receipt() {
    ic_wire::typescript::native_schema::<Result<Status, crate::dto::error::Error>>().unwrap();
}

fn assert_round_trip(status: Status) {
    let canonical = RootComponentSubtreeRemovalResponse::from(status.clone());
    let bytes = ic_wire::native::encode_response(METHOD_ID, &status, RESPONSE_LIMITS).unwrap();
    let decoded: Status =
        ic_wire::native::decode_response(METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    assert_eq!(decoded, status);
    assert_eq!(
        RootComponentSubtreeRemovalResponse::from(decoded),
        canonical
    );
    assert_eq!(Status::from(canonical), status);
}
