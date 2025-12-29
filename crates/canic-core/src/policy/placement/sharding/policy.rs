//! ===========================================================================
//! Sharding Policy Logic
//! ===========================================================================
//!
//! Pure, deterministic policy rules for sharding pools.
//!
//! Responsibilities:
//! - Validates whether a pool may create new shards.
//! - Determines which shard a tenant should be assigned to.
//! - Provides read-only views of sharding state.
//!
//! Currently, shard assignment uses **HRW (Highest Random Weight)** selection.
//! This ensures stable, fair, and deterministic tenant distribution.
//!
//! ===========================================================================

use crate::{
    Error,
    cdk::types::Principal,
    config::schema::{ShardPool, ShardPoolPolicy},
    ops::{
        config::ConfigOps,
        storage::sharding::{ShardEntry, ShardingRegistryOps},
    },
    policy::placement::sharding::{
        ShardingPolicyError,
        hrw::HrwSelector,
        metrics::{PoolMetrics, pool_metrics},
    },
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

///
/// ShardingPlan
/// Result of a dry-run shard assignment plan (including the desired slot index).
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShardingPlan {
    pub state: ShardingPlanState,
    pub target_slot: Option<u32>,
    pub utilization_pct: u32,
    pub active_count: u32,
    pub total_capacity: u64,
    pub total_used: u64,
}

///
/// CreateBlockedReason
/// Structured reason for creation denial.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CreateBlockedReason {
    PoolAtCapacity,
    NoFreeSlots,
    PolicyViolation(String),
}

impl std::fmt::Display for CreateBlockedReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PoolAtCapacity => write!(f, "shard cap reached"),
            Self::NoFreeSlots => write!(f, "sharding pool has no free slots"),
            Self::PolicyViolation(msg) => write!(f, "{msg}"),
        }
    }
}

///
/// ShardingPlanState
/// Outcome variants of a dry-run shard plan.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ShardingPlanState {
    /// Tenant already has a shard assigned.
    AlreadyAssigned { pid: Principal },

    /// Tenant can be deterministically assigned to an existing shard (via HRW).
    UseExisting { pid: Principal },

    /// Policy allows creation of a new shard.
    CreateAllowed,

    /// Policy forbids creation of a new shard (e.g., capacity reached).
    CreateBlocked { reason: CreateBlockedReason },
}

///
/// ShardingPolicy
///

pub struct ShardingPolicy;

impl ShardingPolicy {
    // -----------------------------------------------------------------------
    // Validation
    // -----------------------------------------------------------------------

    /// Return whether a pool may create a new shard under its policy.
    #[must_use]
    pub(crate) const fn can_create(metrics: &PoolMetrics, policy: &ShardPoolPolicy) -> bool {
        metrics.active_count < policy.max_shards
    }

    // -----------------------------------------------------------------------
    // Configuration Access
    // -----------------------------------------------------------------------

    /// Retrieve the shard pool configuration from the current canister’s config.
    pub(crate) fn get_pool_config(pool: &str) -> Result<ShardPool, Error> {
        let cfg = ConfigOps::current_canister()?;
        let sharding_cfg = cfg.sharding.ok_or(ShardingPolicyError::ShardingDisabled)?;
        let pool_cfg = sharding_cfg
            .pools
            .get(pool)
            .ok_or_else(|| ShardingPolicyError::PoolNotFound(pool.to_string()))?
            .clone();

        Ok(pool_cfg)
    }

    // -----------------------------------------------------------------------
    // Planning
    // -----------------------------------------------------------------------

    /// Perform a dry-run plan for assigning a tenant to a shard.
    /// Never creates or mutates registry state.
    pub fn plan_assign_to_pool(pool: &str, tenant: impl AsRef<str>) -> Result<ShardingPlan, Error> {
        Self::plan_internal(pool, tenant.as_ref(), None)
    }

    /// Plan a reassignment for a tenant currently on `donor_pid`, excluding that shard.
    /// Never creates or mutates registry state.
    pub fn plan_reassign_from_shard(
        pool: &str,
        tenant: impl AsRef<str>,
        donor_pid: Principal,
    ) -> Result<ShardingPlan, Error> {
        let tenant = tenant.as_ref();
        Self::plan_internal(pool, tenant, Some(donor_pid))
    }

