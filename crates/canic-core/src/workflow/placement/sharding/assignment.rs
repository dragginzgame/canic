//! Module: workflow::placement::sharding::assignment
//!
//! Responsibility: assign partition keys to shards according to placement policy.
//! Does not own: policy rules, stable registry records, or canister creation internals.
//! Boundary: coordinates metrics, policy input mapping, allocation, and assignment writes.

use crate::{
    InternalError,
    cdk::types::Principal,
    config::schema::ShardPoolPolicy,
    domain::policy::placement::sharding::{
        ShardingPlanState, ShardingPolicy, ShardingState, compute_pool_metrics,
    },
    dto::placement::sharding::ShardingPlanStateResponse,
    ids::CanisterRole,
    log::Topic,
    ops::{
        placement::sharding::mapper::{
            ShardPartitionKeyAssignmentPolicyInputMapper, ShardPlacementPolicyInputMapper,
            ShardingPlanStateResponseMapper,
        },
        runtime::metrics::{
            recording::ShardingMetricEvent as MetricEvent,
            sharding::{
                ShardingMetricOperation as MetricOperation, ShardingMetricReason as MetricReason,
            },
        },
        storage::placement::{
            sharding::ShardingRegistryOps, sharding_lifecycle::ShardingLifecycleOps,
        },
    },
    workflow::placement::sharding::{ShardingWorkflow, ShardingWorkflowError},
};
use std::collections::BTreeSet;

impl ShardingWorkflow {
    pub async fn assign_to_pool(
        pool: &str,
        partition_key: impl AsRef<str>,
    ) -> Result<Principal, InternalError> {
        let pool_cfg = match Self::get_shard_pool_cfg(pool) {
            Ok(pool_cfg) => pool_cfg,
            Err(err) => {
                MetricEvent::started(MetricOperation::Assign);
                MetricEvent::failed(MetricOperation::Assign, &err);
                return Err(err);
            }
        };
        Self::assign_with_policy(
            &pool_cfg.canister_role,
            pool,
            partition_key.as_ref(),
            pool_cfg.policy,
            None,
        )
        .await
    }

