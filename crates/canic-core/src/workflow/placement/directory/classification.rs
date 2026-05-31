use super::state::{pending_is_stale, validate_bind_target_with_reason};
use super::{DirectoryWorkflow, state::DirectoryEntryClassification};
use crate::{
    config::schema::DirectoryPool,
    ops::{
        runtime::metrics::{
            directory::{
                DirectoryMetricOperation as MetricOperation, DirectoryMetricReason as MetricReason,
            },
            recording::DirectoryMetricEvent as MetricEvent,
        },
        storage::placement::directory::{DirectoryEntryState, DirectoryRegistryOps},
    },
};

impl DirectoryWorkflow {
    // Classify the current entry once so resolve and recovery follow the same stale/repair rules.
    pub(super) fn classify_entry(
        pool: &str,
        key_value: &str,
        pool_cfg: &DirectoryPool,
        now: u64,
    ) -> Option<DirectoryEntryClassification> {
        let Some(state) = DirectoryRegistryOps::lookup_state(pool, key_value) else {
            MetricEvent::completed(MetricOperation::Classify, MetricReason::Missing);
            return None;
        };

        let classification = match state {
            DirectoryEntryState::Bound {
                instance_pid,
                bound_at,
            } => DirectoryEntryClassification::Bound {
                instance_pid,
                bound_at,
            },

            DirectoryEntryState::Pending {
                claim_id: _,
                owner_pid,
                created_at,
                provisional_pid,
            } if !pending_is_stale(now, created_at) => DirectoryEntryClassification::PendingFresh {
                owner_pid,
                created_at,
                provisional_pid,
            },

            DirectoryEntryState::Pending {
                claim_id,
                provisional_pid: Some(pid),
                ..
            } if validate_bind_target_with_reason(pid, &pool_cfg.canister_role).is_ok() => {
                DirectoryEntryClassification::Repairable {
                    claim_id,
                    provisional_pid: pid,
                }
            }

            DirectoryEntryState::Pending {
                claim_id,
                provisional_pid,
                ..
            } => DirectoryEntryClassification::NeedsCleanup {
                claim_id,
                provisional_pid,
            },
        };

        MetricEvent::completed(
            MetricOperation::Classify,
            Self::classification_reason(&classification),
        );
        Some(classification)
    }

    // Map an internal directory entry classification to the public metric reason vocabulary.
    const fn classification_reason(classification: &DirectoryEntryClassification) -> MetricReason {
        match classification {
            DirectoryEntryClassification::Bound { .. } => MetricReason::AlreadyBound,
            DirectoryEntryClassification::PendingFresh { .. } => MetricReason::PendingFresh,
            DirectoryEntryClassification::Repairable { .. } => MetricReason::StaleRepairable,
            DirectoryEntryClassification::NeedsCleanup { .. } => MetricReason::StaleCleanup,
        }
    }
}
