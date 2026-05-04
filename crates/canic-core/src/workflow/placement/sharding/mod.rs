pub mod query;

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    config::schema::{ShardPool, ShardPoolPolicy},
    domain::policy::placement::sharding::{
        CreateBlockedReason, HrwSelector, ShardingPlanState, ShardingPolicy, ShardingPolicyError,
        ShardingState, compute_pool_metrics,
    },
    dto::placement::sharding::ShardingPlanStateResponse,
    ids::CanisterRole,
    log::Topic,
    ops::{
        config::ConfigOps,
        ic::IcOps,
        placement::sharding::mapper::{
            ShardPartitionKeyAssignmentPolicyInputMapper, ShardPlacementPolicyInputMapper,
            ShardingPlanStateResponseMapper,
        },
        rpc::request::{CreateCanisterParent, RequestOps},
        runtime::metrics::{
            recording::ShardingMetricEvent as MetricEvent,
            sharding::{
                ShardingMetricOperation as MetricOperation, ShardingMetricOutcome as MetricOutcome,
                ShardingMetricReason as MetricReason,
            },
        },
        storage::{
            children::CanisterChildrenOps,
            placement::{sharding::ShardingRegistryOps, sharding_lifecycle::ShardingLifecycleOps},
        },
    },
    view::placement::sharding::ShardPlacement,
};
use std::collections::BTreeSet;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum ShardingWorkflowError {
    #[error(transparent)]
    Policy(#[from] ShardingPolicyError),

    #[error("invariant violation: {0}")]
    Invariant(&'static str),
}

impl From<ShardingWorkflowError> for InternalError {
    fn from(err: ShardingWorkflowError) -> Self {
        match err {
            ShardingWorkflowError::Policy(err) => {
                Self::domain(InternalErrorOrigin::Domain, err.to_string())
            }
            ShardingWorkflowError::Invariant(msg) => {
                Self::invariant(InternalErrorOrigin::Workflow, msg)
            }
        }
    }
}

pub struct ShardAllocator;

impl ShardAllocator {
    async fn allocate(
        pool: &str,
        slot: u32,
        canister_role: &CanisterRole,
        policy: &ShardPoolPolicy,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<Principal, InternalError> {
        MetricEvent::started(MetricOperation::CreateShard);

        let pid = match Self::create_canister_pid(canister_role, extra_arg).await {
            Ok(pid) => pid,
            Err(err) => {
                MetricEvent::failed(MetricOperation::CreateShard, &err);
                return Err(err);
            }
        };
        let created_at = IcOps::now_secs();
        if let Err(err) =
            ShardingRegistryOps::create(pid, pool, slot, canister_role, policy.capacity, created_at)
        {
            MetricEvent::failed(MetricOperation::CreateShard, &err);
            return Err(err);
        }

        crate::log!(
            Topic::Sharding,
            Ok,
            "✨ shard.create: {pid} pool={pool} slot={slot}"
        );

        MetricEvent::completed(MetricOperation::CreateShard, MetricReason::Ok);
        Ok(pid)
    }

    async fn create_canister_pid(
        canister_role: &CanisterRole,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<Principal, InternalError> {
        let response = RequestOps::create_canister::<Vec<u8>>(
            canister_role,
            CreateCanisterParent::ThisCanister,
            extra_arg,
        )
        .await?;

        Ok(response.new_canister_pid)
    }
}

pub struct ShardingWorkflow;

impl ShardingWorkflow {
    /// Create configured startup shards for every pool on the current canister.
    pub async fn bootstrap_configured_initial_shards() -> Result<(), InternalError> {
        let canister = match ConfigOps::current_canister() {
            Ok(canister) => canister,
            Err(err) => {
                MetricEvent::failed(MetricOperation::BootstrapConfig, &err);
                return Err(err);
            }
        };
        let Some(sharding) = canister.sharding else {
            MetricEvent::skipped(
                MetricOperation::BootstrapConfig,
                MetricReason::ShardingDisabled,
            );
            return Ok(());
        };

        MetricEvent::started(MetricOperation::BootstrapConfig);
        for (pool, pool_cfg) in sharding.pools {
            if let Err(err) = Self::bootstrap_initial_shards_for_pool(&pool, &pool_cfg).await {
                MetricEvent::failed(MetricOperation::BootstrapConfig, &err);
                return Err(err);
            }
        }

        MetricEvent::completed(MetricOperation::BootstrapConfig, MetricReason::Ok);
        Ok(())
    }

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
            config: ShardPool {
                canister_role: canister_role.clone(),
                policy: policy.clone(),
            },
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
            config: pool_cfg,
            metrics: &metrics,
            entries: &entry_views,
            assignments: &assignment_views,
        };

        let plan = ShardingPolicy::plan_assign(&state, partition_key, None);
        Ok(ShardingPlanStateResponseMapper::record_to_view(plan.state))
    }

    // Assign the first shard in an empty pool and persist the initial partition mapping.
    async fn assign_bootstrap_created(
        canister_role: &CanisterRole,
        pool: &str,
        partition_key: &str,
        policy: &ShardPoolPolicy,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<Principal, InternalError> {
        MetricEvent::started(MetricOperation::BootstrapActive);
        let (pid, slot) = match Self::bootstrap_empty_active(
            canister_role,
            pool,
            partition_key,
            policy,
            extra_arg,
        )
        .await
        {
            Ok(value) => value,
            Err(err) => {
                MetricEvent::failed(MetricOperation::BootstrapActive, &err);
                return Err(err);
            }
        };
        crate::perf!("bootstrap_empty_active");

        MetricEvent::started(MetricOperation::AssignKey);
        if let Err(err) = ShardingRegistryOps::assign(pool, partition_key, pid) {
            MetricEvent::failed(MetricOperation::AssignKey, &err);
            MetricEvent::failed(MetricOperation::BootstrapActive, &err);
            return Err(err);
        }
        MetricEvent::completed(MetricOperation::AssignKey, MetricReason::CreateAllowed);
        crate::perf!("assign_bootstrap_created");

        crate::log!(
            Topic::Sharding,
            Ok,
            "✨ partition_key={partition_key} created+assigned shard={pid} pool={pool} slot={slot}"
        );

        MetricEvent::completed(
            MetricOperation::BootstrapActive,
            MetricReason::CreateAllowed,
        );
        Ok(pid)
    }

    // Select a free slot and admit the first active shard for an empty pool.
    #[expect(clippy::cast_possible_truncation)]
    async fn bootstrap_empty_active(
        canister_role: &CanisterRole,
        pool: &str,
        partition_key: &str,
        policy: &ShardPoolPolicy,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<(Principal, u32), InternalError> {
        let pool_entries = Self::pool_entry_views(pool);
        crate::perf!("load_bootstrap_pool_entries");
        if pool_entries.len() as u32 >= policy.max_shards {
            return Err(Self::no_active_shards_exhausted(pool, partition_key));
        }

        let free_slots = Self::free_slots(policy.max_shards, &pool_entries);
        let slot = HrwSelector::select_from_slots(pool, partition_key, &free_slots)
            .ok_or_else(|| Self::no_active_shards_exhausted(pool, partition_key))?;
        crate::perf!("select_bootstrap_slot");

        let pid = Self::allocate_and_admit(pool, slot, canister_role, policy, extra_arg).await?;
        crate::perf!("allocate_bootstrap_shard");
        Ok((pid, slot))
    }

    #[expect(clippy::cast_possible_truncation)]
    fn ensure_bootstrap_capacity(
        pool: &str,
        partition_key: &str,
        policy: &ShardPoolPolicy,
    ) -> Result<(), InternalError> {
        let pool_entries = Self::pool_entry_views(pool);
        if pool_entries.len() as u32 >= policy.max_shards {
            return Err(Self::no_active_shards_exhausted(pool, partition_key));
        }

        Ok(())
    }

    fn pool_entry_views(pool: &str) -> Vec<(Principal, ShardPlacement)> {
        let direct_children = Self::direct_child_pid_set();
        ShardingRegistryOps::entries_for_pool(pool)
            .iter()
            .filter(|(pid, _)| direct_children.is_empty() || direct_children.contains(pid))
            .map(|(pid, entry)| {
                ShardPlacementPolicyInputMapper::record_to_policy_input(*pid, entry)
            })
            .collect()
    }

    fn routable_active_set(active: &BTreeSet<Principal>) -> BTreeSet<Principal> {
        let direct_children = Self::direct_child_pid_set();
        if direct_children.is_empty() {
            return active.clone();
        }

        active.intersection(&direct_children).copied().collect()
    }

    fn direct_child_pid_set() -> BTreeSet<Principal> {
        CanisterChildrenOps::data()
            .entries
            .into_iter()
            .map(|(pid, _)| pid)
            .collect()
    }

    fn free_slots(max_shards: u32, entries: &[(Principal, ShardPlacement)]) -> Vec<u32> {
        let mut occupied = BTreeSet::new();
        for (_, entry) in entries {
            if entry.slot != ShardPlacement::UNASSIGNED_SLOT {
                occupied.insert(entry.slot);
            }
        }

        (0..max_shards)
            .filter(|slot| !occupied.contains(slot))
            .collect()
    }

    // Create enough unassigned shards to satisfy a pool's configured startup target.
    async fn bootstrap_initial_shards_for_pool(
        pool: &str,
        pool_cfg: &ShardPool,
    ) -> Result<(), InternalError> {
        MetricEvent::started(MetricOperation::BootstrapPool);
        let target = pool_cfg
            .policy
            .initial_shards
            .min(pool_cfg.policy.max_shards);
        if target == 0 {
            MetricEvent::skipped(
                MetricOperation::BootstrapPool,
                MetricReason::NoInitialShards,
            );
            return Ok(());
        }

        let mut created = 0u32;
        loop {
            let pool_entries = Self::pool_entry_views(pool);
            let current = u32::try_from(pool_entries.len()).unwrap_or(u32::MAX);
            if current >= target {
                MetricEvent::record(
                    MetricOperation::BootstrapPool,
                    if created == 0 {
                        MetricOutcome::Skipped
                    } else {
                        MetricOutcome::Completed
                    },
                    if created == 0 {
                        MetricReason::TargetSatisfied
                    } else {
                        MetricReason::Ok
                    },
                );
                return Ok(());
            }

            let slot = Self::free_slots(pool_cfg.policy.max_shards, &pool_entries)
                .into_iter()
                .next()
                .ok_or_else(|| Self::no_active_shards_exhausted(pool, "__bootstrap__"))?;
            let pid = match Self::allocate_and_admit(
                pool,
                slot,
                &pool_cfg.canister_role,
                &pool_cfg.policy,
                None,
            )
            .await
            {
                Ok(pid) => pid,
                Err(err) => {
                    MetricEvent::failed(MetricOperation::BootstrapPool, &err);
                    return Err(err);
                }
            };
            created = created.saturating_add(1);

            crate::log!(
                Topic::Sharding,
                Ok,
                "✨ shard.bootstrap: {pid} pool={pool} slot={slot}"
            );
        }
    }

    async fn allocate_and_admit(
        pool: &str,
        slot: u32,
        canister_role: &CanisterRole,
        policy: &ShardPoolPolicy,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<Principal, InternalError> {
        let pid = ShardAllocator::allocate(pool, slot, canister_role, policy, extra_arg).await?;
        Self::admit_shard(pid);
        Ok(pid)
    }

    fn admit_shard(pid: Principal) {
        ShardingLifecycleOps::set_active(pid);
    }

    fn no_active_shards_exhausted(pool: &str, partition_key: &str) -> InternalError {
        InternalError::domain(
            InternalErrorOrigin::Workflow,
            format!(
                "no active shards in pool '{pool}' and max_shards exhausted; cannot assign partition_key '{partition_key}'"
            ),
        )
    }

    fn blocked(reason: CreateBlockedReason, pool: &str, partition_key: &str) -> InternalError {
        ShardingWorkflowError::Policy(ShardingPolicyError::ShardCreationBlocked {
            reason,
            partition_key: partition_key.to_string(),
            pool: pool.to_string(),
        })
        .into()
    }

    fn get_shard_pool_cfg(pool: &str) -> Result<ShardPool, InternalError> {
        let sharding = ConfigOps::current_canister()?
            .sharding
            .ok_or(ShardingPolicyError::ShardingDisabled)?;
        let available = if sharding.pools.is_empty() {
            "<none>".to_string()
        } else {
            let mut names: Vec<_> = sharding.pools.keys().cloned().collect();
            names.sort_unstable();
            names.join(", ")
        };

        sharding
            .pools
            .get(pool)
            .cloned()
            .ok_or_else(|| ShardingPolicyError::PoolNotFound {
                requested: pool.to_string(),
                available: available.clone(),
            })
            .map_err(InternalError::from)
    }
}
