pub mod query;

use crate::{
    mapper::{
        ShardPartitionKeyAssignmentPolicyInputMapper, ShardPlacementPolicyInputMapper,
        ShardingPlanStateResponseMapper,
    },
    policy::{
        CreateBlockedReason, ShardingPlanState, ShardingPolicy, ShardingPolicyError, ShardingState,
        hrw::HrwSelector, metrics::compute_pool_metrics,
    },
    view::ShardPlacement,
};
use canic_core::{
    __sharding_core as sharding_core,
    cdk::types::Principal,
    dto::placement::sharding::ShardingPlanStateResponse,
    error::{InternalError, InternalErrorOrigin},
    ids::CanisterRole,
    log::Topic,
};
use sharding_core::{
    config::schema::{ShardPool, ShardPoolPolicy},
    ops::{
        config::current_sharding_config,
        ic::IcOps,
        rpc::request::{CreateCanisterParent, RequestOps},
        storage::{
            children::CanisterChildrenOps,
            placement::{sharding::ShardingRegistryOps, sharding_lifecycle::ShardingLifecycleOps},
        },
    },
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
            ShardingWorkflowError::Policy(e) => {
                Self::domain(InternalErrorOrigin::Domain, e.to_string())
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
        let pid = Self::create_canister_pid(canister_role, extra_arg).await?;
        let created_at = IcOps::now_secs();
        ShardingRegistryOps::create(pid, pool, slot, canister_role, policy.capacity, created_at)?;

        canic_core::log!(
            Topic::Sharding,
            Ok,
            "✨ shard.create: {pid} pool={pool} slot={slot}"
        );

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
    pub async fn assign_to_pool(
        pool: &str,
        partition_key: impl AsRef<str>,
    ) -> Result<Principal, InternalError> {
        let pool_cfg = Self::get_shard_pool_cfg(pool)?;
        Self::assign_with_policy(
            &pool_cfg.canister_role,
            pool,
            partition_key.as_ref(),
            pool_cfg.policy,
            None,
        )
        .await
    }

    pub async fn assign_with_policy(
        canister_role: &CanisterRole,
        pool: &str,
        partition_key: &str,
        policy: ShardPoolPolicy,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<Principal, InternalError> {
        let active = ShardingLifecycleOps::active_shards();
        canic_core::perf!("load_active_shards");
        if active.is_empty() {
            return Self::assign_bootstrap_created(
                canister_role,
                pool,
                partition_key,
                &policy,
                extra_arg,
            )
            .await;
        }

        let active_set: BTreeSet<_> = active.into_iter().collect();
        let routable_active = Self::routable_active_set(&active_set);

        let registry = ShardingRegistryOps::export();
        let entry_views: Vec<_> = registry
            .entries
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
        canic_core::perf!("collect_registry");

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

        let plan = ShardingPolicy::plan_assign(&state, partition_key, None);
        canic_core::perf!("plan_assign");

        match plan.state {
            ShardingPlanState::AlreadyAssigned { pid } => {
                canic_core::perf!("already_assigned");
                let slot = plan
                    .target_slot
                    .or_else(|| ShardingRegistryOps::slot_for_shard(pool, pid));

                canic_core::log!(
                    Topic::Sharding,
                    Info,
                    "📦 partition_key={partition_key} already shard={pid} pool={pool} slot={slot:?}"
                );

                Ok(pid)
            }

            ShardingPlanState::UseExisting { pid } => {
                ShardingRegistryOps::assign(pool, partition_key, pid)?;
                canic_core::perf!("assign_existing");

                let slot = plan
                    .target_slot
                    .or_else(|| ShardingRegistryOps::slot_for_shard(pool, pid));

                canic_core::log!(
                    Topic::Sharding,
                    Info,
                    "📦 partition_key={partition_key} assigned shard={pid} pool={pool} slot={slot:?}"
                );

                Ok(pid)
            }

            ShardingPlanState::CreateAllowed => {
                let slot = plan.target_slot.ok_or(ShardingWorkflowError::Invariant(
                    "sharding policy allowed creation but returned no slot",
                ))?;

                let pid =
                    Self::allocate_and_admit(pool, slot, canister_role, &policy, extra_arg).await?;
                canic_core::perf!("allocate_shard");

                ShardingRegistryOps::assign(pool, partition_key, pid)?;
                canic_core::perf!("assign_created");

                canic_core::log!(
                    Topic::Sharding,
                    Ok,
                    "✨ partition_key={partition_key} created+assigned shard={pid} pool={pool} slot={slot}"
                );

                Ok(pid)
            }

            ShardingPlanState::CreateBlocked { reason } => {
                canic_core::perf!("create_blocked");
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

        let registry = ShardingRegistryOps::export();
        let entry_views: Vec<_> = registry
            .entries
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
        let (pid, slot) =
            Self::bootstrap_empty_active(canister_role, pool, partition_key, policy, extra_arg)
                .await?;
        canic_core::perf!("bootstrap_empty_active");

        ShardingRegistryOps::assign(pool, partition_key, pid)?;
        canic_core::perf!("assign_bootstrap_created");

        canic_core::log!(
            Topic::Sharding,
            Ok,
            "✨ partition_key={partition_key} created+assigned shard={pid} pool={pool} slot={slot}"
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
        if pool_entries.len() as u32 >= policy.max_shards {
            return Err(Self::no_active_shards_exhausted(pool, partition_key));
        }

        let free_slots = Self::free_slots(policy.max_shards, &pool_entries);
        let slot = HrwSelector::select_from_slots(pool, partition_key, &free_slots)
            .ok_or_else(|| Self::no_active_shards_exhausted(pool, partition_key))?;

        let pid = Self::allocate_and_admit(pool, slot, canister_role, policy, extra_arg).await?;
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
        let registry = ShardingRegistryOps::export();
        let direct_children = Self::direct_child_pid_set();
        registry
            .entries
            .iter()
            .filter(|(pid, _)| direct_children.is_empty() || direct_children.contains(pid))
            .map(|(pid, entry)| {
                ShardPlacementPolicyInputMapper::record_to_policy_input(*pid, entry)
            })
            .filter(|(_, entry)| entry.pool.as_str() == pool)
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
        let sharding = current_sharding_config()?.ok_or(ShardingPolicyError::ShardingDisabled)?;
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
