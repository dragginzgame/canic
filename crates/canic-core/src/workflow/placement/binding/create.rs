//! Module: workflow::placement::binding::create
//!
//! Responsibility: claim keys, create child instances, and bind successful claims.
//! Does not own: registry schemas, canister request execution, or stale cleanup policy.
//! Boundary: performs claim-matching writes around asynchronous child creation.

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    config::schema::BindingPool,
    dto::placement::binding::PlacementBindingStatusResponse,
    model::placement::allocation::PlacementAllocationIdentity,
    ops::{
        ic::IcOps,
        runtime::metrics::{
            placement_binding::{
                PlacementBindingMetricOperation as MetricOperation,
                PlacementBindingMetricReason as MetricReason,
            },
            recording::PlacementBindingMetricEvent as MetricEvent,
        },
        storage::placement::binding::{
            PlacementBindingClaimResult, PlacementBindingPendingClaim, PlacementBindingRegistryOps,
        },
    },
    workflow::placement::{
        allocation::{
            PlacementAllocationPermit, PlacementAllocationRequest, PlacementAllocationWorkflow,
        },
        binding::{PlacementBindingWorkflow, state::new_claim_id},
    },
};

impl PlacementBindingWorkflow {
    // Finalize one freshly created child using claim-matching writes so late async completions
    // cannot overwrite a newer claim after the key has been reclaimed.
    pub(super) async fn finalize_created_instance(
        pool: &str,
        key_value: &str,
        claim: PlacementBindingPendingClaim,
        pid: Principal,
        permit: &PlacementAllocationPermit,
    ) -> Result<Option<PlacementBindingStatusResponse>, InternalError> {
        MetricEvent::started(MetricOperation::Finalize);
        if !PlacementBindingRegistryOps::set_provisional_pid_if_claim_matches(
            pool,
            key_value,
            claim.claim_id,
            pid,
        )? {
            Self::recycle_abandoned_child(pid).await?;
            PlacementAllocationWorkflow::finish_disposed_child(permit, pid)?;
            MetricEvent::skipped(MetricOperation::Finalize, MetricReason::ClaimLost);
            return Ok(None);
        }

        let bound_at = IcOps::now_secs();
        let bound = match PlacementBindingRegistryOps::bind_if_claim_matches(
            pool,
            key_value,
            claim.claim_id,
            pid,
            bound_at,
        ) {
            Ok(bound) => bound,
            Err(err) => {
                MetricEvent::failed(MetricOperation::Finalize, &err);
                return Err(err);
            }
        };
        if !bound {
            MetricEvent::failed_reason(MetricOperation::Finalize, MetricReason::ClaimLost);
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "binding claim lost between provisional attach and final bind",
            ));
        }
        PlacementAllocationWorkflow::finish_registered_child(permit, pid)?;

        MetricEvent::completed(MetricOperation::Finalize, MetricReason::Ok);
        Ok(Some(PlacementBindingStatusResponse::Bound {
            instance_pid: pid,
            bound_at,
        }))
    }

    // Claim one logical key and, if this caller wins the claim, create and bind a new child.
    pub(super) async fn claim_and_create_instance(
        pool: &str,
        key_value: &str,
        pool_cfg: &BindingPool,
        owner_pid: Principal,
    ) -> Result<Option<PlacementBindingStatusResponse>, InternalError> {
        let now = IcOps::now_secs();
        let claim_id = new_claim_id();

        MetricEvent::started(MetricOperation::Claim);
        let claim_result = match PlacementBindingRegistryOps::claim_pending(
            pool, key_value, owner_pid, claim_id, now,
        ) {
            Ok(result) => result,
            Err(err) => {
                MetricEvent::failed(MetricOperation::Claim, &err);
                return Err(err);
            }
        };
        let claim = match claim_result {
            PlacementBindingClaimResult::Bound {
                instance_pid,
                bound_at,
            } => {
                MetricEvent::skipped(MetricOperation::Claim, MetricReason::AlreadyBound);
                return Ok(Some(PlacementBindingStatusResponse::Bound {
                    instance_pid,
                    bound_at,
                }));
            }
            PlacementBindingClaimResult::PendingExisting {
                claim_id: _,
                owner_pid,
                created_at,
                provisional_pid,
            } => {
                MetricEvent::skipped(MetricOperation::Claim, MetricReason::PendingFresh);
                return Ok(Some(PlacementBindingStatusResponse::Pending {
                    owner_pid,
                    created_at,
                    provisional_pid,
                }));
            }
            PlacementBindingClaimResult::Claimed(claim) => {
                MetricEvent::completed(MetricOperation::Claim, MetricReason::Claimed);
                claim
            }
        };

        Self::create_and_finalize_claim(pool, key_value, pool_cfg, claim).await
    }

    // Resume the exact durable create operation owned by an existing pending claim.
    pub(super) async fn resume_pending_instance(
        pool: &str,
        key_value: &str,
        pool_cfg: &BindingPool,
        claim: PlacementBindingPendingClaim,
    ) -> Result<Option<PlacementBindingStatusResponse>, InternalError> {
        let request = placement_binding_allocation_request(pool, key_value, pool_cfg, claim);

        MetricEvent::started(MetricOperation::CreateInstance);
        let (permit, pid) = match PlacementAllocationWorkflow::recover_child(request).await {
            Ok(result) => {
                MetricEvent::completed(MetricOperation::CreateInstance, MetricReason::Ok);
                result
            }
            Err(err) => {
                MetricEvent::failed(MetricOperation::CreateInstance, &err);
                return Err(err);
            }
        };

        Self::finalize_created_instance(pool, key_value, claim, pid, &permit).await
    }

    async fn create_and_finalize_claim(
        pool: &str,
        key_value: &str,
        pool_cfg: &BindingPool,
        claim: PlacementBindingPendingClaim,
    ) -> Result<Option<PlacementBindingStatusResponse>, InternalError> {
        let request = placement_binding_allocation_request(pool, key_value, pool_cfg, claim);

        MetricEvent::started(MetricOperation::CreateInstance);
        let (permit, pid) = match PlacementAllocationWorkflow::create_child(request).await {
            Ok(result) => {
                MetricEvent::completed(MetricOperation::CreateInstance, MetricReason::Ok);
                result
            }
            Err(err) => {
                MetricEvent::failed(MetricOperation::CreateInstance, &err);
                return Err(err);
            }
        };

        Self::finalize_created_instance(pool, key_value, claim, pid, &permit).await
    }
}

pub(super) fn placement_binding_allocation_request(
    pool: &str,
    key_value: &str,
    pool_cfg: &BindingPool,
    claim: PlacementBindingPendingClaim,
) -> PlacementAllocationRequest {
    let identity = PlacementAllocationIdentity::binding(
        claim.owner_pid,
        pool,
        key_value,
        claim.claim_id,
        &pool_cfg.canister_role,
        None,
    );
    let reservation_limit =
        PlacementAllocationWorkflow::reservation_limit_for_available_capacity(&identity, 1);

    PlacementAllocationRequest {
        identity,
        canister_role: pool_cfg.canister_role.clone(),
        extra_arg: None,
        reservation_limit,
    }
}
