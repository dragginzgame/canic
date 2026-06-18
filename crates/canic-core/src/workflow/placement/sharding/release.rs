//! Module: workflow::placement::sharding::release
//!
//! Responsibility: release partition-key assignments from configured shard pools.
//! Does not own: sharding configuration, registry schema, or cleanup scheduling.
//! Boundary: validates pool configuration before mutating assignment storage.

use crate::{
    InternalError,
    cdk::types::Principal,
    log::Topic,
    ops::{
        runtime::metrics::{
            recording::ShardingMetricEvent as MetricEvent,
            sharding::{
                ShardingMetricOperation as MetricOperation, ShardingMetricReason as MetricReason,
            },
        },
        storage::placement::sharding::ShardingRegistryOps,
    },
    workflow::placement::sharding::ShardingWorkflow,
};

impl ShardingWorkflow {
    /// Release (unassign) a partition_key from its shard, freeing shard
    /// capacity and decrementing the shard's load counter. Returns the shard the
    /// key was assigned to, or `None` if it had no assignment. The inverse of
    /// [`Self::assign_to_pool`]; intended for eviction / reclamation of stale or
    /// never-completed assignments by current Canic pool-owner code.
    pub fn release_partition_key(
        pool: &str,
        partition_key: impl AsRef<str>,
    ) -> Result<Option<Principal>, InternalError> {
        MetricEvent::started(MetricOperation::ReleaseKey);
        if let Err(err) = Self::get_shard_pool_cfg(pool) {
            MetricEvent::failed(MetricOperation::ReleaseKey, &err);
            return Err(err);
        }

        let partition_key = partition_key.as_ref();
        let released = match ShardingRegistryOps::release(pool, partition_key) {
            Ok(released) => released,
            Err(err) => {
                MetricEvent::failed(MetricOperation::ReleaseKey, &err);
                return Err(err);
            }
        };

        if let Some(shard) = released {
            crate::log!(
                Topic::Sharding,
                Info,
                "📦 partition_key={partition_key} released shard={shard} pool={pool}"
            );
            MetricEvent::completed(MetricOperation::ReleaseKey, MetricReason::Ok);
            Ok(Some(shard))
        } else {
            MetricEvent::skipped(MetricOperation::ReleaseKey, MetricReason::NotAssigned);
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        InternalErrorClass, InternalErrorOrigin, ids::CanisterRole,
        test::support::init_sharding_test_config,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn release_partition_key_requires_configured_pool_before_storage_mutation() {
        init_sharding_test_config();
        ShardingRegistryOps::clear_for_test();
        let role = CanisterRole::new("shard");
        let shard = p(42);

        ShardingRegistryOps::create(shard, "stale", 0, &role, 2, 0).unwrap();
        ShardingRegistryOps::assign("stale", "pk1", shard).unwrap();

        let err = ShardingWorkflow::release_partition_key("stale", "pk1")
            .expect_err("unknown pool must fail before storage mutation");

        assert_eq!(err.class(), InternalErrorClass::Domain);
        assert_eq!(err.origin(), InternalErrorOrigin::Domain);
        assert_eq!(
            ShardingRegistryOps::partition_key_shard("stale", "pk1"),
            Some(shard)
        );
        assert_eq!(ShardingRegistryOps::get(shard).unwrap().count, 1);
    }

    #[test]
    fn release_partition_key_releases_configured_pool_assignment() {
        init_sharding_test_config();
        ShardingRegistryOps::clear_for_test();
        let role = CanisterRole::new("shard");
        let shard = p(7);

        ShardingRegistryOps::create(shard, "primary", 0, &role, 2, 0).unwrap();
        ShardingRegistryOps::assign("primary", "pk1", shard).unwrap();

        assert_eq!(
            ShardingWorkflow::release_partition_key("primary", "pk1").unwrap(),
            Some(shard)
        );
        assert_eq!(
            ShardingRegistryOps::partition_key_shard("primary", "pk1"),
            None
        );
        assert_eq!(ShardingRegistryOps::get(shard).unwrap().count, 0);
        assert_eq!(
            ShardingWorkflow::release_partition_key("primary", "pk1").unwrap(),
            None
        );
    }
}
