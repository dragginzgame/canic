//! Root-facing Component and subtree retirement response projection.
//!
//! Boundary: retained Registry views are projected into passive response DTOs without reading
//! storage, choosing lifecycle progress or issuing an effect.

use super::*;

pub(super) const fn component_draining_response(
    draining: RootComponentDrainingView,
) -> RootComponentDrainingResponse {
    RootComponentDrainingResponse {
        operation_id: draining.operation_id,
        component: draining.component,
        previous_registry: draining.previous_registry,
        registry: draining.registry,
        descendant_count: draining.descendant_count,
        descendant_content_hash: draining.descendant_content_hash,
        directory_authority_hash: draining.directory_authority_hash,
        started_at_ns: draining.started_at_ns,
    }
}

pub(super) const fn component_final_inventory_response(
    operation_id: [u8; 32],
    component: ComponentInstanceId,
    inventory: RootComponentFinalInventoryView,
) -> RootComponentFinalInventoryResponse {
    RootComponentFinalInventoryResponse {
        operation_id,
        component,
        inventory: component_final_inventory(inventory),
    }
}

pub(super) fn component_deletion_response(
    draining: RootComponentDrainingView,
) -> Result<RootComponentDeletionResponse, InternalError> {
    let progress = draining.deletion.ok_or_else(InternalError::unavailable)?;
    let phase = match progress {
        RootComponentDeletionProgressView::DeleteIntent(intent) => {
            RootComponentDeletionPhase::DeleteIntent(component_deletion_intent(intent))
        }
        RootComponentDeletionProgressView::Deleted(receipt) => {
            RootComponentDeletionPhase::Deleted(RootComponentDeletedReceipt {
                deletion: component_deletion_intent(receipt.deletion),
                deleted_at_ns: receipt.deleted_at_ns,
            })
        }
        RootComponentDeletionProgressView::MembershipRemoved(receipt) => {
            RootComponentDeletionPhase::MembershipRemoved(component_membership_removed_receipt(
                receipt,
            ))
        }
    };
    Ok(RootComponentDeletionResponse {
        operation_id: draining.operation_id,
        component: draining.component,
        phase,
    })
}

const fn component_membership_removed_receipt(
    receipt: RootComponentMembershipRemovedView,
) -> RootComponentMembershipRemovedReceipt {
    RootComponentMembershipRemovedReceipt {
        deleted: RootComponentDeletedReceipt {
            deletion: component_deletion_intent(receipt.deleted.deletion),
            deleted_at_ns: receipt.deleted.deleted_at_ns,
        },
        allocation_operation_id: receipt.allocation_operation_id,
        remaining_spec_committed_instances: receipt.remaining_spec_committed_instances,
        root_committed_component_instances: receipt.root_committed_component_instances,
        root_known_created_component_canisters: receipt.root_known_created_component_canisters,
        root_registry_encoded_bytes: receipt.root_registry_encoded_bytes,
        removed_at_ns: receipt.removed_at_ns,
        removal_hash: receipt.removal_hash,
    }
}

const fn component_deletion_intent(
    intent: RootComponentDeletionIntentView,
) -> RootComponentDeletionIntent {
    RootComponentDeletionIntent {
        final_inventory: component_final_inventory(intent.final_inventory),
        quiescence: RootComponentQuiescentReceipt {
            stop: component_quiescence_stop_intent(intent.quiescence.stop),
            observed_module_hash: intent.quiescence.observed_module_hash,
            quiesced_at_ns: intent.quiescence.quiesced_at_ns,
        },
        prepared_at_ns: intent.prepared_at_ns,
    }
}

const fn component_final_inventory(
    inventory: RootComponentFinalInventoryView,
) -> RootComponentFinalInventory {
    RootComponentFinalInventory {
        registry: inventory.registry,
        descendant_content_hash: inventory.descendant_content_hash,
        registry_encoded_bytes: inventory.registry_encoded_bytes,
        directory_synchronized_at_ns: inventory.directory_synchronized_at_ns,
        covered_fleet_registry_revision: inventory.covered_fleet_registry_revision,
        covered_fleet_registry_content_hash: inventory.covered_fleet_registry_content_hash,
        directory_authority_hash: inventory.directory_authority_hash,
        inventory_hash: inventory.inventory_hash,
        finalized_at_ns: inventory.finalized_at_ns,
    }
}

