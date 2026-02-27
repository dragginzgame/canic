//! Async orchestration layer for sharding.
//!
//! Responsibilities:
//! - assemble sharding state (config + registry + metrics)
//! - delegate decisions to policy
//! - execute side effects (canister creation, registry mutation)
//!
//! This layer contains NO policy logic.

pub mod query;

use crate::{
    InternalError, InternalErrorOrigin,
    config::schema::{ShardPool, ShardPoolPolicy},
    domain::policy::placement::sharding::{
        CreateBlockedReason, ShardingPlanState, ShardingPolicy, ShardingPolicyError, ShardingState,
        hrw::HrwSelector,
    },
    dto::placement::sharding::ShardingPlanStateResponse,
    ops::{
        config::ConfigOps,
        ic::IcOps,
        placement::sharding::mapper::{
            ShardPartitionKeyAssignmentPolicyInputMapper, ShardPlacementPolicyInputMapper,
            ShardingPlanStateResponseMapper,
        },
        rpc::request::{CreateCanisterParent, RequestOps},
        storage::children::CanisterChildrenOps,
        storage::placement::{
            sharding::ShardingRegistryOps, sharding_lifecycle::ShardingLifecycleOps,
        },
    },
    view::placement::sharding::ShardPlacement,
    workflow::prelude::*,
};
use std::collections::BTreeSet;
use thiserror::Error as ThisError;

///
/// ShardingWorkflowError
///

