//! Module: backup::tests::status::json
//!
//! Responsibility: backup status JSON shape tests.
//! Does not own: completion behavior or status read paths.
//! Boundary: serialized status report field names.

use super::super::super::*;
use super::super::fixtures::*;
use canic_backup::execution::BackupExecutionJournal;

// Ensure dry-run status JSON exposes deployment identity, not stale fleet identity.
#[test]
fn backup_dry_run_status_json_uses_deployment_identity_field() {
    let plan = valid_backup_plan();
    let report = BackupDryRunStatusReport {
        layout_status: "dry-run".to_string(),
        plan_id: plan.plan_id.clone(),
        run_id: plan.run_id.clone(),
        deployment: plan.fleet.clone(),
        network: plan.network.clone(),
        targets: plan.targets.len(),
        operations: plan.phases.len(),
        execution: BackupExecutionJournal::from_plan(&plan)
            .expect("execution journal")
            .resume_summary(),
    };

    let value = serde_json::to_value(&report).expect("serialize status report");

    assert_eq!(value["deployment"], "demo");
    assert!(value.get("fleet").is_none());
}
