//! Module: replay_policy::tests::pool_admin
//!
//! Responsibility: verify pool-admin replay-policy command coverage.
//! Does not own: pool workflow dispatch or manifest construction.
//! Boundary: test-only checks comparing source variants to manifest rows.

use super::*;
use std::collections::BTreeSet;

#[test]
fn pool_admin_command_variants_have_replay_policy_entries() {
    let variants = pool_admin_command_variant_names();
    let manifest = POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .map(|entry| entry.variant)
        .collect::<BTreeSet<_>>();

    assert_eq!(manifest, variants);
}

#[test]
fn pool_admin_endpoint_is_manifested_as_implemented_command_dispatch() {
    let entry = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.endpoint == "canic_pool_admin")
        .expect("pool admin endpoint policy entry");

    assert_eq!(
        entry.implementation_status,
        ReplayImplementationStatus::Implemented
    );
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::CommandDispatch {
            command_kind: replay_command_kind("pool.admin.v1"),
            command_manifest: "pool.admin.command_manifest.v1",
        }
    );
    assert_eq!(entry.cost_class, CostClass::ManagementDeployment);
    assert_eq!(entry.quota_policy, Some(DEPLOYMENT_QUOTA_V1));
    assert_eq!(entry.cycle_reserve_policy, Some(DEPLOYMENT_RESERVE_V1));
}

#[test]
fn pool_admin_endpoint_requires_all_command_variants_implemented() {
    let blockers = POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .filter(|entry| entry.implementation_status == ReplayImplementationStatus::ReleaseBlocker)
        .map(|entry| entry.variant)
        .collect::<Vec<_>>();

    assert!(
        blockers.is_empty(),
        "pool admin endpoint cannot be implemented while command variants remain blocked: {blockers:?}"
    );
}

#[test]
fn pool_create_empty_command_is_manifested_as_implemented() {
    let entry = POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.variant == "CreateEmpty")
        .expect("CreateEmpty command policy entry");

    assert_eq!(
        entry.implementation_status,
        ReplayImplementationStatus::Implemented
    );
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ReplayProtected {
            command_kind: replay_command_kind("pool.create_empty.v1"),
            requires_operation_id: true,
        }
    );
}

#[test]
fn pool_import_queued_command_is_manifested_as_implemented_convergent() {
    let entry = POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.variant == "ImportQueued")
        .expect("ImportQueued command policy entry");

    assert_eq!(
        entry.implementation_status,
        ReplayImplementationStatus::Implemented
    );
    assert_eq!(entry.cost_class, CostClass::None);
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::SnapshotConvergent {
            command_kind: replay_command_kind("pool.import_queued.ensure_v1"),
        }
    );
}

#[test]
fn pool_import_immediate_command_is_manifested_as_implemented_idempotent() {
    let entry = POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.variant == "ImportImmediate")
        .expect("ImportImmediate command policy entry");

    assert_eq!(
        entry.implementation_status,
        ReplayImplementationStatus::Implemented
    );
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ResponseIdempotent {
            command_kind: replay_command_kind("pool.import_immediate.ensure_v1"),
        }
    );
    assert_eq!(entry.cost_class, CostClass::ManagementDeployment);
    assert_eq!(entry.quota_policy, Some(DEPLOYMENT_QUOTA_V1));
    assert_eq!(entry.cycle_reserve_policy, Some(DEPLOYMENT_RESERVE_V1));
}

#[test]
fn pool_recycle_command_is_manifested_as_implemented_idempotent() {
    let entry = POOL_ADMIN_COMMAND_REPLAY_POLICY_MANIFEST
        .iter()
        .find(|entry| entry.variant == "Recycle")
        .expect("Recycle command policy entry");

    assert_eq!(
        entry.implementation_status,
        ReplayImplementationStatus::Implemented
    );
    assert_eq!(
        entry.replay_policy,
        ReplayPolicy::ResponseIdempotent {
            command_kind: replay_command_kind("pool.recycle.ensure_v1"),
        }
    );
    assert_eq!(entry.cost_class, CostClass::ManagementDeployment);
    assert_eq!(entry.quota_policy, Some(DEPLOYMENT_QUOTA_V1));
    assert_eq!(entry.cycle_reserve_policy, Some(DEPLOYMENT_RESERVE_V1));
}
