//! Module: workflow::placement::index::classification
//!
//! Responsibility: classify index entries for resolve and recovery flows.
//! Does not own: storage mutation, child creation, or recovery side effects.
//! Boundary: maps registry state into workflow-only classification outcomes.

use crate::{
    config::schema::IndexPool,
    ops::{
        runtime::metrics::{
            placement_index::{
                PlacementIndexMetricOperation as MetricOperation,
                PlacementIndexMetricReason as MetricReason,
            },
            recording::PlacementIndexMetricEvent as MetricEvent,
        },
        storage::placement::index::{PlacementIndexEntryState, PlacementIndexRegistryOps},
    },
    workflow::placement::index::{
        PlacementIndexWorkflow,
        state::{
            PlacementIndexEntryClassification, pending_is_stale, validate_bind_target_with_reason,
        },
    },
};

impl PlacementIndexWorkflow {
    // Classify the current entry once so resolve and recovery follow the same stale/repair rules.
    pub(super) fn classify_entry(
        pool: &str,
        key_value: &str,
        pool_cfg: &IndexPool,
        now: u64,
    ) -> Option<PlacementIndexEntryClassification> {
        let Some(state) = PlacementIndexRegistryOps::lookup_state(pool, key_value) else {
            MetricEvent::completed(MetricOperation::Classify, MetricReason::Missing);
            return None;
        };

        let classification = match state {
            PlacementIndexEntryState::Bound {
                instance_pid,
                bound_at,
            } => PlacementIndexEntryClassification::Bound {
                instance_pid,
                bound_at,
            },

            PlacementIndexEntryState::Pending {
                claim_id,
                owner_pid,
                created_at,
                provisional_pid,
            } if !pending_is_stale(now, created_at) => {
                PlacementIndexEntryClassification::PendingFresh {
                    claim_id,
                    owner_pid,
                    created_at,
                    provisional_pid,
                }
            }

            PlacementIndexEntryState::Pending {
                claim_id,
                owner_pid,
                created_at,
                provisional_pid: None,
            } => PlacementIndexEntryClassification::Resumable {
                claim_id,
                owner_pid,
                created_at,
            },

            PlacementIndexEntryState::Pending {
                claim_id,
                owner_pid,
                provisional_pid: Some(pid),
                ..
            } if validate_bind_target_with_reason(pid, &pool_cfg.canister_role).is_ok() => {
                PlacementIndexEntryClassification::Repairable {
                    claim_id,
                    owner_pid,
                    provisional_pid: pid,
                }
            }

            PlacementIndexEntryState::Pending {
                claim_id,
                owner_pid,
                provisional_pid: Some(provisional_pid),
                ..
            } => PlacementIndexEntryClassification::NeedsCleanup {
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

    // Map an internal index entry classification to the public metric reason vocabulary.
    const fn classification_reason(
        classification: &PlacementIndexEntryClassification,
    ) -> MetricReason {
        match classification {
            PlacementIndexEntryClassification::Bound { .. } => MetricReason::AlreadyBound,
            PlacementIndexEntryClassification::PendingFresh { .. } => MetricReason::PendingFresh,
            PlacementIndexEntryClassification::Repairable { .. } => MetricReason::StaleRepairable,
            PlacementIndexEntryClassification::Resumable { .. } => MetricReason::ResumedPending,
            PlacementIndexEntryClassification::NeedsCleanup { .. } => MetricReason::StaleCleanup,
        }
    }
}
