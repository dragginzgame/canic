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
    ops::{
        rpc::request::{CreateCanisterParent, RequestOps},
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

        let pid = match Self::create_canister_pid(canister_role, extra_arg).await {
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
