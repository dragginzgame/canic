use super::DirectoryWorkflow;
use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    dto::placement::directory::{DirectoryEntryStatusResponse, DirectoryRecoveryResponse},
    ops::{
        ic::IcOps,
        rpc::request::RequestOps,
        runtime::metrics::{
            directory::{
                DirectoryMetricOperation as MetricOperation, DirectoryMetricReason as MetricReason,
            },
            recording::DirectoryMetricEvent as MetricEvent,
        },
        storage::{
            placement::directory::{DirectoryRegistryOps, DirectoryReleaseResult},
            registry::subnet::SubnetRegistryOps,
        },
    },
};

impl DirectoryWorkflow {
    // Recycle any abandoned provisional child and release the stale claim so one caller can
    // re-claim the key in the same user-driven flow without background timers.
    pub(super) async fn cleanup_stale_entry(
        pool: &str,
        key_value: &str,
        claim_id: u64,
        provisional_pid: Option<Principal>,
    ) -> Result<(), InternalError> {
        MetricEvent::started(MetricOperation::CleanupStale);
        if let Some(pid) = provisional_pid
            && let Err(err) = Self::recycle_abandoned_child(pid).await
        {
            MetricEvent::failed(MetricOperation::CleanupStale, &err);
            return Err(err);
        }

        if let Err(err) = DirectoryRegistryOps::release_stale_pending_if_claim_matches(
            pool,
            key_value,
            claim_id,
            IcOps::now_secs(),
        ) {
            MetricEvent::failed(MetricOperation::CleanupStale, &err);
            return Err(err);
        }
        MetricEvent::completed(MetricOperation::CleanupStale, MetricReason::ReleasedStale);
        Ok(())
    }

    // Delegate orphan disposition to the root pool lifecycle instead of encoding pool logic here.
    pub(super) async fn recycle_abandoned_child(pid: Principal) -> Result<(), InternalError> {
        if !SubnetRegistryOps::is_registered(pid) {
            MetricEvent::skipped(
                MetricOperation::RecycleAbandoned,
                MetricReason::RegistryMissing,
            );
            return Ok(());
        }

        MetricEvent::started(MetricOperation::RecycleAbandoned);
        if let Err(err) = RequestOps::recycle_canister(pid).await {
            MetricEvent::failed(MetricOperation::RecycleAbandoned, &err);
            return Err(err);
        }
        MetricEvent::completed(MetricOperation::RecycleAbandoned, MetricReason::Ok);
        Ok(())
    }

    // Release one stale claim after recycling any abandoned child and map the result for
    // explicit recovery callers. If ownership changed during cleanup, the caller should retry.
    pub(super) async fn recover_cleanup_stale_entry(
        pool: &str,
        key_value: &str,
        claim_id: u64,
        provisional_pid: Option<Principal>,
    ) -> Result<Option<DirectoryRecoveryResponse>, InternalError> {
        MetricEvent::started(MetricOperation::CleanupStale);
        if let Some(pid) = provisional_pid
            && let Err(err) = Self::recycle_abandoned_child(pid).await
        {
            MetricEvent::failed(MetricOperation::CleanupStale, &err);
            return Err(err);
        }

        let now = IcOps::now_secs();
        match DirectoryRegistryOps::release_stale_pending_if_claim_matches(
            pool, key_value, claim_id, now,
        ) {
            Ok(DirectoryReleaseResult::ReleasedStalePending {
                owner_pid,
                created_at,
                provisional_pid,
            }) => {
                MetricEvent::completed(MetricOperation::CleanupStale, MetricReason::ReleasedStale);
                Ok(Some(DirectoryRecoveryResponse::ReleasedStalePending {
                    owner_pid,
                    created_at,
                    provisional_pid,
                    released_at: now,
                }))
            }
            Ok(DirectoryReleaseResult::Missing) => {
                MetricEvent::skipped(MetricOperation::CleanupStale, MetricReason::Missing);
                Ok(Some(DirectoryRecoveryResponse::Missing))
            }
            Ok(DirectoryReleaseResult::Bound {
                instance_pid,
                bound_at,
            }) => {
                MetricEvent::skipped(MetricOperation::CleanupStale, MetricReason::AlreadyBound);
                Ok(Some(DirectoryRecoveryResponse::Bound {
                    instance_pid,
                    bound_at,
                }))
            }
            Ok(DirectoryReleaseResult::PendingCurrent { .. }) => {
                MetricEvent::skipped(MetricOperation::CleanupStale, MetricReason::PendingCurrent);
                Ok(None)
            }
            Err(err) => {
                MetricEvent::failed(MetricOperation::CleanupStale, &err);
                Err(err)
            }
        }
    }

    // Repair a stale valid provisional child only if its original claim is still current.
    pub(super) fn repair_stale_entry(
        pool: &str,
        key_value: &str,
        claim_id: u64,
        provisional_pid: Principal,
        now: u64,
    ) -> Result<DirectoryEntryStatusResponse, InternalError> {
        MetricEvent::started(MetricOperation::RepairStale);
        let repaired = match DirectoryRegistryOps::bind_if_claim_matches(
            pool,
            key_value,
            claim_id,
            provisional_pid,
            now,
        ) {
            Ok(repaired) => repaired,
            Err(err) => {
                MetricEvent::failed(MetricOperation::RepairStale, &err);
                return Err(err);
            }
        };
        if !repaired {
            MetricEvent::failed_reason(MetricOperation::RepairStale, MetricReason::ClaimLost);
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "directory claim lost during stale repair without an await boundary",
            ));
        }

        MetricEvent::completed(MetricOperation::RepairStale, MetricReason::Ok);
        Ok(DirectoryEntryStatusResponse::Bound {
            instance_pid: provisional_pid,
            bound_at: now,
        })
    }
}