    #[expect(clippy::too_many_lines)]
    pub async fn assign_with_policy(
        canister_role: &CanisterRole,
        pool: &str,
        partition_key: &str,
        policy: ShardPoolPolicy,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<Principal, InternalError> {
        MetricEvent::started(MetricOperation::Assign);
        let active = ShardingLifecycleOps::active_shards();
        crate::perf!("load_active_shards");
        if active.is_empty() {
            return match Self::assign_bootstrap_created(
                canister_role,
                pool,
                partition_key,
                &policy,
                extra_arg,
            )
            .await
            {
                Ok(pid) => {
                    MetricEvent::completed(MetricOperation::Assign, MetricReason::CreateAllowed);
                    Ok(pid)
                }
                Err(err) => {
                    MetricEvent::failed(MetricOperation::Assign, &err);
                    Err(err)
                }
            };
        }

        let active_set: BTreeSet<_> = active.into_iter().collect();
        let routable_active = Self::routable_active_set(&active_set);

        let entry_views: Vec<_> = ShardingRegistryOps::entries_for_pool(pool)
            .iter()
            .filter(|(pid, _)| routable_active.contains(pid))
            .map(|(pid, entry)| {
                ShardPlacementPolicyInputMapper::record_to_policy_input(*pid, entry)
            })
            .collect();

        let metrics = compute_pool_metrics(pool, &entry_views);

        let assignments_raw = ShardingRegistryOps::assignments_for_pool(pool);
        let assignment_views: Vec<_> = assignments_raw
            .iter()
            .filter(|(_, pid)| routable_active.contains(pid))
            .map(|(key, pid)| {
                ShardPartitionKeyAssignmentPolicyInputMapper::record_to_policy_input(key, *pid)
            })
            .collect();
        crate::perf!("collect_registry");

        let state = ShardingState {
            pool,
            max_shards: policy.max_shards,
            metrics: &metrics,
            entries: &entry_views,
            assignments: &assignment_views,
        };

        MetricEvent::started(MetricOperation::PlanAssign);
        let plan = ShardingPolicy::plan_assign(&state, partition_key, None);
        crate::perf!("plan_assign");

        match plan.state {
            ShardingPlanState::AlreadyAssigned { pid } => {
                MetricEvent::skipped(MetricOperation::PlanAssign, MetricReason::AlreadyAssigned);
                crate::perf!("already_assigned");
                let slot = plan
                    .target_slot
                    .or_else(|| ShardingRegistryOps::slot_for_shard(pool, pid));

                crate::log!(
                    Topic::Sharding,
                    Info,
                    "📦 partition_key={partition_key} already shard={pid} pool={pool} slot={slot:?}"
                );

                MetricEvent::completed(MetricOperation::Assign, MetricReason::AlreadyAssigned);
                Ok(pid)
            }

            ShardingPlanState::UseExisting { pid } => {
                MetricEvent::completed(MetricOperation::PlanAssign, MetricReason::ExistingCapacity);
                MetricEvent::started(MetricOperation::AssignKey);
                if let Err(err) = ShardingRegistryOps::assign(pool, partition_key, pid) {
                    MetricEvent::failed(MetricOperation::AssignKey, &err);
                    MetricEvent::failed(MetricOperation::Assign, &err);
                    return Err(err);
                }
                MetricEvent::completed(MetricOperation::AssignKey, MetricReason::ExistingCapacity);
                crate::perf!("assign_existing");

                let slot = plan
                    .target_slot
                    .or_else(|| ShardingRegistryOps::slot_for_shard(pool, pid));

                crate::log!(
                    Topic::Sharding,
                    Info,
                    "📦 partition_key={partition_key} assigned shard={pid} pool={pool} slot={slot:?}"
                );

                MetricEvent::completed(MetricOperation::Assign, MetricReason::ExistingCapacity);
                Ok(pid)
            }

            ShardingPlanState::CreateAllowed => {
                let Some(slot) = plan.target_slot else {
                    MetricEvent::failed_reason(
                        MetricOperation::PlanAssign,
                        MetricReason::InvalidState,
                    );
                    MetricEvent::failed_reason(MetricOperation::Assign, MetricReason::InvalidState);
                    return Err(ShardingWorkflowError::Invariant(
                        "sharding policy allowed creation but returned no slot",
                    )
                    .into());
                };
                MetricEvent::completed(MetricOperation::PlanAssign, MetricReason::CreateAllowed);

                let pid =
                    match Self::allocate_and_admit(pool, slot, canister_role, &policy, extra_arg)
                        .await
                    {
                        Ok(pid) => pid,
                        Err(err) => {
                            MetricEvent::failed(MetricOperation::Assign, &err);
                            return Err(err);
                        }
                    };
                crate::perf!("allocate_shard");

                MetricEvent::started(MetricOperation::AssignKey);
                if let Err(err) = ShardingRegistryOps::assign(pool, partition_key, pid) {
                    MetricEvent::failed(MetricOperation::AssignKey, &err);
                    MetricEvent::failed(MetricOperation::Assign, &err);
                    return Err(err);
                }
                MetricEvent::completed(MetricOperation::AssignKey, MetricReason::CreateAllowed);
                crate::perf!("assign_created");

                crate::log!(
                    Topic::Sharding,
                    Ok,
                    "✨ partition_key={partition_key} created+assigned shard={pid} pool={pool} slot={slot}"
                );

                MetricEvent::completed(MetricOperation::Assign, MetricReason::CreateAllowed);
                Ok(pid)
            }

            ShardingPlanState::CreateBlocked { reason } => {
                let metric_reason = MetricReason::from_create_blocked_reason(&reason);
                MetricEvent::skipped(MetricOperation::PlanAssign, metric_reason);
                MetricEvent::failed_reason(MetricOperation::Assign, metric_reason);
                crate::perf!("create_blocked");
                Err(Self::blocked(reason, pool, partition_key))
            }
        }
    }

    pub fn plan_assign_to_pool(
        pool: &str,
        partition_key: impl AsRef<str>,
    ) -> Result<ShardingPlanStateResponse, InternalError> {
        let pool_cfg = Self::get_shard_pool_cfg(pool)?;
        let partition_key = partition_key.as_ref();

        let active = ShardingLifecycleOps::active_shards();
        if active.is_empty() {
            Self::ensure_bootstrap_capacity(pool, partition_key, &pool_cfg.policy)?;
        }

        let active_set: BTreeSet<_> = active.into_iter().collect();
        let routable_active = Self::routable_active_set(&active_set);

        let entry_views: Vec<_> = ShardingRegistryOps::entries_for_pool(pool)
            .iter()
            .filter(|(pid, _)| routable_active.contains(pid))
            .map(|(pid, entry)| {
                ShardPlacementPolicyInputMapper::record_to_policy_input(*pid, entry)
            })
            .collect();

        let metrics = compute_pool_metrics(pool, &entry_views);

        let assignments_raw = ShardingRegistryOps::assignments_for_pool(pool);
        let assignment_views: Vec<_> = assignments_raw
            .iter()
            .filter(|(_, pid)| routable_active.contains(pid))
            .map(|(key, pid)| {
                ShardPartitionKeyAssignmentPolicyInputMapper::record_to_policy_input(key, *pid)
            })
            .collect();

        let state = ShardingState {
            pool,
            max_shards: pool_cfg.policy.max_shards,
            metrics: &metrics,
            entries: &entry_views,
            assignments: &assignment_views,
        };

        let plan = ShardingPolicy::plan_assign(&state, partition_key, None);
        Ok(ShardingPlanStateResponseMapper::record_to_view(plan.state))
    }
}
