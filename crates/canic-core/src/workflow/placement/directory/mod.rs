pub mod query;
mod state;

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    config::schema::DirectoryPool,
    dto::placement::directory::{DirectoryEntryStatusResponse, DirectoryRecoveryResponse},
    ops::{
        config::ConfigOps,
        ic::IcOps,
        rpc::request::{CreateCanisterParent, RequestOps},
        runtime::metrics::{
            directory::{
                DirectoryMetricOperation as MetricOperation, DirectoryMetricReason as MetricReason,
            },
            recording::DirectoryMetricEvent as MetricEvent,
        },
        storage::placement::directory::{
            DirectoryClaimResult, DirectoryEntryState, DirectoryPendingClaim, DirectoryRegistryOps,
            DirectoryReleaseResult,
        },
        storage::registry::subnet::SubnetRegistryOps,
    },
};
use state::{
    DirectoryEntryClassification, DirectoryWorkflowError, available_pool_names, new_claim_id,
    pending_is_stale, validate_bind_target_with_reason,
};

pub struct DirectoryWorkflow;

impl DirectoryWorkflow {
    /// Resolve a bound instance for one key or create and bind a new one.
    pub async fn resolve_or_create(
        pool: &str,
        key_value: &str,
    ) -> Result<DirectoryEntryStatusResponse, InternalError> {
        MetricEvent::started(MetricOperation::Resolve);
        let pool_cfg = match Self::get_directory_pool_cfg(pool) {
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
                Some(DirectoryEntryClassification::Bound {
                    instance_pid,
                    bound_at,
                }) => {
                    MetricEvent::completed(MetricOperation::Resolve, MetricReason::AlreadyBound);
                    return Ok(DirectoryEntryStatusResponse::Bound {
                        instance_pid,
                        bound_at,
                    });
                }

                Some(DirectoryEntryClassification::PendingFresh {
                    owner_pid,
                    created_at,
                    provisional_pid,
                }) => {
                    MetricEvent::skipped(MetricOperation::Resolve, MetricReason::PendingFresh);
                    return Ok(DirectoryEntryStatusResponse::Pending {
                        owner_pid,
                        created_at,
                        provisional_pid,
                    });
                }

                Some(DirectoryEntryClassification::Repairable {
                    claim_id,
                    provisional_pid,
                }) => {
                    let repaired =
                        Self::repair_stale_entry(pool, key_value, claim_id, provisional_pid, now)?;
                    MetricEvent::completed(MetricOperation::Resolve, MetricReason::StaleRepairable);
                    return Ok(repaired);
                }

                Some(DirectoryEntryClassification::NeedsCleanup {
                    claim_id,
                    provisional_pid,
                }) => {
                    if let Err(err) =
                        Self::cleanup_stale_entry(pool, key_value, claim_id, provisional_pid).await
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

    /// Recover one directory entry by repairing a valid stale provisional child or
    /// releasing a dead pending claim.
    pub async fn recover_entry(
        pool: &str,
        key_value: &str,
    ) -> Result<DirectoryRecoveryResponse, InternalError> {
        MetricEvent::started(MetricOperation::Recover);
        let pool_cfg = match Self::get_directory_pool_cfg(pool) {
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
                    return Ok(DirectoryRecoveryResponse::Missing);
                }

                Some(DirectoryEntryClassification::Bound {
                    instance_pid,
                    bound_at,
                }) => {
                    MetricEvent::completed(MetricOperation::Recover, MetricReason::AlreadyBound);
                    return Ok(DirectoryRecoveryResponse::Bound {
                        instance_pid,
                        bound_at,
                    });
                }

                Some(DirectoryEntryClassification::PendingFresh {
                    owner_pid,
                    created_at,
                    provisional_pid,
                }) => {
                    MetricEvent::skipped(MetricOperation::Recover, MetricReason::PendingFresh);
                    return Ok(DirectoryRecoveryResponse::FreshPending {
                        owner_pid,
                        created_at,
                        provisional_pid,
                    });
                }

                Some(DirectoryEntryClassification::Repairable {
                    claim_id,
                    provisional_pid,
                }) => {
                    let repaired =
                        Self::repair_stale_entry(pool, key_value, claim_id, provisional_pid, now)?;

                    let DirectoryEntryStatusResponse::Bound {
                        instance_pid,
                        bound_at,
                    } = repaired
                    else {
                        return Err(InternalError::invariant(
                            InternalErrorOrigin::Workflow,
                            "directory stale repair returned non-bound status",
                        ));
                    };

                    MetricEvent::completed(MetricOperation::Recover, MetricReason::StaleRepairable);
                    return Ok(DirectoryRecoveryResponse::RepairedToBound {
                        instance_pid,
                        bound_at,
                    });
                }

                Some(DirectoryEntryClassification::NeedsCleanup {
                    claim_id,
                    provisional_pid,
                }) => {
                    if let Some(response) = Self::recover_cleanup_stale_entry(
                        pool,
                        key_value,
                        claim_id,
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

    /// Bind one logical directory key to a validated direct child instance.
    pub fn bind_instance(pool: &str, key_value: &str, pid: Principal) -> Result<(), InternalError> {
        MetricEvent::started(MetricOperation::Bind);
        let pool_cfg = match Self::get_directory_pool_cfg(pool) {
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
        if let Err(err) = DirectoryRegistryOps::bind(pool, key_value, pid, IcOps::now_secs()) {
            MetricEvent::failed(MetricOperation::Bind, &err);
            return Err(err);
        }

        MetricEvent::completed(MetricOperation::Bind, MetricReason::Ok);
        Ok(())
    }

    // Finalize one freshly created child using claim-matching writes so late async completions
    // cannot overwrite a newer claim after the key has been reclaimed.
    async fn finalize_created_instance(
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
    async fn claim_and_create_instance(
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

    // Recycle any abandoned provisional child and release the stale claim so one caller can
    // re-claim the key in the same user-driven flow without background timers.
    async fn cleanup_stale_entry(
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
    async fn recycle_abandoned_child(pid: Principal) -> Result<(), InternalError> {
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
    async fn recover_cleanup_stale_entry(
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

    // Classify the current entry once so resolve and recovery follow the same stale/repair rules.
    fn classify_entry(
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

    // Repair a stale valid provisional child only if its original claim is still current.
    fn repair_stale_entry(
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

    // Resolve the configured pool definition for the current directory-bearing parent.
    fn get_directory_pool_cfg(pool: &str) -> Result<DirectoryPool, InternalError> {
        let directory = ConfigOps::current_directory_config()?
            .ok_or(DirectoryWorkflowError::DirectoryDisabled)?;
        let available = available_pool_names(&directory);

        directory
            .pools
            .get(pool)
            .cloned()
            .ok_or_else(|| DirectoryWorkflowError::UnknownPool {
                requested: pool.to_string(),
                available,
            })
            .map_err(InternalError::from)
    }
}

#[cfg(test)]
mod tests;
