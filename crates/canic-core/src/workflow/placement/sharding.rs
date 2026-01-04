//! Async orchestration layer for sharding.
//!
//! Handles tenant assignment, shard creation, and draining.
//! Depends on [`policy`] for validation and [`registry`] for state.

use crate::{
    Error,
    config::schema::{ShardPool, ShardPoolPolicy},
    domain::policy::placement::sharding::{
        ShardingPolicyError,
        metrics::pool_metrics,
        policy::{ShardingPlanState, ShardingPolicy},
    },
    dto::{placement::ShardingPlanStateView, rpc::CreateCanisterParent},
    ops::{rpc::request::RequestOps, storage::placement::sharding::ShardingRegistryOps},
    workflow::{placement::mapper::PlacementMapper, prelude::*},
};

///
/// ShardAllocator
/// Allocates new shards when policy allows.
///

pub struct ShardAllocator;

impl ShardAllocator {
    /// Create a new shard in the given pool if policy allows.
    pub async fn allocate(
        pool: &str,
        slot: u32,
        canister_role: &CanisterRole,
        policy: &ShardPoolPolicy,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<Principal, Error> {
        let metrics = pool_metrics(pool);
        if !ShardingPolicy::can_create(&metrics, policy) {
            return Err(ShardingPolicyError::ShardCreationBlocked(format!(
                "shard cap reached for pool {pool}"
            ))
            .into());
        }

        let response = RequestOps::create_canister::<Vec<u8>>(
            canister_role,
            CreateCanisterParent::ThisCanister,
            extra_arg,
        )
        .await?;
        let pid = response.new_canister_pid;

        ShardingRegistryOps::create(pid, pool, slot, canister_role, policy.capacity)?;
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
    ) -> Result<Principal, Error> {
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

    /// Assign a tenant according to pool policy and HRW selection.
    pub(crate) async fn assign_with_policy(
        canister_role: &CanisterRole,
        pool: &str,
        tenant: &str,
        policy: ShardPoolPolicy,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<Principal, Error> {
        // Step 1: Determine plan via HRW-based policy
        let plan = ShardingPolicy::plan_assign_to_pool(pool, tenant)?;

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
                let slot = plan.target_slot.ok_or_else(|| {
                    ShardingPolicyError::ShardCreationBlocked(
                        "missing target slot in allocation plan".into(),
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

            ShardingPlanState::CreateBlocked { reason } => {
                Err(ShardingPolicyError::ShardCreationBlocked(reason.to_string()).into())
            }
        }
    }

    /// Plan a tenant assignment without mutating state.
    pub(crate) fn plan_assign_to_pool(
        pool: &str,
        tenant: impl AsRef<str>,
    ) -> Result<ShardingPlanStateView, Error> {
        let plan = ShardingPolicy::plan_assign_to_pool(pool, tenant)?;

        Ok(PlacementMapper::sharding_plan_state_to_view(plan.state))
    }

    /// Internal: fetch shard pool config for the current canister.
    fn get_shard_pool_cfg(pool: &str) -> Result<ShardPool, Error> {
        ShardingPolicy::get_pool_config(pool)
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config, ids::CanisterRole, ops::storage::placement::sharding::ShardingRegistryOps,
    };
    use candid::Principal;
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

        ShardingRegistryOps::create(shard, "primary", 0, &role, 1).unwrap();
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

        ShardingRegistryOps::create(shard, "primary", 0, &role, 2).unwrap();

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

        ShardingRegistryOps::create(shard_a, "primary", 0, &role, 1).unwrap();
        ShardingRegistryOps::create(shard_b, "primary", 1, &role, 1).unwrap();

        ShardingRegistryOps::assign("primary", "a", shard_a).unwrap();
        ShardingRegistryOps::assign("primary", "b", shard_b).unwrap();

        let err = block_on(ShardingWorkflow::assign_to_pool("primary", "c")).unwrap_err();

        let msg = err.to_string();
        assert!(
            msg.contains("no free slots") || msg.contains("shard cap reached"),
            "unexpected error: {msg}"
        );
    }
}
