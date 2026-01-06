//! Pure, deterministic rules for shard placement and capacity planning.
//!
//! This module contains no storage access, no configuration loading,
//! and no side effects. All required state is provided explicitly
//! by callers (typically query/workflow code).

mod backfill;
pub mod hrw;
pub mod metrics;

use crate::{
    Error, ThisError,
    cdk::candid::Principal,
    config::schema::{ShardPool, ShardPoolPolicy},
    domain::policy::PolicyError,
    ops::storage::placement::sharding::{ShardEntry, ShardKey},
};
use backfill::plan_slot_backfill;
use hrw::HrwSelector;
use metrics::PoolMetrics;

///
/// ShardingPolicyError
/// Policy error types
///

#[derive(Debug, thiserror::Error)]
pub enum ShardingPolicyError {
    #[error("shard pool not found: {0}")]
    PoolNotFound(String),

    #[error("shard creation blocked: {0}")]
    ShardCreationBlocked(String),

    #[error("sharding disabled")]
    ShardingDisabled,
}

impl From<ShardingPolicyError> for Error {
    fn from(err: ShardingPolicyError) -> Self {
        PolicyError::ShardingPolicy(err).into()
    }
}

///
/// ShardingState
/// Snapshot of sharding state required by policy.
///
///  NOTE:
/// `assignments` MUST be pre-scoped to this pool by the caller.
/// Policy logic assumes pool isolation and will not re-filter.
///

pub struct ShardingState<'a> {
    pub pool: &'a str,
    pub config: ShardPool,
    pub metrics: &'a PoolMetrics,
    pub entries: &'a [(Principal, ShardEntry)],
    pub assignments: &'a [(ShardKey, Principal)], // tenants for *this pool only*
}

///
/// ShardingPlan
/// Result of a dry-run shard assignment plan.
///

#[derive(Clone, Debug)]
pub struct ShardingPlan {
    pub state: ShardingPlanState,
    pub target_slot: Option<u32>,
    pub utilization_pct: u32,
    pub active_count: u32,
    pub total_capacity: u64,
    pub total_used: u64,
}

///
/// ShardingPlanState
/// Outcome variants of a shard plan.
///

#[derive(Clone, Debug)]
pub enum ShardingPlanState {
    AlreadyAssigned { pid: Principal },
    UseExisting { pid: Principal },
    CreateAllowed,
    CreateBlocked { reason: CreateBlockedReason },
}

///
/// CreateBlockedReason
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum CreateBlockedReason {
    #[error("pool at capacity")]
    PoolAtCapacity,

    #[error("no free shard slots")]
    NoFreeSlots,

    #[error("{0}")]
    PolicyViolation(String),
}
///
/// ShardingPolicy
///

pub struct ShardingPolicy;

impl ShardingPolicy {
    /// Pure capacity check.
    #[must_use]
    pub const fn can_create(metrics: &PoolMetrics, policy: &ShardPoolPolicy) -> bool {
        metrics.active_count < policy.max_shards
    }

    /// Lookup the shard assigned to a tenant, if any.
    /// Invariant: `assignments` contains only entries for the active pool.
    #[must_use]
    pub(crate) fn lookup_tenant(
        tenant: &str,
        assignments: &[(ShardKey, Principal)],
    ) -> Option<Principal> {
        assignments
            .iter()
            .find(|(key, _)| key.tenant.as_ref() == tenant)
            .map(|(_, pid)| *pid)
    }

    /// Perform a dry-run assignment plan.
    pub(crate) fn plan_assign(
        state: &ShardingState,
        tenant: &str,
        exclude_pid: Option<Principal>,
    ) -> ShardingPlan {
        let pool_cfg = state.config.clone();
        let metrics = state.metrics;
        let entries = state.entries;

        let slot_plan = plan_slot_backfill(state.pool, entries, pool_cfg.policy.max_shards);

        if let Some(pid) =
            Self::lookup_tenant(tenant, state.assignments).filter(|pid| exclude_pid != Some(*pid))
        {
            let slot = slot_plan.slots.get(&pid).copied();

            return Self::make_plan(ShardingPlanState::AlreadyAssigned { pid }, metrics, slot);
        }

        // Prefer an existing shard with spare capacity.
        let shards_with_capacity: Vec<_> = entries
            .iter()
            .filter(|(pid, entry)| {
                entry.pool == state.pool && entry_has_capacity(entry) && exclude_pid != Some(*pid)
            })
            .map(|(pid, _)| *pid)
            .collect();

        if let Some(target_pid) = HrwSelector::select(tenant, &shards_with_capacity) {
            let slot = slot_plan.slots.get(&target_pid).copied();

            return Self::make_plan(
                ShardingPlanState::UseExisting { pid: target_pid },
                metrics,
                slot,
            );
        }

        let max_slots = pool_cfg.policy.max_shards;
        let free_slots: Vec<u32> = (0..max_slots)
            .filter(|slot| !slot_plan.occupied.contains(slot))
            .collect();

        // Slot selection is independent of create eligibility; we still compute a target slot
        // so callers can distinguish "no slots exist" from "policy forbids creating one".
        let Some(target_slot) = HrwSelector::select_from_slots(state.pool, tenant, &free_slots)
        else {
            return Self::make_plan(
                ShardingPlanState::CreateBlocked {
                    reason: CreateBlockedReason::NoFreeSlots,
                },
                metrics,
                None,
            );
        };

        if Self::can_create(metrics, &pool_cfg.policy) {
            Self::make_plan(ShardingPlanState::CreateAllowed, metrics, Some(target_slot))
        } else {
            Self::make_plan(
                ShardingPlanState::CreateBlocked {
                    reason: CreateBlockedReason::PoolAtCapacity,
                },
                metrics,
                Some(target_slot),
            )
        }
    }

    const fn make_plan(
        state: ShardingPlanState,
        metrics: &PoolMetrics,
        slot: Option<u32>,
    ) -> ShardingPlan {
        ShardingPlan {
            state,
            target_slot: slot,
            utilization_pct: metrics.utilization_pct,
            active_count: metrics.active_count,
            total_capacity: metrics.total_capacity,
            total_used: metrics.total_used,
        }
    }
}

const fn entry_has_capacity(entry: &ShardEntry) -> bool {
    entry.count < entry.capacity
}
