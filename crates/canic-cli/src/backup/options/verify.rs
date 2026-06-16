//! Module: backup::options::verify
//!
//! Responsibility: parse `canic backup verify` options.
//! Does not own: backup verification execution.
//! Boundary: verify source selector and output path.

use super::shared::{backup_dir_out_command, backup_target, parse_backup_options};
use crate::{
    backup::{BackupCommandError, verify_usage},
    cli::clap::path_option,
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

///
/// BackupVerifyOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::backup) struct BackupVerifyOptions {
    pub(in crate::backup) backup_ref: Option<String>,
    pub(in crate::backup) dir: Option<PathBuf>,
    pub(in crate::backup) out: Option<PathBuf>,
}

impl BackupVerifyOptions {
    pub(in crate::backup) fn parse<I>(args: I) -> Result<Self, BackupCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_backup_options(backup_verify_command(), verify_usage, args)?;
        let (backup_ref, dir) = backup_target(&matches);

        Ok(Self {
            backup_ref,
            dir,
            out: path_option(&matches, "out"),
        })
    }
}

pub(in crate::backup) fn backup_verify_command() -> ClapCommand {
    backup_dir_out_command(
        "verify",
        "canic backup verify",
        "Verify layout, journal agreement, and durable artifact checksums",
    )
}
