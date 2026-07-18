//! Module: workflow::placement::sharding::allocation
//!
//! Responsibility: create shard canisters and admit them into sharding storage.
//! Does not own: sharding policy, request endpoint authorization, or storage schema.
//! Boundary: invokes request ops and records successful shard allocation.

use crate::{
    InternalError,
    cdk::types::Principal,
    config::schema::ShardPoolPolicy,
    ids::CanisterRole,
    log::Topic,
    model::placement::allocation::PlacementAllocationIdentity,
    ops::{
        ic::IcOps,
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
    workflow::placement::{
        allocation::{PlacementAllocationRequest, PlacementAllocationWorkflow},
        sharding::ShardingWorkflow,
    },
};

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
        let owner = IcOps::canister_self();
        let identity_probe = PlacementAllocationIdentity::sharding(
            owner,
            pool,
            slot,
            0,
            canister_role,
            extra_arg.as_deref(),
        );
        let generation = PlacementAllocationWorkflow::next_sequence(&identity_probe);
        let identity = PlacementAllocationIdentity::sharding(
            owner,
            pool,
            slot,
            generation,
            canister_role,
            extra_arg.as_deref(),
        );
        let reservation_limit =
            PlacementAllocationWorkflow::reservation_limit_for_available_capacity(&identity, 1);
        let (permit, pid) =
            match PlacementAllocationWorkflow::create_child(PlacementAllocationRequest {
                identity,
                canister_role: canister_role.clone(),
                extra_arg,
                reservation_limit,
            })
            .await
            {
                Ok(result) => result,
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
        ShardingLifecycleOps::set_active(pid);
        if let Err(err) = PlacementAllocationWorkflow::finish_registered_child(&permit, pid) {
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
}

impl ShardingWorkflow {
    pub(super) async fn allocate_and_admit(
        pool: &str,
        slot: u32,
        canister_role: &CanisterRole,
        policy: &ShardPoolPolicy,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<Principal, InternalError> {
        ShardAllocator::allocate(pool, slot, canister_role, policy, extra_arg).await
    }
}
