use crate::{
    Error,
    config::Config,
    memory::PartitionRegistry,
    ops::{canister::create_and_install_canister, prelude::*, request::create_canister_request},
};
use candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// PartitionPolicy
///

#[derive(Clone, Copy, Debug)]
pub struct PartitionPolicy {
    pub initial_capacity: u32,
    pub max_partitions: u32,
    pub growth_threshold_bps: u32, // e.g., 8000 = 80%
}

///
/// PartitionPlan
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct PartitionPlan {
    pub state: PartitionPlanState,
    pub utilization_bps: u32,
    pub active_count: u32,
    pub total_capacity: u64,
    pub total_used: u64,
}

///
/// PartitionPlanState
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum PartitionPlanState {
    AlreadyAssigned { pid: Principal },
    UseExisting { pid: Principal },
    CreateAllowed,
    CreateBlocked { reason: String },
}

/// Ensure an item is assigned to a partition; create a new partition canister on demand
/// respecting the provided policy.
async fn ensure_item_assignment_internal(
    canister_type: &CanisterType,
    item: Principal,
    policy: PartitionPolicy,
    initial_capacity: u32,
    extra_arg: Option<Vec<u8>>,
) -> Result<Principal, Error> {
    // If already assigned return
    if let Some(pid) = PartitionRegistry::get_item_partition(&item) {
        crate::log!(
            crate::Log::Info,
            "🗄️  partition: already assigned item={item} -> {pid}",
        );
        return Ok(pid);
    }

    // Try existing partitions
    if let Some(pid) = PartitionRegistry::assign_item_best_effort(item) {
        crate::log!(
            crate::Log::Info,
            "🗄️  partition: assigned existing item={item} -> {pid}",
        );
        return Ok(pid);
    }

    // Evaluate whether to create a new partition based on policy
    let view = PartitionRegistry::export();
    let mut active_count = 0u32;
    let mut total_capacity = 0u64;
    let mut total_used = 0u64;
    for (_pid, e) in &view {
        if e.capacity > 0 {
            active_count += 1;
            total_capacity += u64::from(e.capacity);
            total_used += u64::from(e.count);
        }
    }

    let utilization_bps: u32 = if total_capacity == 0 {
        0
    } else {
        (total_used.saturating_mul(10_000) / total_capacity) as u32
    };

    if active_count >= policy.max_partitions {
        crate::log!(
            crate::Log::Warn,
            "🗄️  partition: creation blocked (cap reached) util={}bps active={} max={} cap={} used={}",
            utilization_bps,
            active_count,
            policy.max_partitions,
            total_capacity,
            total_used
        );
        return Err(Error::custom("partition cap reached"));
    }

    // Create only if above threshold or no capacity exists
    if utilization_bps < policy.growth_threshold_bps && total_capacity > 0 {
        crate::log!(
            crate::Log::Info,
            "🗄️  partition: below growth threshold util={}bps < {}bps (cap={}, used={})",
            utilization_bps,
            policy.growth_threshold_bps,
            total_capacity,
            total_used
        );
        return Err(Error::custom("no capacity and below growth threshold"));
    }

    // Create and register a new partition canister
    crate::log!(
        crate::Log::Info,
        "🗄️  partition: creating new canister type={} util={}bps active={} of max={} (cap={}, used={})",
        canister_type,
        utilization_bps,
        active_count,
        policy.max_partitions,
        total_capacity,
        total_used
    );
    let pid = if crate::memory::CanisterState::is_root() {
        // Root can create directly
        let parents = vec![crate::memory::canister::CanisterEntry::this()?];
        create_and_install_canister(canister_type, &parents, extra_arg).await?
    } else {
        // Non-root: send request to root
        crate::log!(
            crate::Log::Info,
            "🗄️  partition: requesting root to create canister type={canister_type}",
        );
        let res = create_canister_request::<()>(canister_type, None).await?;
        res.new_canister_pid
    };

    PartitionRegistry::register(pid, initial_capacity);
    crate::log!(
        crate::Log::Ok,
        "🗄️  partition: created {pid} with capacity={initial_capacity} (registered)",
    );

    // Try assignment again (prefer the newly created partition)
    if PartitionRegistry::assign_item_to_partition(item, pid).is_err() {
        let pid2 = PartitionRegistry::assign_item_best_effort(item)
            .ok_or_else(|| Error::custom("failed to assign after creation"))?;
        crate::log!(
            crate::Log::Info,
            "🗄️  partition: assigned after creation via fallback item={item} -> {pid2}",
        );
        return Ok(pid2);
    }

    crate::log!(
        crate::Log::Ok,
        "🗄️  partition: assigned item={item} -> {pid}",
    );

    Ok(pid)
}

