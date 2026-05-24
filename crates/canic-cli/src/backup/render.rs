use super::{
    BackupCommandError, BackupCreateReport, BackupInspectOptions, BackupInspectReport,
    BackupListEntry, BackupListOptions, BackupPruneReport, BackupStatusOptions, BackupStatusReport,
    BackupVerifyOptions,
};
use crate::{output, support::path_stamp::backup_list_timestamp};
use canic_backup::persistence::BackupIntegrityReport;
use canic_host::table::{ColumnAlign, render_table};

pub(super) fn write_status_report(
    options: &BackupStatusOptions,
    report: &BackupStatusReport,
) -> Result<(), BackupCommandError> {
    output::write_pretty_json(options.out.as_ref(), report)
}

pub(super) fn write_inspect_report(
    options: &BackupInspectOptions,
    report: &BackupInspectReport,
) -> Result<(), BackupCommandError> {
    if options.json {
        return output::write_pretty_json(options.out.as_ref(), report);
    }

    output::write_text::<BackupCommandError>(options.out.as_ref(), &render_inspect_report(report))
}

pub(super) fn write_create_report(report: &BackupCreateReport) {
    println!("{}", render_create_report(report));
}

fn render_create_report(report: &BackupCreateReport) -> String {
    let rows = [[
        report.fleet.clone(),
        report.network.clone(),
        report.mode.clone(),
        report.layout.clone(),
        report.status.clone(),
        report.scope.clone(),
        report.targets.to_string(),
        report.operations.to_string(),
        report.executed_operations.to_string(),
        report.out.display().to_string(),
    ]];
    render_table(
        &[
            "FLEET",
            "NETWORK",
            "MODE",
            "LAYOUT",
            "STATUS",
            "SCOPE",
            "TARGETS",
            "OPERATIONS",
            "EXECUTED",
            "OUT",
        ],
        &rows,
        &[ColumnAlign::Left; 10],
    )
}

pub(super) fn write_verify_report(
    options: &BackupVerifyOptions,
    report: &BackupIntegrityReport,
) -> Result<(), BackupCommandError> {
    output::write_pretty_json(options.out.as_ref(), report)
}

pub(super) fn write_list_report(
    options: &BackupListOptions,
    entries: &[BackupListEntry],
) -> Result<(), BackupCommandError> {
    output::write_text::<BackupCommandError>(options.out.as_ref(), &render_backup_list(entries))
}

pub(super) fn write_prune_report(
    options: &super::BackupPruneOptions,
    report: &BackupPruneReport,
) -> Result<(), BackupCommandError> {
    output::write_text::<BackupCommandError>(options.out.as_ref(), &render_prune_report(report))
}

fn render_backup_list(entries: &[BackupListEntry]) -> String {
    let rows = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            [
                (index + 1).to_string(),
                entry.dir.display().to_string(),
                entry.backup_id.clone(),
                display_created_at(&entry.created_at),
                entry.members.to_string(),
                entry.status.clone(),
            ]
        })
        .collect::<Vec<_>>();
    render_table(
        &["#", "DIR", "BACKUP_ID", "CREATED_AT", "MEMBERS", "STATUS"],
        &rows,
        &[
            ColumnAlign::Right,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
        ],
    )
}

fn render_prune_report(report: &BackupPruneReport) -> String {
    let summary_rows = [[
        if report.dry_run { "dry-run" } else { "execute" }.to_string(),
        report.scanned.to_string(),
        report.selected.to_string(),
        report.pruned.to_string(),
    ]];
    let entry_rows = report
        .entries
        .iter()
        .map(|entry| {
            [
                entry.index.to_string(),
                entry.dir.display().to_string(),
                entry.backup_id.clone(),
                entry.status.clone(),
                entry.action.clone(),
            ]
        })
        .collect::<Vec<_>>();

    [
        render_table(
            &["MODE", "SCANNED", "SELECTED", "PRUNED"],
            &summary_rows,
            &[ColumnAlign::Left; 4],
        ),
        String::new(),
        render_table(
            &["#", "DIR", "BACKUP_ID", "STATUS", "ACTION"],
            &entry_rows,
            &[
                ColumnAlign::Right,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
            ],
        ),
    ]
    .join("\n")
}

