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

use super::hrw::HrwSelector;
use crate::{
    Error,
    config::model::{ShardPool, ShardPoolPolicy},
    memory::ext::sharding::{PoolMetrics, ShardingRegistry, ShardingRegistryView},
    ops::{context::cfg_current_canister, ext::sharding::ShardingError},
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

///
/// ShardingPlan
/// Result of a dry-run shard assignment plan.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ShardingPlan {
    pub state: ShardingPlanState,
    pub utilization_pct: u32,
    pub active_count: u32,
    pub total_capacity: u64,
    pub total_used: u64,
}

/// Outcome variants of a dry-run shard plan.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum ShardingPlanState {
    /// Tenant already has a shard assigned.
    AlreadyAssigned { pid: Principal },

    /// Tenant can be deterministically assigned to an existing shard (via HRW).
    UseExisting { pid: Principal },

    /// Policy allows creation of a new shard.
    CreateAllowed,

    /// Policy forbids creation of a new shard (e.g., capacity reached).
    CreateBlocked { reason: String },
}

///
/// ShardingPolicyOps
///

pub struct ShardingPolicyOps;

impl ShardingPolicyOps {
    // -----------------------------------------------------------------------
    // Validation
    // -----------------------------------------------------------------------

    /// Validate whether a pool may create a new shard under its policy.
    #[inline]
    pub fn check_create_allowed(
        metrics: &PoolMetrics,
        policy: &ShardPoolPolicy,
    ) -> Result<(), Error> {
        if metrics.active_count >= policy.max_shards {
            Err(ShardingError::ShardCapReached.into())
        } else {
            Ok(())
        }
    }

    // -----------------------------------------------------------------------
    // Configuration Access
    // -----------------------------------------------------------------------

    /// Retrieve the shard pool configuration from the current canister’s config.
    pub fn get_pool_config(pool: &str) -> Result<ShardPool, Error> {
        let cfg = cfg_current_canister()?;
        let sharding_cfg = cfg.sharding.ok_or(ShardingError::ShardingDisabled)?;
        let pool_cfg = sharding_cfg
            .pools
            .get(pool)
            .ok_or_else(|| ShardingError::PoolNotFound(pool.to_string()))?
            .clone();

        Ok(pool_cfg)
    }

    // -----------------------------------------------------------------------
    // Planning
    // -----------------------------------------------------------------------

    /// Perform a dry-run plan for assigning a tenant to a shard.
    /// Never creates or mutates registry state.
    pub fn plan_assign_to_pool(pool: &str, tenant: Principal) -> Result<ShardingPlan, Error> {
        let metrics = ShardingRegistry::metrics(pool);

        // Case 1: Tenant already assigned → nothing to do
        if let Some(pid) = ShardingRegistry::tenant_shard(pool, tenant) {
            return Ok(Self::make_plan(
                ShardingPlanState::AlreadyAssigned { pid },
                &metrics,
            ));
        }

        // Case 2: Try to reuse an existing shard using HRW.
        // This ensures deterministic placement even if pool size changes over time
        let active_shards = ShardingRegistry::list_active_shards(pool);
        if !active_shards.is_empty()
            && let Some(pid) = HrwSelector::select(&tenant, &active_shards)
        {
            return Ok(Self::make_plan(
                ShardingPlanState::UseExisting { pid },
                &metrics,
            ));
        }

        // Case 3: No shards or none suitable → check policy for creation
        let pool_cfg = Self::get_pool_config(pool)?;
        match Self::check_create_allowed(&metrics, &pool_cfg.policy) {
            Ok(()) => Ok(Self::make_plan(ShardingPlanState::CreateAllowed, &metrics)),
            Err(e) => Ok(Self::make_plan(
                ShardingPlanState::CreateBlocked {
                    reason: e.to_string(),
                },
                &metrics,
            )),
        }
    }

    // -----------------------------------------------------------------------
    // Registry Access Helpers
    // -----------------------------------------------------------------------

    /// Export a read-only view of the sharding registry.
    #[must_use]
    pub fn export_registry() -> ShardingRegistryView {
        ShardingRegistry::export()
    }

    /// Lookup the shard assigned to a tenant, if any.
    #[must_use]
    pub fn lookup_tenant(pool: &str, tenant_pid: Principal) -> Option<Principal> {
        ShardingRegistry::tenant_shard(pool, tenant_pid)
    }

    /// Lookup the shard assigned to a tenant, returning an error if none exists.
    pub fn try_lookup_tenant(pool: &str, tenant_pid: Principal) -> Result<Principal, Error> {
        Self::lookup_tenant(pool, tenant_pid)
            .ok_or_else(|| ShardingError::TenantNotFound(tenant_pid).into())
    }

    // -----------------------------------------------------------------------
    // Utilities
    // -----------------------------------------------------------------------

    /// Internal helper to construct a plan from metrics and state.
    const fn make_plan(state: ShardingPlanState, metrics: &PoolMetrics) -> ShardingPlan {
        ShardingPlan {
            state,
            utilization_pct: metrics.utilization_pct,
            active_count: metrics.active_count,
            total_capacity: metrics.total_capacity,
            total_used: metrics.total_used,
        }
    }
}

/// ===========================================================================
/// Tests
/// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;

    #[test]
    fn check_create_allowed_blocks_when_at_capacity() {
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
        assert!(ShardingPolicyOps::check_create_allowed(&metrics, &policy).is_err());
    }

    #[test]
    fn plan_returns_already_assigned_if_tenant_exists() {
        let tenant = Principal::anonymous();
        let plan = ShardingPlan {
            state: ShardingPlanState::AlreadyAssigned { pid: tenant },
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
}
