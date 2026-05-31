use super::ShardingWorkflow;
use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    config::schema::{ShardPool, ShardPoolPolicy},
    domain::policy::placement::sharding::HrwSelector,
    ids::CanisterRole,
    log::Topic,
    ops::{
        config::ConfigOps,
        runtime::metrics::{
            recording::ShardingMetricEvent as MetricEvent,
            sharding::{
                ShardingMetricOperation as MetricOperation, ShardingMetricOutcome as MetricOutcome,
                ShardingMetricReason as MetricReason,
            },
        },
        storage::placement::sharding::ShardingRegistryOps,
    },
};

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

    // Assign the first shard in an empty pool and persist the initial partition mapping.
    pub(super) async fn assign_bootstrap_created(
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
    pub(super) fn ensure_bootstrap_capacity(
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

    fn no_active_shards_exhausted(pool: &str, partition_key: &str) -> InternalError {
        InternalError::domain(
            InternalErrorOrigin::Workflow,
            format!(
                "no active shards in pool '{pool}' and max_shards exhausted; cannot assign partition_key '{partition_key}'"
            ),
        )
    }
}
