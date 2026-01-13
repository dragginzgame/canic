//! Async orchestration layer for sharding.
//!
//! Responsibilities:
//! - assemble sharding state (config + registry + metrics)
//! - delegate decisions to policy
//! - execute side effects (canister creation, registry mutation)
//!
//! This layer contains NO policy logic.

pub mod mapper;
pub mod query;

use crate::{
    InternalError, InternalErrorOrigin,
    config::schema::{ShardPool, ShardPoolPolicy},
    domain::policy::placement::sharding::{
        CreateBlockedReason, ShardingPlanState, ShardingPolicy, ShardingPolicyError, ShardingState,
    },
    dto::placement::sharding::ShardingPlanStateView,
    ops::{
        config::ConfigOps,
        ic::IcOps,
        rpc::request::{CreateCanisterParent, RequestOps},
        storage::placement::sharding::ShardingRegistryOps,
    },
    workflow::{placement::sharding::mapper::ShardingMapper, prelude::*},
};
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
        let response = RequestOps::create_canister::<Vec<u8>>(
            canister_role,
            CreateCanisterParent::ThisCanister,
            extra_arg,
        )
        .await?;

        let pid = response.new_canister_pid;

        let created_at = IcOps::now_secs();
        ShardingRegistryOps::create(pid, pool, slot, canister_role, policy.capacity, created_at)?;

        log!(
            Topic::Sharding,
            Ok,
            "✨ shard.create: {pid} pool={pool} slot={slot}"
        );

        Ok(pid)
    }
}

///
/// ShardingWorkflow
/// High-level orchestration flows for tenant assignment and rebalancing.
///

pub struct ShardingWorkflow;

impl ShardingWorkflow {
    /// Assign a tenant to the given pool, creating a shard if necessary.
    pub(crate) async fn assign_to_pool(
        pool: &str,
        tenant: impl AsRef<str>,
    ) -> Result<Principal, InternalError> {
        let pool_cfg = Self::get_shard_pool_cfg(pool)?;
        Self::assign_with_policy(
            &pool_cfg.canister_role,
            pool,
            tenant.as_ref(),
            pool_cfg.policy,
            None,
        )
        .await
    }