fn render_inspect_report(report: &BackupInspectReport) -> String {
    let summary_rows = [[
        report.layout_status.clone(),
        report.fleet.clone(),
        report.network.clone(),
        report.scope.clone(),
        report.targets.len().to_string(),
        report.operations.len().to_string(),
        report.execution.next_operation.as_ref().map_or_else(
            || "-".to_string(),
            |operation| operation.operation_id.clone(),
        ),
    ]];
    let target_rows = report
        .targets
        .iter()
        .map(|target| {
            [
                target.role.clone(),
                target.canister_id.clone(),
                target.parent_canister_id.clone(),
                target.expected_module_hash.clone(),
                target.depth.to_string(),
                target.control_authority.clone(),
                target.snapshot_read_authority.clone(),
            ]
        })
        .collect::<Vec<_>>();
    let operation_rows = report
        .operations
        .iter()
        .map(|operation| {
            [
                operation.sequence.to_string(),
                operation.kind.clone(),
                operation.target_canister_id.clone(),
                operation.state.clone(),
                operation.blocking_reasons.join("; "),
            ]
        })
        .collect::<Vec<_>>();

    [
        format!("Plan: {}", report.plan_id),
        format!("Run:  {}", report.run_id),
        String::new(),
        render_table(
            &[
                "STATUS",
                "FLEET",
                "NETWORK",
                "SCOPE",
                "TARGETS",
                "OPERATIONS",
                "NEXT",
            ],
            &summary_rows,
            &[ColumnAlign::Left; 7],
        ),
        String::new(),
        "Targets".to_string(),
        render_table(
            &[
                "ROLE",
                "CANISTER_ID",
                "PARENT",
                "MODULE_HASH",
                "DEPTH",
                "CONTROL",
                "SNAPSHOT_READ",
            ],
            &target_rows,
            &[ColumnAlign::Left; 7],
        ),
        String::new(),
        "Operations".to_string(),
        render_table(
            &["SEQ", "KIND", "TARGET", "STATE", "REASONS"],
            &operation_rows,
            &[
                ColumnAlign::Right,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
            ],
        ),
    ]
    .join("\n")
}

fn display_created_at(created_at: &str) -> String {
    created_at
        .strip_prefix("unix:")
        .and_then(|seconds| seconds.parse::<u64>().ok())
        .map_or_else(|| created_at.to_string(), backup_list_timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::{BackupInspectOperation, BackupInspectTarget};
    use std::path::PathBuf;

    // Ensure backup create output makes reused output layouts visible.
    #[test]
    fn render_backup_create_report_shows_layout_source() {
        let report = BackupCreateReport {
            fleet: "demo".to_string(),
            network: "local".to_string(),
            out: PathBuf::from("backups/demo"),
            plan_id: "plan-demo".to_string(),
            run_id: "run-demo".to_string(),
            mode: "dry-run".to_string(),
            layout: "existing".to_string(),
            status: "planned".to_string(),
            scope: "fleet".to_string(),
            targets: 2,
            operations: 3,
            executed_operations: 0,
        };

        let text = render_create_report(&report);

        assert!(text.contains("LAYOUT"));
        assert!(text.contains("existing"));
    }

    // Ensure backup list text output includes table headers and display timestamps.
    #[test]
    fn render_backup_list_formats_unix_created_at() {
        let entries = vec![BackupListEntry {
            dir: PathBuf::from("backups/fleet-demo-20240507-140000"),
            backup_id: "backup".to_string(),
            created_at: "unix:1715090400".to_string(),
            members: 7,
            status: "ok".to_string(),
        }];

        let rendered = render_backup_list(&entries);

        assert!(rendered.contains('#'));
        assert!(rendered.contains("DIR"));
        assert!(rendered.contains("07/05/2024 14:00"));
        assert!(!rendered.contains("unix:"));
    }

    // Ensure backup inspect text includes summary, target, and operation sections.
    #[test]
    fn render_backup_inspect_report_shows_layout_sections() {
        let report = BackupInspectReport {
            layout_status: "dry-run".to_string(),
            plan_id: "plan-test".to_string(),
            run_id: "run-test".to_string(),
            fleet: "demo".to_string(),
            network: "local".to_string(),
            scope: "fleet".to_string(),
            targets: vec![BackupInspectTarget {
                role: "child".to_string(),
                canister_id: "aaaaa-aa".to_string(),
                parent_canister_id: "bbbbb-bb".to_string(),
                expected_module_hash: "hash-test".to_string(),
                depth: 1,
                control_authority: "root-controller/proven".to_string(),
                snapshot_read_authority: "root-configured-read/proven".to_string(),
            }],
            operations: vec![BackupInspectOperation {
                sequence: 1,
                kind: "create-snapshot".to_string(),
                target_canister_id: "aaaaa-aa".to_string(),
                state: "ready".to_string(),
                blocking_reasons: Vec::new(),
            }],
            execution: canic_backup::execution::BackupExecutionResumeSummary {
                plan_id: "plan-test".to_string(),
                run_id: "run-test".to_string(),
                preflight_id: None,
                preflight_accepted: false,
                restart_required: false,
                total_operations: 1,
                ready_operations: 1,
                pending_operations: 0,
                blocked_operations: 0,
                completed_operations: 0,
                failed_operations: 0,
                skipped_operations: 0,
                next_operation: None,
            },
        };

        let rendered = render_inspect_report(&report);

        assert!(rendered.contains("Plan: plan-test"));
        assert!(rendered.contains("Targets"));
        assert!(rendered.contains("Operations"));
        assert!(rendered.contains("child"));
        assert!(rendered.contains("MODULE_HASH"));
        assert!(rendered.contains("hash-test"));
    }
}
