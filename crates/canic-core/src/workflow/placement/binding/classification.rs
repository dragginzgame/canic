//! Module: workflow::placement::binding::classification
//!
//! Responsibility: classify binding entries for resolve and recovery flows.
//! Does not own: storage mutation, child creation, or recovery side effects.
//! Boundary: maps registry state into workflow-only classification outcomes.

use crate::{
    config::schema::BindingPool,
    ops::{
        runtime::metrics::{
            placement_binding::{
                PlacementBindingMetricOperation as MetricOperation,
                PlacementBindingMetricReason as MetricReason,
            },
            recording::PlacementBindingMetricEvent as MetricEvent,
        },
        storage::placement::binding::{PlacementBindingEntryState, PlacementBindingRegistryOps},
    },
    workflow::placement::binding::{
        PlacementBindingWorkflow,
        state::{
            PlacementBindingEntryClassification, pending_is_stale, validate_bind_target_with_reason,
        },
    },
};

impl PlacementBindingWorkflow {
    // Classify the current entry once so resolve and recovery follow the same stale/repair rules.
    pub(super) fn classify_entry(
        pool: &str,
        key_value: &str,
        pool_cfg: &BindingPool,
        now: u64,
    ) -> Option<PlacementBindingEntryClassification> {
        let Some(state) = PlacementBindingRegistryOps::lookup_state(pool, key_value) else {
            MetricEvent::completed(MetricOperation::Classify, MetricReason::Missing);
            return None;
        };

        let classification = match state {
            PlacementBindingEntryState::Bound {
                instance_pid,
                bound_at,
            } => PlacementBindingEntryClassification::Bound {
                instance_pid,
                bound_at,
            },

            PlacementBindingEntryState::Pending {
                claim_id,
                owner_pid,
                created_at,
                provisional_pid,
            } if !pending_is_stale(now, created_at) => {
                PlacementBindingEntryClassification::PendingFresh {
                    claim_id,
                    owner_pid,
                    created_at,
                    provisional_pid,
                }
            }

            PlacementBindingEntryState::Pending {
                claim_id,
                owner_pid,
                created_at,
                provisional_pid: None,
            } => PlacementBindingEntryClassification::Resumable {
                claim_id,
                owner_pid,
                created_at,
            },

            PlacementBindingEntryState::Pending {
                claim_id,
                owner_pid,
                provisional_pid: Some(pid),
                ..
            } if validate_bind_target_with_reason(pid, &pool_cfg.canister_role).is_ok() => {
                PlacementBindingEntryClassification::Repairable {
                    claim_id,
                    owner_pid,
                    provisional_pid: pid,
                }
            }

            PlacementBindingEntryState::Pending {
                claim_id,
                owner_pid,
                provisional_pid: Some(provisional_pid),
                ..
            } => PlacementBindingEntryClassification::NeedsCleanup {
                claim_id,
                owner_pid,
                provisional_pid,
            },
        };

        MetricEvent::completed(
            MetricOperation::Classify,
            Self::classification_reason(&classification),
        );
        Some(classification)
    }

    // Map an internal binding entry classification to the public metric reason vocabulary.
    const fn classification_reason(
        classification: &PlacementBindingEntryClassification,
    ) -> MetricReason {
        match classification {
            PlacementBindingEntryClassification::Bound { .. } => MetricReason::AlreadyBound,
            PlacementBindingEntryClassification::PendingFresh { .. } => MetricReason::PendingFresh,
            PlacementBindingEntryClassification::Repairable { .. } => MetricReason::StaleRepairable,
            PlacementBindingEntryClassification::Resumable { .. } => MetricReason::ResumedPending,
            PlacementBindingEntryClassification::NeedsCleanup { .. } => MetricReason::StaleCleanup,
        }
    }
}
