//! Module: workflow::placement::directory::create
//!
//! Responsibility: claim keys, create child instances, and bind successful claims.
//! Does not own: registry schemas, canister request execution, or stale cleanup policy.
//! Boundary: performs claim-matching writes around asynchronous child creation.

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    config::schema::DirectoryPool,
    dto::placement::directory::DirectoryEntryStatusResponse,
    ops::{
        ic::IcOps,
        rpc::request::{CreateCanisterParent, RequestOps},
        runtime::metrics::{
            directory::{
                DirectoryMetricOperation as MetricOperation, DirectoryMetricReason as MetricReason,
            },
            recording::DirectoryMetricEvent as MetricEvent,
        },
        storage::placement::directory::{
            DirectoryClaimResult, DirectoryPendingClaim, DirectoryRegistryOps,
        },
    },
    workflow::placement::directory::{DirectoryWorkflow, state::new_claim_id},
};

impl DirectoryWorkflow {
    // Finalize one freshly created child using claim-matching writes so late async completions
    // cannot overwrite a newer claim after the key has been reclaimed.
    pub(super) async fn finalize_created_instance(
        pool: &str,
        key_value: &str,
        claim: DirectoryPendingClaim,
        pid: Principal,
    ) -> Result<Option<DirectoryEntryStatusResponse>, InternalError> {
        MetricEvent::started(MetricOperation::Finalize);
        if !DirectoryRegistryOps::set_provisional_pid_if_claim_matches(
            pool,
            key_value,
            claim.claim_id,
            pid,
        )? {
            Self::recycle_abandoned_child(pid).await?;
            MetricEvent::skipped(MetricOperation::Finalize, MetricReason::ClaimLost);
            return Ok(None);
        }

        let bound_at = IcOps::now_secs();
        let bound = match DirectoryRegistryOps::bind_if_claim_matches(
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
                "directory claim lost between provisional attach and final bind",
            ));
        }

        MetricEvent::completed(MetricOperation::Finalize, MetricReason::Ok);
        Ok(Some(DirectoryEntryStatusResponse::Bound {
            instance_pid: pid,
            bound_at,
        }))
    }

    // Claim one logical key and, if this caller wins the claim, create and bind a new child.
    pub(super) async fn claim_and_create_instance(
        pool: &str,
        key_value: &str,
        pool_cfg: &DirectoryPool,
        owner_pid: Principal,
    ) -> Result<Option<DirectoryEntryStatusResponse>, InternalError> {
        let now = IcOps::now_secs();
        let claim_id = new_claim_id();

        MetricEvent::started(MetricOperation::Claim);
        let claim_result =
            match DirectoryRegistryOps::claim_pending(pool, key_value, owner_pid, claim_id, now) {
                Ok(result) => result,
                Err(err) => {
                    MetricEvent::failed(MetricOperation::Claim, &err);
                    return Err(err);
                }
            };
        let claim = match claim_result {
            DirectoryClaimResult::Bound {
                instance_pid,
                bound_at,
            } => {
                MetricEvent::skipped(MetricOperation::Claim, MetricReason::AlreadyBound);
                return Ok(Some(DirectoryEntryStatusResponse::Bound {
                    instance_pid,
                    bound_at,
                }));
            }
            DirectoryClaimResult::PendingFresh {
                claim_id: _,
                owner_pid,
                created_at,
                provisional_pid,
            } => {
                MetricEvent::skipped(MetricOperation::Claim, MetricReason::PendingFresh);
                return Ok(Some(DirectoryEntryStatusResponse::Pending {
                    owner_pid,
                    created_at,
                    provisional_pid,
                }));
            }
            DirectoryClaimResult::Claimed(claim) => {
                MetricEvent::completed(MetricOperation::Claim, MetricReason::Claimed);
                claim
            }
        };

        MetricEvent::started(MetricOperation::CreateInstance);
        let pid = match RequestOps::create_canister::<()>(
            &pool_cfg.canister_role,
            CreateCanisterParent::ThisCanister,
            None,
        )
        .await
        {
            Ok(response) => {
                MetricEvent::completed(MetricOperation::CreateInstance, MetricReason::Ok);
                response.new_canister_pid
            }
            Err(err) => {
                MetricEvent::failed(MetricOperation::CreateInstance, &err);
                return Err(err);
            }
        };

        Self::finalize_created_instance(pool, key_value, claim, pid).await
    }
}