/// Ensure an item is assigned, using explicit policy and capacity.
pub async fn ensure_item_assignment(
    canister_type: &CanisterType,
    item: Principal,
    policy: PartitionPolicy,
    initial_capacity: u32,
    extra_arg: Option<Vec<u8>>,
) -> Result<Principal, Error> {
    ensure_item_assignment_internal(canister_type, item, policy, initial_capacity, extra_arg).await
}

/// Convenience: derive policy/capacity from icu.toml for the given hub and partition types.
pub async fn ensure_item_assignment_from_config(
    partition_type: &CanisterType,
    hub_type: &CanisterType,
    item: Principal,
) -> Result<Principal, Error> {
    // Hub policy and capacity (children do not read config)
    let part = Config::try_get_canister(hub_type)
        .ok()
        .and_then(|c| c.partition)
        .ok_or_else(|| Error::custom("partitioning disabled"))?;

    let policy = PartitionPolicy {
        initial_capacity: part.initial_capacity,
        max_partitions: part.max_partitions,
        growth_threshold_bps: part.growth_threshold_bps,
    };

    ensure_item_assignment_internal(partition_type, item, policy, part.initial_capacity, None).await
}

/// Short alias: assign using config.
pub async fn assign_with_config(
    partition_type: &CanisterType,
    hub_type: &CanisterType,
    item: Principal,
) -> Result<Principal, Error> {
    ensure_item_assignment_from_config(partition_type, hub_type, item).await
}

/// Short alias: assign with explicit policy/capacity.
pub async fn assign_with_policy(
    canister_type: &CanisterType,
    item: Principal,
    policy: PartitionPolicy,
    initial_capacity: u32,
) -> Result<Principal, Error> {
    ensure_item_assignment_internal(canister_type, item, policy, initial_capacity, None).await
}

/// Dry-run (plan) using config: never creates; returns current metrics and decision.
pub fn plan_with_config(hub_type: &CanisterType, item: Principal) -> Result<PartitionPlan, Error> {
    // Already assigned?
    if let Some(pid) = PartitionRegistry::get_item_partition(&item) {
        return Ok(PartitionPlan {
            state: PartitionPlanState::AlreadyAssigned { pid },
            utilization_bps: 0,
            active_count: 0,
            total_capacity: 0,
            total_used: 0,
        });
    }

    // Existing candidate? (peek only; do not mutate state)
    if let Some(pid) = PartitionRegistry::peek_best_effort() {
        return Ok(PartitionPlan {
            state: PartitionPlanState::UseExisting { pid },
            utilization_bps: 0,
            active_count: 0,
            total_capacity: 0,
            total_used: 0,
        });
    }

    // Policy from config; require Some to consider partitioning enabled
    let part = Config::try_get_canister(hub_type)
        .ok()
        .and_then(|c| c.partition)
        .ok_or_else(|| Error::custom("partitioning disabled"))?;
    let max_partitions = part.max_partitions;
    let growth_threshold_bps = part.growth_threshold_bps;

    // Metrics
    let view = PartitionRegistry::export();
    let mut active_count = 0u32;
    let mut total_capacity = 0u64;
    let mut total_used = 0u64;
    for (_pid, e) in &view {
        if e.capacity > 0 {
            active_count += 1;
            total_capacity += u64::from(e.capacity);
            total_used += u64::from(e.count);
        }
    }
    let utilization_bps: u32 = if total_capacity == 0 {
        0
    } else {
        (total_used.saturating_mul(10_000) / total_capacity) as u32
    };

    if active_count >= max_partitions {
        return Ok(PartitionPlan {
            state: PartitionPlanState::CreateBlocked {
                reason: "partition cap reached".to_string(),
            },
            utilization_bps,
            active_count,
            total_capacity,
            total_used,
        });
    }

    if utilization_bps < growth_threshold_bps && total_capacity > 0 {
        return Ok(PartitionPlan {
            state: PartitionPlanState::CreateBlocked {
                reason: "below growth threshold".to_string(),
            },
            utilization_bps,
            active_count,
            total_capacity,
            total_used,
        });
    }

    Ok(PartitionPlan {
        state: PartitionPlanState::CreateAllowed,
        utilization_bps,
        active_count,
        total_capacity,
        total_used,
    })
}

/// auto_register_from_config
/// Auto-register this canister as a partition if config contains a
/// partition block
pub fn auto_register_from_config() {
    // Only non-root canisters should auto-register
    if crate::memory::CanisterState::is_root() {
        return;
    }

    // Determine this canister's type
    let Some(ty) = crate::memory::CanisterState::get_type() else {
        return;
    };

    // Read capacity from config if a partition block exists
    if let Ok(c) = Config::try_get_canister(&ty)
        && let Some(p) = c.partition
    {
        let cap = p.initial_capacity;
        let me = crate::cdk::api::canister_self();

        PartitionRegistry::register(me, cap);

        crate::log!(
            crate::Log::Ok,
            "partition: auto-registered {ty} capacity={cap}"
        );
    }
}
