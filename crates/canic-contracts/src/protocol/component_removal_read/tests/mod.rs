//! Exact framing and complete destructive-operation evidence round trips.
use super::*;
use crate::{
    dto::{
        component_registry::{
            ComponentRegistryHead, ComponentRuntimeActivationEvidence,
            RootComponentDrainingResponse, RootComponentFinalInventory,
        },
        component_removal_read::*,
        root::RootComponentRemovalOperationStatus,
    },
    ids::ComponentInstanceId,
};

fn registry() -> ComponentRegistryHead {
    ComponentRegistryHead {
        component: ComponentInstanceId::from_generated_bytes([1; 32]),
        revision: u64::MAX,
        content_hash: [2; 32],
    }
}

fn intent() -> DeleteIntent {
    DeleteIntent {
        final_inventory: RootComponentFinalInventory {
            registry: registry(),
            descendant_content_hash: [3; 32],
            registry_encoded_bytes: u64::MAX,
            directory_synchronized_at_ns: 4,
            covered_fleet_registry_revision: 5,
            covered_fleet_registry_content_hash: [6; 32],
            directory_authority_hash: [7; 32],
            inventory_hash: [8; 32],
            finalized_at_ns: 9,
        },
        quiescence: Quiescent {
            stop: StopIntent {
                registry: registry(),
                descendant_count: 10,
                descendant_content_hash: [11; 32],
                canister_id: ic_platform::Principal::from_slice(&[12]),
                controller: ic_platform::Principal::from_slice(&[13]),
                expected_module_hash: [14; 32],
                covered_fleet_registry_revision: 15,
                covered_fleet_registry_content_hash: [16; 32],
                covered_authority_hash: [17; 32],
                runtime_operation_id: [18; 32],
                activation: ComponentRuntimeActivationEvidence {
                    directory_authority_hash: [19; 32],
                    activated_at_ns: 20,
                },
                prepared_at_ns: 21,
            },
            observed_module_hash: [22; 32],
            quiesced_at_ns: 23,
        },
        prepared_at_ns: 24,
    }
}

#[test]
fn component_removal_preserves_each_boundary_and_unstarted_deletion() {
    let deleted = Deleted {
        deletion: intent(),
        deleted_at_ns: u64::MAX,
    };
    let removed = MembershipRemoved {
        deleted: deleted.clone(),
        allocation_operation_id: [25; 32],
        remaining_spec_committed_instances: u32::MAX,
        root_committed_component_instances: 26,
        root_known_created_component_canisters: 27,
        root_registry_encoded_bytes: u64::MAX,
        removed_at_ns: 28,
        removal_hash: [29; 32],
    };
    let draining = RootComponentDrainingResponse {
        operation_id: [30; 32],
        component: registry().component,
        previous_registry: registry(),
        registry: registry(),
        descendant_count: 31,
        descendant_content_hash: [32; 32],
        directory_authority_hash: [33; 32],
        started_at_ns: 34,
    };
    for phase in [
        None,
        Some(Phase::DeleteIntent(Box::new(intent()))),
        Some(Phase::Deleted(Box::new(deleted))),
        Some(Phase::MembershipRemoved(Box::new(removed))),
    ] {
        let status = Status {
            draining: draining.clone(),
            deletion: phase.map(|phase| Deletion {
                operation_id: draining.operation_id,
                component: draining.component,
                phase,
            }),
        };
        let canonical = RootComponentRemovalOperationStatus::from(status.clone());
        let bytes = ic_wire::native::encode_response(METHOD_ID, &status, RESPONSE_LIMITS).unwrap();
        let decoded: Status =
            ic_wire::native::decode_response(METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
        assert_eq!(decoded, status);
        assert_eq!(
            RootComponentRemovalOperationStatus::from(decoded),
            canonical
        );
        assert_eq!(Status::from(canonical), status);
    }
}

#[test]
fn component_removal_query_has_an_exact_frame() {
    let operation = [35_u8; 32];
    let bytes = ic_wire::native::encode_request(METHOD_ID, &operation, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<[u8; 32]>(METHOD_ID, &bytes, REQUEST_LIMITS).unwrap(),
        operation
    );
    assert!(matches!(
        ic_wire::native::decode_request::<[u8; 32]>(
            crate::protocol::subtree_read::METHOD_ID,
            &bytes,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn component_removal_schema_is_complete() {
    ic_wire::typescript::native_schema::<Result<Status, crate::dto::error::Error>>().unwrap();
}
