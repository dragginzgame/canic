//! Module: replay_policy::pool_admin_manifest
//!
//! Responsibility: record replay policy for pool-admin command variants.
//! Does not own: pool workflow dispatch, canister management, or replay storage.
//! Boundary: command manifest rows consumed by replay policy tests and workflows.

use crate::replay_policy::{
    quota::{DEPLOYMENT_QUOTA_V1, DEPLOYMENT_RESERVE_V1},
    types::{CostClass, PoolAdminCommandReplayPolicy, ReplayImplementationStatus, ReplayPolicy},
};

/// Canonical replay-policy rows for `PoolAdminCommand` variants.
pub const POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST: &[PoolAdminCommandReplayPolicy] = &[
    pool_admin_replay_protected(
        "CreateEmpty",
        "pool.create_empty.v1",
        ReplayImplementationStatus::Implemented,
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    pool_admin_response_idempotent(
        "Recycle",
        "pool.recycle.ensure_v1",
        ReplayImplementationStatus::Implemented,
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    pool_admin_response_idempotent(
        "ImportImmediate",
        "pool.import_immediate.ensure_v1",
        ReplayImplementationStatus::Implemented,
        CostClass::ManagementDeployment,
        Some(DEPLOYMENT_QUOTA_V1),
        Some(DEPLOYMENT_RESERVE_V1),
    ),
    pool_admin_snapshot_convergent(
        "ImportQueued",
        "pool.import_queued.ensure_v1",
        ReplayImplementationStatus::Implemented,
        CostClass::None,
        None,
        None,
    ),
];

/// Returns the canonical pool-admin command replay-policy manifest.
#[must_use]
pub const fn pool_admin_command_replay_policy_manifest() -> &'static [PoolAdminCommandReplayPolicy]
{
    POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST
}

const fn pool_admin_response_idempotent(
    variant: &'static str,
    command_kind: &'static str,
    implementation_status: ReplayImplementationStatus,
    cost_class: CostClass,
    quota_policy: Option<&'static str>,
    cycle_reserve_policy: Option<&'static str>,
) -> PoolAdminCommandReplayPolicy {
    PoolAdminCommandReplayPolicy {
        variant,
        replay_policy: ReplayPolicy::ResponseIdempotent { command_kind },
        implementation_status,
        cost_class,
        quota_policy,
        cycle_reserve_policy,
    }
}

const fn pool_admin_replay_protected(
    variant: &'static str,
    command_kind: &'static str,
    implementation_status: ReplayImplementationStatus,
    cost_class: CostClass,
    quota_policy: Option<&'static str>,
    cycle_reserve_policy: Option<&'static str>,
) -> PoolAdminCommandReplayPolicy {
    PoolAdminCommandReplayPolicy {
        variant,
        replay_policy: ReplayPolicy::ReplayProtected {
            command_kind,
            requires_operation_id: true,
        },
        implementation_status,
        cost_class,
        quota_policy,
        cycle_reserve_policy,
    }
}

const fn pool_admin_snapshot_convergent(
    variant: &'static str,
    command_kind: &'static str,
    implementation_status: ReplayImplementationStatus,
    cost_class: CostClass,
    quota_policy: Option<&'static str>,
    cycle_reserve_policy: Option<&'static str>,
) -> PoolAdminCommandReplayPolicy {
    PoolAdminCommandReplayPolicy {
        variant,
        replay_policy: ReplayPolicy::SnapshotConvergent { command_kind },
        implementation_status,
        cost_class,
        quota_policy,
        cycle_reserve_policy,
    }
}
