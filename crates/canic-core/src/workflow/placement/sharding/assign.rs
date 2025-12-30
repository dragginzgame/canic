//! Async orchestration layer for sharding.
//!
//! Handles tenant assignment, shard creation, and draining.
//! Depends on [`policy`] for validation and [`registry`] for state.

use crate::{
    Error,
    cdk::types::Principal,
    config::schema::{ShardPool, ShardPoolPolicy},
    dto::rpc::CreateCanisterParent,
    ids::CanisterRole,
    log,
    log::Topic,
    ops::{rpc::create_canister_request, storage::sharding::ShardingRegistryOps},
    policy::placement::sharding::{
        ShardingPolicyError,
        metrics::pool_metrics,
        policy::{ShardingPlanState, ShardingPolicy},
    },
};

///
/// ShardAllocator
/// Allocates new shards when policy allows.
///

pub(crate) struct ShardAllocator;

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

        let response = create_canister_request::<Vec<u8>>(
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
/// ShardingOps
/// High-level orchestration flows for tenant assignment and rebalancing.
///

pub struct ShardingOps;

impl ShardingOps {
    /// Plan a tenant assignment without mutating state.
    pub fn plan_assign_to_pool(
        pool: &str,
        tenant: impl AsRef<str>,
    ) -> Result<ShardingPlanState, Error> {
        let plan = ShardingPolicy::plan_assign_to_pool(pool, tenant)?;

        Ok(plan.state)
    }

    /// Assign a tenant to the given pool, creating a shard if necessary.
    pub async fn assign_to_pool(pool: &str, tenant: impl AsRef<str>) -> Result<Principal, Error> {
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
    pub async fn assign_with_policy(
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

    /// Drain up to `limit` tenants from a shard into others or new shards.
    pub async fn drain_shard(
        pool: &str,
        donor_shard_pid: Principal,
        limit: u32,
    ) -> Result<u32, Error> {
        let pool_cfg = Self::get_shard_pool_cfg(pool)?;
        let tenants = ShardingRegistryOps::tenants_in_shard(pool, donor_shard_pid);
        let mut moved = 0u32;

        for tenant in tenants.iter().take(limit as usize) {
            // Let the normal policy decide where this tenant should go.
            let plan = ShardingPolicy::plan_reassign_from_shard(pool, tenant, donor_shard_pid)?;
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

                ShardingPlanState::AlreadyAssigned { pid } if pid != donor_shard_pid => {
                    log!(
                        Topic::Sharding,
                        Info,
                        "🚰 tenant={tenant} already moved donor={donor_shard_pid} shard={pid}"
                    );
                }

                ShardingPlanState::CreateAllowed => {
                    let slot = plan.target_slot.ok_or_else(|| {
                        ShardingPolicyError::ShardCreationBlocked(
                            "missing slot when draining shard".into(),
                        )
                    })?;

                    let new_pid = ShardAllocator::allocate(
                        pool,
                        slot,
                        &pool_cfg.canister_role,
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
        config::Config,
        ids::{CanisterRole, SubnetRole},
        ops::{runtime::env::EnvOps, storage::sharding::ShardingRegistryOps},
    };
    use candid::Principal;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn init_config() {
        let toml = r#"
            [subnets.prime.canisters.manager]
            cardinality = "single"
            initial_cycles = "5T"

            [subnets.prime.canisters.manager.sharding.pools.primary]
            canister_role = "shard"
            [subnets.prime.canisters.manager.sharding.pools.primary.policy]
            capacity = 2
            max_shards = 3

            [subnets.prime.canisters.shard]
            cardinality = "many"
            initial_cycles = "5T"
        "#;

        Config::init_from_toml(toml).unwrap();
        EnvOps::set_subnet_role(SubnetRole::PRIME);
        EnvOps::set_canister_role(CanisterRole::from("manager"));
    }

    #[test]
    fn drain_shard_moves_tenant_to_other_shard() {
        Config::reset_for_tests();
        init_config();
        ShardingRegistryOps::clear_for_test();

        let shard_role = CanisterRole::from("shard");
        let shard_a = p(1);
        let shard_b = p(2);

        ShardingRegistryOps::create(shard_a, "primary", 0, &shard_role, 2).unwrap();
        ShardingRegistryOps::create(shard_b, "primary", 1, &shard_role, 2).unwrap();
        ShardingRegistryOps::assign("primary", "tenant-a", shard_a).unwrap();
        ShardingRegistryOps::assign("primary", "tenant-b", shard_a).unwrap();

        let moved =
            futures::executor::block_on(ShardingOps::drain_shard("primary", shard_a, 1)).unwrap();
        assert_eq!(moved, 1);

        let entry_a = ShardingRegistryOps::get(shard_a).unwrap();
        let entry_b = ShardingRegistryOps::get(shard_b).unwrap();
        assert_eq!(entry_a.count, 1);
        assert_eq!(entry_b.count, 1);

        let tenant_a = ShardingRegistryOps::tenant_shard("primary", "tenant-a").unwrap();
        let tenant_b = ShardingRegistryOps::tenant_shard("primary", "tenant-b").unwrap();
        assert!(tenant_a == shard_b || tenant_b == shard_b);

        Config::reset_for_tests();
    }
}