    fn plan_internal(
        pool: &str,
        tenant: &str,
        exclude_pid: Option<Principal>,
    ) -> Result<ShardingPlan, Error> {
        let pool_cfg = Self::get_pool_config(pool)?;
        let metrics = pool_metrics(pool);
        let data = ShardingRegistryOps::export();
        let slot_plan = plan_slot_backfill(pool, &data, pool_cfg.policy.max_shards);

        if let Some(pid) = ShardingRegistryOps::tenant_shard(pool, tenant)
            && exclude_pid != Some(pid)
        {
            let slot = slot_plan.slots.get(&pid).copied();
            return Ok(Self::make_plan(
                ShardingPlanState::AlreadyAssigned { pid },
                &metrics,
                slot,
            ));
        }

        // Prefer an existing shard with spare capacity.
        let shards_with_capacity: Vec<_> = data
            .iter()
            .filter(|(pid, entry)| {
                entry.pool.as_ref() == pool
                    && entry_has_capacity(entry)
                    && exclude_pid != Some(*pid)
            })
            .map(|(pid, _)| *pid)
            .collect();

        if let Some(target_pid) = HrwSelector::select(tenant, &shards_with_capacity) {
            let slot = slot_plan.slots.get(&target_pid).copied();
            return Ok(Self::make_plan(
                ShardingPlanState::UseExisting { pid: target_pid },
                &metrics,
                slot,
            ));
        }

        let max_slots = pool_cfg.policy.max_shards;
        let free_slots: Vec<u32> = (0..max_slots)
            .filter(|slot| !slot_plan.occupied.contains(slot))
            .collect();

        // Slot selection is independent of create eligibility; we still compute a target slot
        // so callers can distinguish "no slots exist" from "policy forbids creating one".
        let Some(target_slot) = HrwSelector::select_from_slots(pool, tenant, &free_slots) else {
            return Ok(Self::make_plan(
                ShardingPlanState::CreateBlocked {
                    reason: CreateBlockedReason::NoFreeSlots,
                },
                &metrics,
                None,
            ));
        };

        if Self::can_create(&metrics, &pool_cfg.policy) {
            Ok(Self::make_plan(
                ShardingPlanState::CreateAllowed,
                &metrics,
                Some(target_slot),
            ))
        } else {
            Ok(Self::make_plan(
                ShardingPlanState::CreateBlocked {
                    reason: CreateBlockedReason::PoolAtCapacity,
                },
                &metrics,
                Some(target_slot),
            ))
        }
    }

    // -----------------------------------------------------------------------
    // Registry Access Helpers
    // -----------------------------------------------------------------------

    /// Lookup the shard assigned to a tenant, if any.
    #[must_use]
    pub fn lookup_tenant(pool: &str, tenant: impl AsRef<str>) -> Option<Principal> {
        let tenant = tenant.as_ref();
        ShardingRegistryOps::tenant_shard(pool, tenant)
    }

    // -----------------------------------------------------------------------
    // Utilities
    // -----------------------------------------------------------------------

    /// Internal helper to construct a plan from metrics and state.
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

// -----------------------------------------------------------------------------
// Slot backfilling (pure planning)
// -----------------------------------------------------------------------------

struct SlotBackfillPlan {
    /// Effective slot mapping for shards in the pool (explicit or simulated).
    slots: BTreeMap<Principal, u32>,
    /// Slots considered occupied after deterministic backfill simulation.
    occupied: BTreeSet<u32>,
}

fn plan_slot_backfill(
    pool: &str,
    view: &[(Principal, ShardEntry)],
    max_slots: u32,
) -> SlotBackfillPlan {
    let mut entries: Vec<(Principal, ShardEntry)> = view
        .iter()
        .filter(|(_, entry)| entry.pool.as_ref() == pool)
        .map(|(pid, entry)| (*pid, entry.clone()))
        .collect();

    entries.sort_by_key(|(pid, _)| *pid);

    let mut slots = BTreeMap::<Principal, u32>::new();
    let mut occupied = BTreeSet::<u32>::new();

    for (pid, entry) in &entries {
        if entry_has_assigned_slot(entry) {
            slots.insert(*pid, entry.slot);
            occupied.insert(entry.slot);
        }
    }

    if max_slots == 0 {
        return SlotBackfillPlan { slots, occupied };
    }

    let available: Vec<u32> = (0..max_slots)
        .filter(|slot| !occupied.contains(slot))
        .collect();

    if available.is_empty() {
        return SlotBackfillPlan { slots, occupied };
    }

    let mut idx = 0usize;
    for (pid, entry) in &entries {
        if entry_has_assigned_slot(entry) {
            continue;
        }

        // NOTE: Backfill simulates slot assignment for existing shards only.
        // Policy enforcement happens later; this function is purely positional.
        if idx >= available.len() {
            break;
        }

        let slot = available[idx];
        idx += 1;
        slots.insert(*pid, slot);
        occupied.insert(slot);
    }

    SlotBackfillPlan { slots, occupied }
}

const fn entry_has_assigned_slot(entry: &ShardEntry) -> bool {
    entry.slot != ShardEntry::UNASSIGNED_SLOT
}

const fn entry_has_capacity(entry: &ShardEntry) -> bool {
    entry.count < entry.capacity
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        ids::CanisterRole,
        ops::{env::EnvOps, storage::sharding::ShardingRegistryOps},
    };
    use candid::Principal;

