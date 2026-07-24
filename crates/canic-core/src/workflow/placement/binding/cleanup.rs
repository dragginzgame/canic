//! Module: workflow::placement::binding::cleanup
//!
//! Responsibility: recycle abandoned binding children and release stale claims.
//! Does not own: pool lifecycle rules, registry schemas, or recovery endpoint mapping.
//! Boundary: delegates orphan disposal and performs claim-matching cleanup writes.

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    config::schema::BindingPool,
    dto::placement::binding::{PlacementBindingRecoveryResponse, PlacementBindingStatusResponse},
    ops::{
        ic::IcOps,
        rpc::request::RequestOps,
        runtime::metrics::{
            placement_binding::{
                PlacementBindingMetricOperation as MetricOperation,
                PlacementBindingMetricReason as MetricReason,
            },
            recording::PlacementBindingMetricEvent as MetricEvent,
        },
        storage::{
            placement::binding::{
                PlacementBindingPendingClaim, PlacementBindingRegistryOps,
                PlacementBindingReleaseResult,
            },
            registry::subnet::SubnetRegistryOps,
        },
    },
    workflow::placement::{
        allocation::PlacementAllocationWorkflow,
        binding::{PlacementBindingWorkflow, create::placement_binding_allocation_request},
    },
};

impl PlacementBindingWorkflow {
    // Recycle any abandoned provisional child and release the stale claim so one caller can
    // re-claim the key in the same user-driven flow without background timers.
    pub(super) async fn cleanup_stale_entry(
        pool: &str,
        key_value: &str,
        pool_cfg: &BindingPool,
        claim_id: u64,
        owner_pid: Principal,
        provisional_pid: Principal,
    ) -> Result<(), InternalError> {
        MetricEvent::started(MetricOperation::CleanupStale);
        let claim = PlacementBindingPendingClaim {
            claim_id,
            owner_pid,
            created_at: 0,
        };
        let request = placement_binding_allocation_request(pool, key_value, pool_cfg, claim);
        let permit = PlacementAllocationWorkflow::resume_permit(&request)?;
        if let Err(err) = Self::recycle_abandoned_child(provisional_pid).await {
            MetricEvent::failed(MetricOperation::CleanupStale, &err);
            return Err(err);
        }

        if let Err(err) = PlacementBindingRegistryOps::release_stale_pending_if_claim_matches(
            pool,
            key_value,
            claim_id,
            IcOps::now_secs(),
        ) {
            MetricEvent::failed(MetricOperation::CleanupStale, &err);
            return Err(err);
        }
        PlacementAllocationWorkflow::finish_disposed_child(&permit, provisional_pid)?;
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
        pool_cfg: &BindingPool,
        claim_id: u64,
        owner_pid: Principal,
        provisional_pid: Principal,
    ) -> Result<Option<PlacementBindingRecoveryResponse>, InternalError> {
        MetricEvent::started(MetricOperation::CleanupStale);
        let claim = PlacementBindingPendingClaim {
            claim_id,
            owner_pid,
            created_at: 0,
        };
        let request = placement_binding_allocation_request(pool, key_value, pool_cfg, claim);
        let permit = PlacementAllocationWorkflow::resume_permit(&request)?;
        if let Err(err) = Self::recycle_abandoned_child(provisional_pid).await {
            MetricEvent::failed(MetricOperation::CleanupStale, &err);
            return Err(err);
        }

        let now = IcOps::now_secs();
        let result = PlacementBindingRegistryOps::release_stale_pending_if_claim_matches(
            pool, key_value, claim_id, now,
        );
        let result = match result {
            Ok(result) => result,
            Err(err) => {
                MetricEvent::failed(MetricOperation::CleanupStale, &err);
                return Err(err);
            }
        };
        PlacementAllocationWorkflow::finish_disposed_child(&permit, provisional_pid)?;
        match result {
            PlacementBindingReleaseResult::ReleasedStalePending {
                owner_pid,
                created_at,
                provisional_pid,
            } => {
                MetricEvent::completed(MetricOperation::CleanupStale, MetricReason::ReleasedStale);
                Ok(Some(
                    PlacementBindingRecoveryResponse::ReleasedStalePending {
                        owner_pid,
                        created_at,
                        provisional_pid,
                        released_at: now,
                    },
                ))
            }
            PlacementBindingReleaseResult::Missing => {
                MetricEvent::skipped(MetricOperation::CleanupStale, MetricReason::Missing);
                Ok(Some(PlacementBindingRecoveryResponse::Missing))
            }
            PlacementBindingReleaseResult::Bound {
                instance_pid,
                bound_at,
            } => {
                MetricEvent::skipped(MetricOperation::CleanupStale, MetricReason::AlreadyBound);
                Ok(Some(PlacementBindingRecoveryResponse::Bound {
                    instance_pid,
                    bound_at,
                }))
            }
            PlacementBindingReleaseResult::PendingRetained { .. } => {
                MetricEvent::skipped(MetricOperation::CleanupStale, MetricReason::PendingCurrent);
                Ok(None)
            }
        }
    }

    // Repair a stale valid provisional child only if its original claim is still current.
    pub(super) fn repair_stale_entry(
        pool: &str,
        key_value: &str,
        pool_cfg: &BindingPool,
        claim_id: u64,
        owner_pid: Principal,
        provisional_pid: Principal,
        now: u64,
    ) -> Result<PlacementBindingStatusResponse, InternalError> {
        MetricEvent::started(MetricOperation::RepairStale);
        let claim = PlacementBindingPendingClaim {
            claim_id,
            owner_pid,
            created_at: 0,
        };
        let request = placement_binding_allocation_request(pool, key_value, pool_cfg, claim);
        let permit = PlacementAllocationWorkflow::resume_permit(&request)?;
        let repaired = match PlacementBindingRegistryOps::bind_if_claim_matches(
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
                "binding claim lost during stale repair without an await boundary",
            ));
        }
        PlacementAllocationWorkflow::finish_registered_child(&permit, provisional_pid)?;

        MetricEvent::completed(MetricOperation::RepairStale, MetricReason::Ok);
        Ok(PlacementBindingStatusResponse::Bound {
            instance_pid: provisional_pid,
            bound_at: now,
        })
    }
}
