//! Async orchestration layer for sharding.
//!
//! Handles tenant assignment, shard creation, and draining.
//! Depends on [`policy`] for validation and [`registry`] for state.

use super::policy::{ShardingPlanState, ShardingPolicyOps};
use crate::{
    Error,
    config::schema::{ShardPool, ShardPoolPolicy},
    log,
    log::Topic,
    model::memory::sharding::ShardingRegistry,
    ops::{
        config::ConfigOps,
        model::memory::sharding::{ShardingOpsError, ShardingRegistryOps, pool_metrics},
        request::{CreateCanisterParent, create_canister_request},
    },
    types::CanisterType,
};
use candid::Principal;

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
        canister_type: &CanisterType,
        policy: &ShardPoolPolicy,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<Principal, Error> {
        let metrics = pool_metrics(pool);
        ShardingPolicyOps::check_create_allowed(&metrics, policy)?;

        let response = create_canister_request::<Vec<u8>>(
            canister_type,
            CreateCanisterParent::ThisCanister,
            extra_arg,
        )
        .await?;
        let pid = response.new_canister_pid;

        ShardingRegistryOps::create(pid, pool, slot, canister_type, policy.capacity)?;
        log!(
            Topic::Sharding,
            Ok,
            "✨ shard.create: {pid} pool={pool} slot={slot}"
        );

        Ok(pid)
    }
}

///
/// ShardingOps
/// High-level orchestration flows for tenant assignment and rebalancing.
///

pub struct ShardingOps;

impl ShardingOps {
    /// Assign a tenant to the given pool, creating a shard if necessary.
    pub async fn assign_to_pool<S: ToString>(pool: &str, tenant: S) -> Result<Principal, Error> {
        let pool_cfg = Self::get_shard_pool_cfg(pool)?;
        Self::assign_with_policy(
            &pool_cfg.canister_type,
            pool,
            &tenant.to_string(),
            pool_cfg.policy,
            None,
        )
        .await
    }

    /// Assign a tenant according to pool policy and HRW selection.
    pub async fn assign_with_policy(
        canister_type: &CanisterType,
        pool: &str,
        tenant: &str,
        policy: ShardPoolPolicy,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<Principal, Error> {
        // Step 1: Determine plan via HRW-based policy
        let plan = ShardingPolicyOps::plan_assign_to_pool(pool, tenant)?;

        match plan.state {
            ShardingPlanState::AlreadyAssigned { pid } => {
                let slot = plan
                    .target_slot
                    .or_else(|| ShardingRegistry::slot_for_shard(pool, pid));
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
                    .or_else(|| ShardingRegistry::slot_for_shard(pool, pid));
                log!(
                    Topic::Sharding,
                    Info,
                    "📦 tenant={tenant} assigned shard={pid} pool={pool} slot={slot:?}"
                );

                Ok(pid)
            }

            ShardingPlanState::CreateAllowed => {
                let slot = plan.target_slot.ok_or_else(|| {
                    ShardingOpsError::ShardCreationBlocked(
                        "missing target slot in allocation plan".into(),
                    )
                })?;
                let pid =
                    ShardAllocator::allocate(pool, slot, canister_type, &policy, extra_arg).await?;
                ShardingRegistryOps::assign(pool, tenant, pid)?;
                log!(
                    Topic::Sharding,
                    Ok,
                    "✨ tenant={tenant} created+assigned shard={pid} pool={pool} slot={slot}"
                );

                Ok(pid)
            }

            ShardingPlanState::CreateBlocked { reason } => {
                Err(ShardingOpsError::ShardCreationBlocked(reason).into())
            }
        }
    }

    /// Drain up to `limit` tenants from a shard into others or new shards.
    pub async fn drain_shard(
        pool: &str,
        donor_shard_pid: Principal,
        limit: u32,
    ) -> Result<u32, Error> {
        let pool_cfg = Self::get_shard_pool_cfg(pool)?;
        let tenants = ShardingRegistry::tenants_in_shard(pool, donor_shard_pid);
        let mut moved = 0u32;

        for tenant in tenants.iter().take(limit as usize) {
            // Let the normal policy decide where this tenant should go.
            let plan = ShardingPolicyOps::plan_assign_to_pool(pool, tenant)?;
            match plan.state {
                ShardingPlanState::UseExisting { pid } if pid != donor_shard_pid => {
                    ShardingRegistryOps::assign(pool, tenant, pid)?;
                    log!(
                        Topic::Sharding,
                        Info,
                        "🚰 drained tenant={tenant} donor={donor_shard_pid} → shard={pid}"
                    );
                    moved += 1;
                }

                ShardingPlanState::CreateAllowed => {
                    let slot = plan.target_slot.ok_or_else(|| {
                        ShardingOpsError::ShardCreationBlocked(
                            "missing slot when draining shard".into(),
                        )
                    })?;

                    let new_pid = ShardAllocator::allocate(
                        pool,
                        slot,
                        &pool_cfg.canister_type,
                        &pool_cfg.policy,
                        None,
                    )
                    .await?;

                    ShardingRegistryOps::assign(pool, tenant, new_pid)?;

                    log!(
                        Topic::Sharding,
                        Ok,
                        "✨ shard.create: {new_pid} draining donor={donor_shard_pid} slot={slot}"
                    );
                    moved += 1;
                }
                _ => {}
            }
        }

        Ok(moved)
    }

    /// Internal: fetch shard pool config for the current canister.
    fn get_shard_pool_cfg(pool: &str) -> Result<ShardPool, Error> {
        let cfg = ConfigOps::current_canister()?;
        let sharding_cfg = cfg.sharding.ok_or(ShardingOpsError::ShardingDisabled)?;
        let pool_cfg = sharding_cfg
            .pools
            .get(pool)
            .ok_or_else(|| ShardingOpsError::PoolNotFound(pool.to_string()))?
            .clone();

        Ok(pool_cfg)
    }
}