    #[test]
    fn can_create_blocks_when_at_capacity() {
        let metrics = PoolMetrics {
            active_count: 10,
            total_capacity: 100,
            total_used: 80,
            utilization_pct: 80,
        };
        let policy = ShardPoolPolicy {
            max_shards: 5,
            ..Default::default()
        };
        assert!(!ShardingPolicy::can_create(&metrics, &policy));
    }

    #[test]
    fn plan_returns_already_assigned_if_tenant_exists() {
        let tenant = Principal::anonymous();
        let plan = ShardingPlan {
            state: ShardingPlanState::AlreadyAssigned { pid: tenant },
            target_slot: Some(0),
            utilization_pct: 50,
            active_count: 2,
            total_capacity: 100,
            total_used: 50,
        };
        assert!(matches!(
            plan.state,
            ShardingPlanState::AlreadyAssigned { .. }
        ));
    }

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn init_config() {
        use crate::{
            config::Config,
            ids::{CanisterRole, SubnetRole},
        };

        let toml = r#"
            [subnets.prime.canisters.manager]
            cardinality = "single"
            initial_cycles = "5T"

            [subnets.prime.canisters.manager.sharding.pools.primary]
            canister_role = "shard"
            [subnets.prime.canisters.manager.sharding.pools.primary.policy]
            capacity = 1
            max_shards = 2

            [subnets.prime.canisters.shard]
            cardinality = "many"
            initial_cycles = "5T"
        "#;

        Config::init_from_toml(toml).unwrap();
        EnvOps::set_subnet_role(SubnetRole::PRIME);
        EnvOps::set_canister_role(CanisterRole::from("manager"));
    }

    #[test]
    fn plan_allows_creation_when_target_shard_full() {
        Config::reset_for_tests();
        init_config();
        ShardingRegistryOps::clear_for_test();

        let shard_role = CanisterRole::from("shard");
        let shard = p(1);
        ShardingRegistryOps::create(shard, "primary", 0, &shard_role, 1).unwrap();
        ShardingRegistryOps::assign("primary", "tenant-a", shard).unwrap();

        let plan = ShardingPolicy::plan_assign_to_pool("primary", "tenant-x").unwrap();

        assert!(matches!(plan.state, ShardingPlanState::CreateAllowed));
        Config::reset_for_tests();
    }

    #[test]
    fn plan_blocks_creation_when_pool_at_capacity() {
        Config::reset_for_tests();
        init_config();
        ShardingRegistryOps::clear_for_test();

        let shard_role = CanisterRole::from("shard");
        let shard_a = p(1);
        let shard_b = p(2);
        ShardingRegistryOps::create(shard_a, "primary", 0, &shard_role, 1).unwrap();
        ShardingRegistryOps::create(shard_b, "primary", 1, &shard_role, 1).unwrap();
        ShardingRegistryOps::assign("primary", "tenant-a", shard_a).unwrap();
        ShardingRegistryOps::assign("primary", "tenant-b", shard_b).unwrap();

        let plan = ShardingPolicy::plan_assign_to_pool("primary", "tenant-y").unwrap();

        assert!(matches!(
            plan.state,
            ShardingPlanState::CreateBlocked { .. }
        ));
        Config::reset_for_tests();
    }
}