    /// Assign a tenant according to policy and HRW selection.
    pub(crate) async fn assign_with_policy(
        canister_role: &CanisterRole,
        pool: &str,
        tenant: &str,
        policy: ShardPoolPolicy,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<Principal, InternalError> {
        // ---------------------------------------------------------------------
        // Assemble state
        // ---------------------------------------------------------------------

        let registry = ShardingRegistryOps::export();

        let metrics = crate::domain::policy::placement::sharding::metrics::compute_pool_metrics(
            pool,
            &registry.entries,
        );

        let assignments = ShardingRegistryOps::assignments_for_pool(pool);

        let state = ShardingState {
            pool,
            config: ShardPool {
                canister_role: canister_role.clone(),
                policy: policy.clone(),
            },
            metrics: &metrics,
            entries: &registry.entries,
            assignments: &assignments,
        };

        // ---------------------------------------------------------------------
        // Policy decision
        // ---------------------------------------------------------------------

        let plan = ShardingPolicy::plan_assign(&state, tenant, None);

        match plan.state {
            ShardingPlanState::AlreadyAssigned { pid } => {
                let slot = plan
                    .target_slot
                    .or_else(|| ShardingRegistryOps::slot_for_shard(pool, pid));

                log!(
                    Topic::Sharding,
                    Info,
                    "📦 tenant={tenant} already shard={pid} pool={pool} slot={slot:?}"
                );

                Ok(pid)
            }

            ShardingPlanState::UseExisting { pid } => {
                ShardingRegistryOps::assign(pool, tenant, pid)?;

                let slot = plan
                    .target_slot
                    .or_else(|| ShardingRegistryOps::slot_for_shard(pool, pid));

                log!(
                    Topic::Sharding,
                    Info,
                    "📦 tenant={tenant} assigned shard={pid} pool={pool} slot={slot:?}"
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
                    ShardAllocator::allocate(pool, slot, canister_role, &policy, extra_arg).await?;

                ShardingRegistryOps::assign(pool, tenant, pid)?;

                log!(
                    Topic::Sharding,
                    Ok,
                    "✨ tenant={tenant} created+assigned shard={pid} pool={pool} slot={slot}"
                );

                Ok(pid)
            }

            ShardingPlanState::CreateBlocked { reason } => Err(Self::blocked(reason, pool, tenant)),
        }
    }

    /// Plan a tenant assignment without mutating state.
    pub(crate) fn plan_assign_to_pool(
        pool: &str,
        tenant: impl AsRef<str>,
    ) -> Result<ShardingPlanStateView, InternalError> {
        let registry = ShardingRegistryOps::export();

        let metrics = crate::domain::policy::placement::sharding::metrics::compute_pool_metrics(
            pool,
            &registry.entries,
        );

        let assignments = ShardingRegistryOps::assignments_for_pool(pool);

        let pool_cfg = Self::get_shard_pool_cfg(pool)?;

        let state = ShardingState {
            pool,
            config: pool_cfg,
            metrics: &metrics,
            entries: &registry.entries,
            assignments: &assignments,
        };

        let plan = ShardingPolicy::plan_assign(&state, tenant.as_ref(), None);

        Ok(ShardingMapper::sharding_plan_state_to_view(plan.state))
    }

    /// Convert a policy block reason into an error.
    fn blocked(reason: CreateBlockedReason, pool: &str, tenant: &str) -> InternalError {
        ShardingWorkflowError::Policy(ShardingPolicyError::ShardCreationBlocked {
            reason,
            tenant: tenant.to_string(),
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
        InternalErrorClass, cdk::candid::Principal, config::Config, ids::CanisterRole,
        ops::storage::placement::sharding::ShardingRegistryOps,
    };
    use futures::executor::block_on;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn init_config() {
        crate::test::support::init_sharding_test_config();
    }

    #[test]
    fn assign_returns_existing_when_already_assigned() {
        Config::reset_for_tests();
        init_config();
        ShardingRegistryOps::clear_for_test();

        let shard = p(1);
        let role = CanisterRole::from("shard");
        let created_at = 0;

        ShardingRegistryOps::create(shard, "primary", 0, &role, 1, created_at).unwrap();
        ShardingRegistryOps::assign("primary", "tenant-a", shard).unwrap();

        let pid = block_on(ShardingWorkflow::assign_to_pool("primary", "tenant-a")).unwrap();

        assert_eq!(pid, shard);
    }

    #[test]
    fn assign_uses_existing_shard_with_capacity() {
        Config::reset_for_tests();
        init_config();
        ShardingRegistryOps::clear_for_test();

        let shard = p(1);
        let role = CanisterRole::from("shard");
        let created_at = 0;

        ShardingRegistryOps::create(shard, "primary", 0, &role, 2, created_at).unwrap();

        let pid = block_on(ShardingWorkflow::assign_to_pool("primary", "tenant-x")).unwrap();

        assert_eq!(pid, shard);
        assert_eq!(
            ShardingRegistryOps::tenant_shard("primary", "tenant-x"),
            Some(shard)
        );
    }

    #[test]
    fn assign_fails_when_pool_at_capacity_or_no_slots() {
        Config::reset_for_tests();
        init_config();
        ShardingRegistryOps::clear_for_test();

        let role = CanisterRole::from("shard");

        let shard_a = p(1);
        let shard_b = p(2);
        let created_at = 0;

        ShardingRegistryOps::create(shard_a, "primary", 0, &role, 1, created_at).unwrap();
        ShardingRegistryOps::create(shard_b, "primary", 1, &role, 1, created_at).unwrap();

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
}