pub(super) fn component_quiescence_response(
    draining: RootComponentDrainingView,
) -> Result<RootComponentQuiescenceResponse, InternalError> {
    let phase = draining.quiescence.ok_or_else(InternalError::unavailable)?;
    Ok(RootComponentQuiescenceResponse {
        operation_id: draining.operation_id,
        component: draining.component,
        phase: match phase {
            RootComponentQuiescenceProgressView::StopIntent(intent) => {
                RootComponentQuiescencePhase::StopIntent(component_quiescence_stop_intent(intent))
            }
            RootComponentQuiescenceProgressView::Quiescent(receipt) => {
                RootComponentQuiescencePhase::Quiescent(RootComponentQuiescentReceipt {
                    stop: component_quiescence_stop_intent(receipt.stop),
                    observed_module_hash: receipt.observed_module_hash,
                    quiesced_at_ns: receipt.quiesced_at_ns,
                })
            }
        },
    })
}

const fn component_quiescence_stop_intent(
    intent: RootComponentQuiescenceStopIntentView,
) -> RootComponentQuiescenceStopIntent {
    RootComponentQuiescenceStopIntent {
        registry: intent.registry,
        descendant_count: intent.descendant_count,
        descendant_content_hash: intent.descendant_content_hash,
        canister_id: intent.canister_id,
        controller: intent.controller,
        expected_module_hash: intent.expected_module_hash,
        covered_fleet_registry_revision: intent.covered_fleet_registry_revision,
        covered_fleet_registry_content_hash: intent.covered_fleet_registry_content_hash,
        covered_authority_hash: intent.covered_authority_hash,
        runtime_operation_id: intent.runtime_operation_id,
        activation: intent.activation,
        prepared_at_ns: intent.prepared_at_ns,
    }
}

pub(super) fn subtree_removal_response(
    removal: RootComponentSubtreeRemovalView,
) -> RootComponentSubtreeRemovalResponse {
    RootComponentSubtreeRemovalResponse {
        operation_id: removal.operation_id,
        component: removal.component,
        target_canister_id: removal.target_canister_id,
        target_parent_canister_id: removal.target_parent_canister_id,
        target_role: removal.target_role,
        target_status: removal.target_status,
        reserved_against_registry: removal.reserved_against_registry,
        maximum_completed_leaves: removal.maximum_completed_leaves,
        completed_leaves: removal.completed_leaves,
        traversal_steps: removal.traversal_steps,
        phase: match removal.progress {
            RootComponentSubtreeRemovalProgressView::Fenced => {
                RootComponentSubtreeRemovalPhase::Fenced
            }
            RootComponentSubtreeRemovalProgressView::Traversing { cursor } => {
                RootComponentSubtreeRemovalPhase::Traversing(RootComponentSubtreeRemovalNode {
                    canister_id: cursor.canister_id,
                    parent_canister_id: cursor.parent_canister_id,
                    role: cursor.role,
                    kind: cursor.kind,
                    installed_artifact_hash: cursor.installed_artifact_hash,
                    status: cursor.status,
                })
            }
            RootComponentSubtreeRemovalProgressView::LeafSelected { leaf } => {
                RootComponentSubtreeRemovalPhase::LeafSelected(RootComponentSubtreeRemovalNode {
                    canister_id: leaf.canister_id,
                    parent_canister_id: leaf.parent_canister_id,
                    role: leaf.role,
                    kind: leaf.kind,
                    installed_artifact_hash: leaf.installed_artifact_hash,
                    status: leaf.status,
                })
            }
            RootComponentSubtreeRemovalProgressView::StopIntent(effect) => {
                RootComponentSubtreeRemovalPhase::StopIntent(subtree_stop_intent_response(effect))
            }
            RootComponentSubtreeRemovalProgressView::Stopped(receipt) => {
                RootComponentSubtreeRemovalPhase::Stopped(subtree_stopped_receipt_response(receipt))
            }
            RootComponentSubtreeRemovalProgressView::DeleteIntent(deletion) => {
                RootComponentSubtreeRemovalPhase::DeleteIntent(
                    RootComponentSubtreeRemovalDeleteIntent {
                        stopped: subtree_stopped_receipt_response(deletion.stopped),
                    },
                )
            }
            RootComponentSubtreeRemovalProgressView::Deleted(receipt) => {
                RootComponentSubtreeRemovalPhase::Deleted(
                    RootComponentSubtreeRemovalDeletedReceipt {
                        deletion: RootComponentSubtreeRemovalDeleteIntent {
                            stopped: subtree_stopped_receipt_response(receipt.deletion.stopped),
                        },
                    },
                )
            }
            RootComponentSubtreeRemovalProgressView::MembershipRemoved(receipt) => {
                RootComponentSubtreeRemovalPhase::MembershipRemoved(
                    subtree_membership_removed_receipt_response(receipt),
                )
            }
            RootComponentSubtreeRemovalProgressView::DirectorySynchronized(receipt) => {
                RootComponentSubtreeRemovalPhase::DirectorySynchronized(
                    subtree_directory_synchronized_receipt_response(receipt),
                )
            }
            RootComponentSubtreeRemovalProgressView::Completed(completed) => {
                RootComponentSubtreeRemovalPhase::Completed(
                    RootComponentSubtreeRemovalCompletedReceipt {
                        registry: completed.registry,
                        directory_authority_hash: completed.directory_authority_hash,
                    },
                )
            }
        },
    }
}

