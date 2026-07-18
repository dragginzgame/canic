//! Module: workflow::placement::sharding::allocation
//!
//! Responsibility: create shard canisters and admit them into sharding storage.
//! Does not own: sharding policy, request endpoint authorization, or storage schema.
//! Boundary: invokes request ops and records successful shard allocation.

use crate::{
    InternalError,
    cdk::types::Principal,
    config::schema::ShardPoolPolicy,
    dto::rpc::CreateCanisterParent,
    ids::CanisterRole,
    log::Topic,
    model::replay::OperationId,
    ops::{
        ic::IcOps,
        rpc::request::RequestOps,
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
    workflow::placement::sharding::ShardingWorkflow,
};
use sha2::{Digest, Sha256};

const SHARD_ALLOCATION_OPERATION_DOMAIN: &[u8] = b"canic-shard-allocation:v1";

///
/// ShardAllocator
///
/// Internal helper for creating shard canisters before registry admission.
///
pub(super) struct ShardAllocator;

impl ShardAllocator {
    async fn allocate(
        pool: &str,
        slot: u32,
        canister_role: &CanisterRole,
        policy: &ShardPoolPolicy,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<Principal, InternalError> {
        MetricEvent::started(MetricOperation::CreateShard);
        if let Err(err) =
            ShardingRegistryOps::validate_new_shard(pool, slot, canister_role, policy.capacity)
        {
            MetricEvent::failed(MetricOperation::CreateShard, &err);
            return Err(err);
        }
        let operation_id = shard_allocation_operation_id(IcOps::canister_self(), pool, slot);

        let pid = match Self::create_canister_pid(canister_role, extra_arg, operation_id).await {
            Ok(pid) => pid,
            Err(err) => {
                MetricEvent::failed(MetricOperation::CreateShard, &err);
                return Err(err);
            }
        };
        let created_at = crate::ops::ic::IcOps::now_secs();
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
        operation_id: OperationId,
    ) -> Result<Principal, InternalError> {
        let response = RequestOps::create_canister_for_operation::<Vec<u8>>(
            canister_role,
            CreateCanisterParent::ThisCanister,
            extra_arg,
            operation_id,
        )
        .await?;

        Ok(response.new_canister_pid)
    }
}

fn shard_allocation_operation_id(owner: Principal, pool: &str, slot: u32) -> OperationId {
    let owner_bytes = owner.as_slice();
    let pool_bytes = pool.as_bytes();
    let mut hasher = Sha256::new();
    hasher.update(SHARD_ALLOCATION_OPERATION_DOMAIN);
    hasher.update(
        u64::try_from(owner_bytes.len())
            .expect("principal length must fit u64")
            .to_be_bytes(),
    );
    hasher.update(owner_bytes);
    hasher.update(
        u64::try_from(pool_bytes.len())
            .expect("validated pool length must fit u64")
            .to_be_bytes(),
    );
    hasher.update(pool_bytes);
    hasher.update(slot.to_be_bytes());
    OperationId::from_bytes(hasher.finalize().into())
}

impl ShardingWorkflow {
    pub(super) async fn allocate_and_admit(
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocation_operation_id_is_stable_and_resource_bound() {
        let owner = Principal::from_slice(&[1; 29]);
        let expected = shard_allocation_operation_id(owner, "poolA", 3);

        assert_eq!(shard_allocation_operation_id(owner, "poolA", 3), expected);
        assert_ne!(shard_allocation_operation_id(owner, "poolB", 3), expected);
        assert_ne!(shard_allocation_operation_id(owner, "poolA", 4), expected);
        assert_ne!(
            shard_allocation_operation_id(Principal::from_slice(&[2; 29]), "poolA", 3),
            expected
        );
    }
}
