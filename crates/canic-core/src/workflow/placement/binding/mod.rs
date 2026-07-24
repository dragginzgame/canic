//! Module: workflow::placement::binding
//!
//! Responsibility: resolve, create, repair, and recover binding-bound instances.
//! Does not own: storage schemas, request endpoint authorization, or pool lifecycle policy.
//! Boundary: coordinates binding storage, child creation, and stale-claim recovery.

mod classification;
mod cleanup;
mod config;
mod create;
pub mod query;
mod state;

use crate::workflow::placement::binding::state::{
    PlacementBindingEntryClassification, validate_bind_target_with_reason,
};
use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    dto::placement::binding::{PlacementBindingRecoveryResponse, PlacementBindingStatusResponse},
    ops::{
        ic::IcOps,
        runtime::metrics::{
            placement_binding::{
                PlacementBindingMetricOperation as MetricOperation,
                PlacementBindingMetricReason as MetricReason,
            },
            recording::PlacementBindingMetricEvent as MetricEvent,
        },
        storage::placement::binding::PlacementBindingRegistryOps,
    },
};

///
/// PlacementBindingWorkflow
///
/// Entry point for binding placement orchestration.
///
pub struct PlacementBindingWorkflow;

impl PlacementBindingWorkflow {
    /// Resolve a bound instance for one key or create and bind a new one.
    #[expect(
        clippy::too_many_lines,
        reason = "the exhaustive binding state-machine loop is clearer in one owner"
    )]
    pub async fn resolve_or_create(
        pool: &str,
        key_value: &str,
    ) -> Result<PlacementBindingStatusResponse, InternalError> {
        MetricEvent::started(MetricOperation::Resolve);
        let pool_cfg = match Self::get_binding_pool_cfg(pool) {
            Ok(pool_cfg) => pool_cfg,
            Err(err) => {
                MetricEvent::failed(MetricOperation::Resolve, &err);
                return Err(err);
            }
        };
        let owner_pid = IcOps::metadata_entropy_canister();

        loop {
            let now = IcOps::now_secs();

            match Self::classify_entry(pool, key_value, &pool_cfg, now) {
                Some(PlacementBindingEntryClassification::Bound {
                    instance_pid,
                    bound_at,
                }) => {
                    MetricEvent::completed(MetricOperation::Resolve, MetricReason::AlreadyBound);
                    return Ok(PlacementBindingStatusResponse::Bound {
                        instance_pid,
                        bound_at,
                    });
                }

                Some(PlacementBindingEntryClassification::PendingFresh {
                    claim_id: _,
                    owner_pid,
                    created_at,
                    provisional_pid,
                }) => {
                    MetricEvent::skipped(MetricOperation::Resolve, MetricReason::PendingFresh);
                    return Ok(PlacementBindingStatusResponse::Pending {
                        owner_pid,
                        created_at,
                        provisional_pid,
                    });
                }

                Some(PlacementBindingEntryClassification::Repairable {
                    claim_id,
                    owner_pid,
                    provisional_pid,
                }) => {
                    let repaired = Self::repair_stale_entry(
                        pool,
                        key_value,
                        &pool_cfg,
                        claim_id,
                        owner_pid,
                        provisional_pid,
                        now,
                    )?;
                    MetricEvent::completed(MetricOperation::Resolve, MetricReason::StaleRepairable);
                    return Ok(repaired);
                }

                Some(PlacementBindingEntryClassification::Resumable {
                    claim_id,
                    owner_pid,
                    created_at,
                }) => {
                    let status = Self::resume_pending_instance(
                        pool,
                        key_value,
                        &pool_cfg,
                        crate::ops::storage::placement::binding::PlacementBindingPendingClaim {
                            claim_id,
                            owner_pid,
                            created_at,
                        },
                    )
                    .await?;
                    if let Some(status) = status {
                        MetricEvent::completed(
                            MetricOperation::Resolve,
                            MetricReason::ResumedPending,
                        );
                        return Ok(status);
                    }
                }

                Some(PlacementBindingEntryClassification::NeedsCleanup {
                    claim_id,
                    owner_pid,
                    provisional_pid,
                }) => {
                    if let Err(err) = Self::cleanup_stale_entry(
                        pool,
                        key_value,
                        &pool_cfg,
                        claim_id,
                        owner_pid,
                        provisional_pid,
                    )
                    .await
                    {
                        MetricEvent::failed(MetricOperation::Resolve, &err);
                        return Err(err);
                    }
                }

                None => {
                    let status = match Self::claim_and_create_instance(
                        pool, key_value, &pool_cfg, owner_pid,
                    )
                    .await
                    {
                        Ok(status) => status,
                        Err(err) => {
                            MetricEvent::failed(MetricOperation::Resolve, &err);
                            return Err(err);
                        }
                    };
                    if let Some(status) = status {
                        MetricEvent::completed(MetricOperation::Resolve, MetricReason::Ok);
                        return Ok(status);
                    }
                }
            }
        }
    }

    /// Recover one binding entry by repairing a valid stale provisional child or
    /// releasing a dead pending claim.
    #[expect(
        clippy::too_many_lines,
        reason = "the exhaustive binding recovery state machine has one orchestration owner"
    )]
    pub async fn recover_entry(
        pool: &str,
        key_value: &str,
    ) -> Result<PlacementBindingRecoveryResponse, InternalError> {
        MetricEvent::started(MetricOperation::Recover);
        let pool_cfg = match Self::get_binding_pool_cfg(pool) {
            Ok(pool_cfg) => pool_cfg,
            Err(err) => {
                MetricEvent::failed(MetricOperation::Recover, &err);
                return Err(err);
            }
        };
        loop {
            let now = IcOps::now_secs();

            match Self::classify_entry(pool, key_value, &pool_cfg, now) {
                None => {
                    MetricEvent::skipped(MetricOperation::Recover, MetricReason::Missing);
                    return Ok(PlacementBindingRecoveryResponse::Missing);
                }

                Some(PlacementBindingEntryClassification::Bound {
                    instance_pid,
                    bound_at,
                }) => {
                    MetricEvent::completed(MetricOperation::Recover, MetricReason::AlreadyBound);
                    return Ok(PlacementBindingRecoveryResponse::Bound {
                        instance_pid,
                        bound_at,
                    });
                }

                Some(PlacementBindingEntryClassification::PendingFresh {
                    claim_id: _,
                    owner_pid,
                    created_at,
                    provisional_pid,
                }) => {
                    MetricEvent::skipped(MetricOperation::Recover, MetricReason::PendingFresh);
                    return Ok(PlacementBindingRecoveryResponse::FreshPending {
                        owner_pid,
                        created_at,
                        provisional_pid,
                    });
                }

                Some(PlacementBindingEntryClassification::Repairable {
                    claim_id,
                    owner_pid,
                    provisional_pid,
                }) => {
                    let repaired = Self::repair_stale_entry(
                        pool,
                        key_value,
                        &pool_cfg,
                        claim_id,
                        owner_pid,
                        provisional_pid,
                        now,
                    )?;

                    let PlacementBindingStatusResponse::Bound {
                        instance_pid,
                        bound_at,
                    } = repaired
                    else {
                        return Err(InternalError::invariant(
                            InternalErrorOrigin::Workflow,
                            "binding stale repair returned non-bound status",
                        ));
                    };

                    MetricEvent::completed(MetricOperation::Recover, MetricReason::StaleRepairable);
                    return Ok(PlacementBindingRecoveryResponse::RepairedToBound {
                        instance_pid,
                        bound_at,
                    });
                }

                Some(PlacementBindingEntryClassification::Resumable {
                    claim_id,
                    owner_pid,
                    created_at,
                }) => {
                    let status = Self::resume_pending_instance(
                        pool,
                        key_value,
                        &pool_cfg,
                        crate::ops::storage::placement::binding::PlacementBindingPendingClaim {
                            claim_id,
                            owner_pid,
                            created_at,
                        },
                    )
                    .await?;
                    let Some(PlacementBindingStatusResponse::Bound {
                        instance_pid,
                        bound_at,
                    }) = status
                    else {
                        continue;
                    };
                    MetricEvent::completed(MetricOperation::Recover, MetricReason::ResumedPending);
                    return Ok(PlacementBindingRecoveryResponse::ResumedToBound {
                        instance_pid,
                        bound_at,
                    });
                }

                Some(PlacementBindingEntryClassification::NeedsCleanup {
                    claim_id,
                    owner_pid,
                    provisional_pid,
                }) => {
                    if let Some(response) = Self::recover_cleanup_stale_entry(
                        pool,
                        key_value,
                        &pool_cfg,
                        claim_id,
                        owner_pid,
                        provisional_pid,
                    )
                    .await?
                    {
                        MetricEvent::completed(
                            MetricOperation::Recover,
                            MetricReason::StaleCleanup,
                        );
                        return Ok(response);
                    }
                }
            }
        }
    }

    /// Bind one logical binding key to a validated direct child instance.
    pub fn bind_instance(pool: &str, key_value: &str, pid: Principal) -> Result<(), InternalError> {
        MetricEvent::started(MetricOperation::Bind);
        let pool_cfg = match Self::get_binding_pool_cfg(pool) {
            Ok(pool_cfg) => pool_cfg,
            Err(err) => {
                MetricEvent::failed(MetricOperation::Bind, &err);
                return Err(err);
            }
        };
        if let Err((err, reason)) = validate_bind_target_with_reason(pid, &pool_cfg.canister_role) {
            MetricEvent::failed_reason(MetricOperation::Bind, reason);
            return Err(err);
        }
        if let Err(err) = PlacementBindingRegistryOps::bind(pool, key_value, pid, IcOps::now_secs())
        {
            MetricEvent::failed(MetricOperation::Bind, &err);
            return Err(err);
        }

        MetricEvent::completed(MetricOperation::Bind, MetricReason::Ok);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
