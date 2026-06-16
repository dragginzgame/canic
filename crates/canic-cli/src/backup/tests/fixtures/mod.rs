//! Module: backup::tests::fixtures
//!
//! Responsibility: shared backup CLI test fixtures and layout builders.
//! Does not own: production backup planning, persistence, or command dispatch.
//! Boundary: deterministic test data for backup command unit tests.

mod artifact;
mod download;
mod execution;
mod layout;
mod manifest;
mod plan;
mod stamp;

pub(super) use artifact::write_artifact;
pub(super) use download::{created_journal, journal_with_checksum};
pub(super) use execution::{
    accepted_execution_journal, complete_execution_operation, fail_execution_operation,
};
pub(super) use layout::{
    backup_status_for_execution_journal, write_manifest_plan_journal,
    write_manifest_plan_without_execution_journal,
};
pub(super) use manifest::{valid_manifest, valid_manifest_with};
pub(super) use plan::{valid_backup_plan, valid_executable_backup_plan};
pub(super) use stamp::unix_marker_for_stamp;

pub(super) const ROOT: &str = "aaaaa-aa";
pub(super) const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";
pub(super) const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