#[derive(Debug, ThisError)]
pub enum ShardingWorkflowError {
    /// Policy rejected the operation (expected outcome).
    #[error(transparent)]
    Policy(#[from] ShardingPolicyError),

    /// Policy returned an internally inconsistent plan.
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

///
/// ShardAllocator
/// Allocates new shards when policy allows.
///

pub struct ShardAllocator;

impl ShardAllocator {
    /// Create and register a new shard.
    ///
    /// Assumes policy has already approved creation.
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

        log!(
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

///
/// ShardingWorkflow
/// High-level orchestration flows for partition_key assignment and rebalancing.
///

pub struct ShardingWorkflow;

impl ShardingWorkflow {
    /// Assign a partition_key to the given pool, creating a shard if necessary.
    pub(crate) async fn assign_to_pool(
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

    /// Assign a partition_key according to policy and HRW selection.
    pub(crate) async fn assign_with_policy(
        canister_role: &CanisterRole,
        pool: &str,
        partition_key: &str,
        policy: ShardPoolPolicy,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<Principal, InternalError> {
        // ---------------------------------------------------------------------
        // Assemble state
        // ---------------------------------------------------------------------

        let mut active = ShardingLifecycleOps::active_shards();
        if active.is_empty() {
            Self::bootstrap_empty_active(
                canister_role,
                pool,
                partition_key,
                &policy,
                extra_arg.clone(),
            )
            .await?;
            active = ShardingLifecycleOps::active_shards();
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

        let metrics = crate::domain::policy::placement::sharding::metrics::compute_pool_metrics(
            pool,
            &entry_views,
        );

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
            config: ShardPool {
                canister_role: canister_role.clone(),
                policy: policy.clone(),
            },
            metrics: &metrics,
            entries: &entry_views,
            assignments: &assignment_views,
        };

        // ---------------------------------------------------------------------
        // Policy decision
        // ---------------------------------------------------------------------

        let plan = ShardingPolicy::plan_assign(&state, partition_key, None);

        match plan.state {
            ShardingPlanState::AlreadyAssigned { pid } => {
                let slot = plan
                    .target_slot
                    .or_else(|| ShardingRegistryOps::slot_for_shard(pool, pid));

                log!(
                    Topic::Sharding,
                    Info,
                    "📦 partition_key={partition_key} already shard={pid} pool={pool} slot={slot:?}"
                );

                Ok(pid)
            }

            ShardingPlanState::UseExisting { pid } => {
                ShardingRegistryOps::assign(pool, partition_key, pid)?;

                let slot = plan
                    .target_slot
                    .or_else(|| ShardingRegistryOps::slot_for_shard(pool, pid));

                log!(
                    Topic::Sharding,
                    Info,
                    "📦 partition_key={partition_key} assigned shard={pid} pool={pool} slot={slot:?}"
                );

                Ok(pid)
            }

            ShardingPlanState::CreateAllowed => {
                let slot = plan.target_slot.ok_or({
                    ShardingWorkflowError::Invariant(
                        "sharding policy allowed creation but returned no slot",
                    )
                })?;

                let pid =
                    Self::allocate_and_admit(pool, slot, canister_role, &policy, extra_arg).await?;

                ShardingRegistryOps::assign(pool, partition_key, pid)?;

                log!(
                    Topic::Sharding,
                    Ok,
                    "✨ partition_key={partition_key} created+assigned shard={pid} pool={pool} slot={slot}"
                );

                Ok(pid)
            }

            ShardingPlanState::CreateBlocked { reason } => {
                Err(Self::blocked(reason, pool, partition_key))
            }
        }
    }

    /// Plan a partition_key assignment without mutating state.
    pub(crate) fn plan_assign_to_pool(
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

        let metrics = crate::domain::policy::placement::sharding::metrics::compute_pool_metrics(
            pool,
            &entry_views,
        );

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

    #[expect(clippy::cast_possible_truncation)]
    async fn bootstrap_empty_active(
        canister_role: &CanisterRole,
        pool: &str,
        partition_key: &str,
        policy: &ShardPoolPolicy,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<(), InternalError> {
        let pool_entries = Self::pool_entry_views(pool);
        if pool_entries.len() as u32 >= policy.max_shards {
            return Err(Self::no_active_shards_exhausted(pool, partition_key));
        }

        let free_slots = Self::free_slots(policy.max_shards, &pool_entries);
        let slot = HrwSelector::select_from_slots(pool, partition_key, &free_slots)
            .ok_or_else(|| Self::no_active_shards_exhausted(pool, partition_key))?;

        let _ = Self::allocate_and_admit(pool, slot, canister_role, policy, extra_arg).await?;

        Ok(())
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

    // Shard allocation is not sufficient for routability.
    // A shard becomes routable only after explicit lifecycle admission
    // performed by the sharding workflow.
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

    /// Convert a policy block reason into an error.
    fn blocked(reason: CreateBlockedReason, pool: &str, partition_key: &str) -> InternalError {
        ShardingWorkflowError::Policy(ShardingPolicyError::ShardCreationBlocked {
            reason,
            partition_key: partition_key.to_string(),
            pool: pool.to_string(),
        })
        .into()
    }

    /// Fetch shard pool configuration for the current canister.
    fn get_shard_pool_cfg(pool: &str) -> Result<ShardPool, InternalError> {
        let cfg = ConfigOps::current_canister()?;
        let sharding = cfg.sharding.ok_or(ShardingPolicyError::ShardingDisabled)?;

        sharding
            .pools
            .get(pool)
            .cloned()
            .ok_or_else(|| ShardingPolicyError::PoolNotFound(pool.to_string()))
            .map_err(InternalError::from)
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        InternalErrorClass, InternalErrorOrigin,
        cdk::candid::Principal,
        config::Config,
        dto::placement::sharding::ShardingPlanStateResponse,
        ids::CanisterRole,
        ops::storage::{
            children::CanisterChildrenOps,
            placement::{sharding::ShardingRegistryOps, sharding_lifecycle::ShardingLifecycleOps},
        },
    };
    use futures::executor::block_on;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn init_config() {
        crate::test::support::init_sharding_test_config();
    }

    fn activate(pid: Principal) {
        ShardingLifecycleOps::set_active(pid);
    }

    fn seed_direct_children(pids: &[Principal]) {
        let parent = p(250);
        let role = CanisterRole::from("shard");
        let entries = pids
            .iter()
            .copied()
            .map(|pid| (pid, role.clone()))
            .collect();
        CanisterChildrenOps::import_direct_children(parent, entries);
    }

    #[test]
    fn assign_returns_existing_when_already_assigned() {
        Config::reset_for_tests();
        init_config();
        ShardingRegistryOps::clear_for_test();
        ShardingLifecycleOps::clear_for_test();

        let shard = p(1);
        let role = CanisterRole::from("shard");
        let created_at = 0;

        seed_direct_children(&[shard]);
        ShardingRegistryOps::create(shard, "primary", 0, &role, 1, created_at).unwrap();
        activate(shard);
        ShardingRegistryOps::assign("primary", "partition_key-a", shard).unwrap();

        let pid = block_on(ShardingWorkflow::assign_to_pool(
            "primary",
            "partition_key-a",
        ))
        .unwrap();

        assert_eq!(pid, shard);
    }

    #[test]
    fn assign_uses_existing_shard_with_capacity() {
        Config::reset_for_tests();
        init_config();
        ShardingRegistryOps::clear_for_test();
        ShardingLifecycleOps::clear_for_test();

        let shard = p(1);
        let role = CanisterRole::from("shard");
        let created_at = 0;

        seed_direct_children(&[shard]);
        ShardingRegistryOps::create(shard, "primary", 0, &role, 2, created_at).unwrap();
        activate(shard);

        let pid = block_on(ShardingWorkflow::assign_to_pool(
            "primary",
            "partition_key-x",
        ))
        .unwrap();

        assert_eq!(pid, shard);
        assert_eq!(
            ShardingRegistryOps::partition_key_shard("primary", "partition_key-x"),
            Some(shard)
        );
    }

    #[test]
    fn assign_fails_when_active_empty_and_max_shards_exhausted() {
        Config::reset_for_tests();
        init_config();
        ShardingRegistryOps::clear_for_test();
        ShardingLifecycleOps::clear_for_test();

        let role = CanisterRole::from("shard");

        let shard_a = p(1);
        let shard_b = p(2);
        let created_at = 0;

        seed_direct_children(&[shard_a, shard_b]);
        ShardingRegistryOps::create(shard_a, "primary", 0, &role, 1, created_at).unwrap();
        ShardingRegistryOps::create(shard_b, "primary", 1, &role, 1, created_at).unwrap();

        let err = block_on(ShardingWorkflow::assign_to_pool(
            "primary",
            "partition_key-x",
        ))
        .unwrap_err();

        assert_eq!(err.class(), InternalErrorClass::Domain);
        assert_eq!(err.origin(), InternalErrorOrigin::Workflow);

        let msg = err.to_string();
        assert!(
            msg.contains("no active shards") && msg.contains("max_shards"),
            "unexpected error message: {msg}",
        );
    }

    #[test]
    fn assign_fails_when_pool_at_capacity_or_no_slots() {
        Config::reset_for_tests();
        init_config();
        ShardingRegistryOps::clear_for_test();
        ShardingLifecycleOps::clear_for_test();

        let role = CanisterRole::from("shard");

        let shard_a = p(1);
        let shard_b = p(2);
        let created_at = 0;

        seed_direct_children(&[shard_a, shard_b]);
        ShardingRegistryOps::create(shard_a, "primary", 0, &role, 1, created_at).unwrap();
        ShardingRegistryOps::create(shard_b, "primary", 1, &role, 1, created_at).unwrap();
        activate(shard_a);
        activate(shard_b);

        ShardingRegistryOps::assign("primary", "a", shard_a).unwrap();
        ShardingRegistryOps::assign("primary", "b", shard_b).unwrap();

        let err = block_on(ShardingWorkflow::assign_to_pool("primary", "c")).unwrap_err();

        assert_eq!(err.class(), InternalErrorClass::Domain);
        assert_eq!(err.origin(), InternalErrorOrigin::Domain);

        let msg = err.to_string();
        assert!(
            msg.contains("no free slots")
                || msg.contains("no free shard slots")
                || msg.contains("pool at capacity"),
            "unexpected error message: {msg}",
        );
    }

    #[test]
    fn admit_shard_marks_active_set() {
        Config::reset_for_tests();
        init_config();
        ShardingRegistryOps::clear_for_test();
        ShardingLifecycleOps::clear_for_test();

        let shard = p(1);
        let role = CanisterRole::from("shard");
        let created_at = 0;

        seed_direct_children(&[shard]);
        ShardingRegistryOps::create(shard, "primary", 0, &role, 1, created_at).unwrap();
        assert!(ShardingLifecycleOps::active_shards().is_empty());

        ShardingWorkflow::admit_shard(shard);

        let active = ShardingLifecycleOps::active_shards();
        assert_eq!(active, vec![shard]);
    }

    #[test]
    fn admission_required_for_routing() {
        Config::reset_for_tests();
        init_config();
        ShardingRegistryOps::clear_for_test();
        ShardingLifecycleOps::clear_for_test();

        let shard = p(1);
        let role = CanisterRole::from("shard");
        let created_at = 0;

        seed_direct_children(&[shard]);
        ShardingRegistryOps::create(shard, "primary", 0, &role, 1, created_at).unwrap();

        let plan = ShardingWorkflow::plan_assign_to_pool("primary", "partition_key-x").unwrap();
        assert!(
            matches!(plan, ShardingPlanStateResponse::CreateAllowed),
            "expected create-allowed when no shards are admitted"
        );

        ShardingWorkflow::admit_shard(shard);

        let plan = ShardingWorkflow::plan_assign_to_pool("primary", "partition_key-x").unwrap();
        assert_eq!(plan, ShardingPlanStateResponse::UseExisting { pid: shard });
    }

    #[test]
    fn plan_ignores_non_child_assigned_shard() {
        Config::reset_for_tests();
        init_config();
        ShardingRegistryOps::clear_for_test();
        ShardingLifecycleOps::clear_for_test();
        seed_direct_children(&[p(201)]);

        let stale = p(200);
        let role = CanisterRole::from("shard");
        let created_at = 0;

        ShardingRegistryOps::create(stale, "primary", 0, &role, 1, created_at).unwrap();
        activate(stale);
        ShardingRegistryOps::assign("primary", "partition_key-stale", stale).unwrap();

        let plan = ShardingWorkflow::plan_assign_to_pool("primary", "partition_key-stale").unwrap();
        assert_eq!(plan, ShardingPlanStateResponse::CreateAllowed);
    }
}
