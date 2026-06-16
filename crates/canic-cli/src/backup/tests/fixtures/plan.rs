//! Module: backup::tests::fixtures::plan
//!
//! Responsibility: build backup plan test fixtures.
//! Does not own: backup create persistence or execution journal fixtures.
//! Boundary: deterministic plan values for backup command tests.

use super::{CHILD, HASH, ROOT};
use canic_backup::{
    manifest::IdentityMode,
    plan::{
        AuthorityEvidence, BackupPlan, BackupPlanBuildInput, BackupScopeKind, ControlAuthority,
        SnapshotReadAuthority, build_backup_plan,
    },
    registry::RegistryEntry,
};

// Build one backup plan for create dry-run persistence tests.
pub(in crate::backup::tests) fn valid_backup_plan() -> BackupPlan {
    build_backup_plan(BackupPlanBuildInput {
        plan_id: "plan-test".to_string(),
        run_id: "run-test".to_string(),
        fleet: "demo".to_string(),
        network: "local".to_string(),
        root_canister_id: ROOT.to_string(),
        selected_canister_id: Some(CHILD.to_string()),
        selected_scope_kind: BackupScopeKind::Subtree,
        include_descendants: true,
        topology_hash_before_quiesce: HASH.to_string(),
        registry: &[
            RegistryEntry {
                pid: ROOT.to_string(),
                role: Some("root".to_string()),
                kind: Some("root".to_string()),
                parent_pid: None,
                module_hash: None,
            },
            RegistryEntry {
                pid: CHILD.to_string(),
                role: Some("app".to_string()),
                kind: Some("singleton".to_string()),
                parent_pid: Some(ROOT.to_string()),
                module_hash: Some(HASH.to_string()),
            },
        ],
        control_authority: ControlAuthority::root_controller(AuthorityEvidence::Declared),
        snapshot_read_authority: SnapshotReadAuthority::root_configured_read(
            AuthorityEvidence::Declared,
        ),
        quiescence_policy: canic_backup::plan::QuiescencePolicy::RootCoordinated,
        identity_mode: IdentityMode::Relocatable,
    })
    .expect("backup plan")
}

// Build the executable counterpart of the standard dry-run backup plan.
pub(in crate::backup::tests) fn valid_executable_backup_plan() -> BackupPlan {
    build_backup_plan(BackupPlanBuildInput {
        plan_id: "plan-test".to_string(),
        run_id: "run-test".to_string(),
        fleet: "demo".to_string(),
        network: "local".to_string(),
        root_canister_id: ROOT.to_string(),
        selected_canister_id: Some(CHILD.to_string()),
        selected_scope_kind: BackupScopeKind::Subtree,
        include_descendants: true,
        topology_hash_before_quiesce: HASH.to_string(),
        registry: &[
            RegistryEntry {
                pid: ROOT.to_string(),
                role: Some("root".to_string()),
                kind: Some("root".to_string()),
                parent_pid: None,
                module_hash: None,
            },
            RegistryEntry {
                pid: CHILD.to_string(),
                role: Some("app".to_string()),
                kind: Some("singleton".to_string()),
                parent_pid: Some(ROOT.to_string()),
                module_hash: Some(HASH.to_string()),
            },
        ],
        control_authority: ControlAuthority::operator_controller(AuthorityEvidence::Proven),
        snapshot_read_authority: SnapshotReadAuthority::operator_controller(
            AuthorityEvidence::Proven,
        ),
        quiescence_policy: canic_backup::plan::QuiescencePolicy::CrashConsistent,
        identity_mode: IdentityMode::Relocatable,
    })
    .expect("executable backup plan")
}
