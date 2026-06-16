//! Module: backup::tests::fixtures::layout
//!
//! Responsibility: build backup layout test fixtures.
//! Does not own: manifest, plan, or execution journal construction.
//! Boundary: filesystem layout helpers for backup command tests.

use super::{manifest::valid_manifest, plan::valid_backup_plan};
use crate::{
    backup::{BackupDryRunStatusReport, BackupStatusOptions, BackupStatusReport, backup_status},
    test_support::temp_dir,
};
use canic_backup::{execution::BackupExecutionJournal, persistence::BackupLayout};
use std::{fs, path::Path};

// Write a manifest plus matching plan but no execution journal.
pub(in crate::backup::tests) fn write_manifest_plan_without_execution_journal(root: &Path) {
    let layout = BackupLayout::new(root.to_path_buf());
    layout
        .write_backup_plan(&valid_backup_plan())
        .expect("write backup plan");
    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
}

// Write a manifest plus matching plan and caller-provided execution journal.
pub(in crate::backup::tests) fn write_manifest_plan_journal(
    root: &Path,
    journal: BackupExecutionJournal,
) {
    let layout = BackupLayout::new(root.to_path_buf());
    layout
        .write_manifest(&valid_manifest())
        .expect("write manifest");
    layout
        .write_backup_plan(&valid_backup_plan())
        .expect("write backup plan");
    layout
        .write_execution_journal(&journal)
        .expect("write execution journal");
}

// Read backup status from one caller-provided execution journal layout.
pub(in crate::backup::tests) fn backup_status_for_execution_journal(
    name: &str,
    journal: BackupExecutionJournal,
    write_manifest: bool,
) -> BackupDryRunStatusReport {
    let root = temp_dir(name);
    let layout = BackupLayout::new(root.clone());
    layout
        .write_backup_plan(&valid_backup_plan())
        .expect("write backup plan");
    layout
        .write_execution_journal(&journal)
        .expect("write execution journal");
    if write_manifest {
        layout
            .write_manifest(&valid_manifest())
            .expect("write manifest");
    }
    let options = BackupStatusOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
        require_complete: false,
    };
    let report = backup_status(&options).expect("read backup status");

    fs::remove_dir_all(root).expect("remove temp root");
    let BackupStatusReport::DryRun(report) = report else {
        panic!("expected execution status");
    };
    report
}
