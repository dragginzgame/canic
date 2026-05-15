use super::{
    BackupCommandError, BackupCreateReport, BackupInspectOptions, BackupInspectReport,
    BackupListEntry, BackupListOptions, BackupStatusOptions, BackupStatusReport,
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

pub(super) fn render_create_report(report: &BackupCreateReport) -> String {
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

pub(super) fn render_backup_list(entries: &[BackupListEntry]) -> String {
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

pub(super) fn render_inspect_report(report: &BackupInspectReport) -> String {
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