fn subtree_directory_synchronized_receipt_response(
    receipt: RootComponentSubtreeDirectorySynchronizedView,
) -> RootComponentSubtreeRemovalDirectorySynchronizedReceipt {
    RootComponentSubtreeRemovalDirectorySynchronizedReceipt {
        membership_removed: subtree_membership_removed_receipt_response(receipt.membership_removed),
        covered_fleet_registry_revision: receipt.covered_fleet_registry_revision,
        covered_fleet_registry_content_hash: receipt.covered_fleet_registry_content_hash,
        covered_component_registry: receipt.covered_component_registry,
        covered_authority_hash: receipt.covered_authority_hash,
        owning_component: receipt
            .owning_component
            .map(subtree_directory_convergence_evidence_response),
        parent: receipt
            .parent
            .map(subtree_directory_convergence_evidence_response),
    }
}

const fn subtree_directory_convergence_evidence_response(
    evidence: RootComponentSubtreeDirectoryConvergenceView,
) -> RootComponentSubtreeRemovalDirectoryConvergenceEvidence {
    RootComponentSubtreeRemovalDirectoryConvergenceEvidence {
        operation_id: evidence.operation_id,
        canister_id: evidence.canister_id,
        activation: evidence.activation,
    }
}

fn subtree_membership_removed_receipt_response(
    receipt: RootComponentSubtreeMembershipRemovedView,
) -> RootComponentSubtreeRemovalMembershipRemovedReceipt {
    RootComponentSubtreeRemovalMembershipRemovedReceipt {
        deleted: RootComponentSubtreeRemovalDeletedReceipt {
            deletion: RootComponentSubtreeRemovalDeleteIntent {
                stopped: subtree_stopped_receipt_response(receipt.deleted.deletion.stopped),
            },
        },
        removed_from_registry: receipt.removed_from_registry,
        previous_descendant_content_hash: receipt.previous_descendant_content_hash,
        previous_committed_descendants: receipt.previous_committed_descendants,
        registry: receipt.registry,
        descendant_content_hash: receipt.descendant_content_hash,
        registry_encoded_bytes: receipt.registry_encoded_bytes,
        reserved_descendants: receipt.reserved_descendants,
        committed_descendants: receipt.committed_descendants,
        directory_synchronized_at_ns: receipt.directory_synchronized_at_ns,
        directory_authority_hash: receipt.directory_authority_hash,
        parent_role_instances: receipt.parent_role_instances,
        root_managed_descendants: receipt.root_managed_descendants,
        root_known_created_component_canisters: receipt.root_known_created_component_canisters,
    }
}

fn subtree_stop_intent_response(
    effect: RootComponentSubtreeStopEffectView,
) -> RootComponentSubtreeRemovalStopIntent {
    RootComponentSubtreeRemovalStopIntent {
        leaf: RootComponentSubtreeRemovalNode {
            canister_id: effect.leaf.canister_id,
            parent_canister_id: effect.leaf.parent_canister_id,
            role: effect.leaf.role,
            kind: effect.leaf.kind,
            installed_artifact_hash: effect.leaf.installed_artifact_hash,
            status: effect.leaf.status,
        },
        controller: effect.controller,
    }
}

fn subtree_stopped_receipt_response(
    receipt: RootComponentSubtreeStoppedEffectView,
) -> RootComponentSubtreeRemovalStoppedReceipt {
    RootComponentSubtreeRemovalStoppedReceipt {
        stop: subtree_stop_intent_response(receipt.stop),
        observed_module_hash: receipt.observed_module_hash,
    }
}
