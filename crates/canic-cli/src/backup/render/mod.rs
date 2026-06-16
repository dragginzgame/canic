use super::{
    BackupCommandError, BackupCreateReport, BackupInspectOptions, BackupInspectReport,
    BackupListEntry, BackupListOptions, BackupPruneReport, BackupStatusOptions, BackupStatusReport,
    BackupVerifyOptions,
};
use crate::output;
use canic_backup::persistence::BackupIntegrityReport;

mod create;
mod inspect;
mod list;
mod prune;

use create::render_create_report;
use inspect::render_inspect_report;
use list::render_backup_list;
use prune::render_prune_report;

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
