//! Module: backup::dispatch
//!
//! Responsibility: dispatch `canic backup` subcommands.
//! Does not own: command definitions, option parsing internals, or operations.
//! Boundary: maps parsed backup subcommands to option parsing, execution, and output.

use super::{
    BackupCommandError, BackupCreateOptions, BackupInspectOptions, BackupListOptions,
    BackupPruneOptions, BackupStatusOptions, BackupVerifyOptions, backup_command, backup_create,
    backup_inspect, backup_list, backup_prune, backup_status, create_usage,
    enforce_status_requirements, inspect_usage, list_usage, prune_usage, run_manifest,
    status_usage, usage, verify_backup, verify_usage, write_create_report, write_inspect_report,
    write_list_report, write_prune_report, write_status_report, write_verify_report,
};
use crate::{cli::clap::parse_required_subcommand, cli::help::print_help_or_version, version_text};
use std::ffi::OsString;

pub fn run<I>(args: I) -> Result<(), BackupCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let (command, args) = parse_required_subcommand(backup_command(), args)
        .map_err(|_| BackupCommandError::Usage(usage()))?;

    match command.as_str() {
        "create" => {
            if print_help_or_version(&args, create_usage, version_text()) {
                return Ok(());
            }
            let options = BackupCreateOptions::parse(args)?;
            let report = backup_create(&options)?;
            write_create_report(&report);
            Ok(())
        }
        "list" => {
            if print_help_or_version(&args, list_usage, version_text()) {
                return Ok(());
            }
            let options = BackupListOptions::parse(args)?;
            let entries = backup_list(&options)?;
            write_list_report(&options, &entries)?;
            Ok(())
        }
        "inspect" => {
            if print_help_or_version(&args, inspect_usage, version_text()) {
                return Ok(());
            }
            let options = BackupInspectOptions::parse(args)?;
            let report = backup_inspect(&options)?;
            write_inspect_report(&options, &report)?;
            Ok(())
        }
        "manifest" => run_manifest(args).map_err(BackupCommandError::from),
        "prune" => {
            if print_help_or_version(&args, prune_usage, version_text()) {
                return Ok(());
            }
            let options = BackupPruneOptions::parse(args)?;
            let report = backup_prune(&options)?;
            write_prune_report(&options, &report)?;
            Ok(())
        }
        "status" => {
            if print_help_or_version(&args, status_usage, version_text()) {
                return Ok(());
            }
            let options = BackupStatusOptions::parse(args)?;
            let report = backup_status(&options)?;
            write_status_report(&options, &report)?;
            enforce_status_requirements(&options, &report)?;
            Ok(())
        }
        "verify" => {
            if print_help_or_version(&args, verify_usage, version_text()) {
                return Ok(());
            }
            let options = BackupVerifyOptions::parse(args)?;
            let report = verify_backup(&options)?;
            write_verify_report(&options, &report)?;
            Ok(())
        }
        _ => unreachable!("backup dispatch command only defines known commands"),
    }
}
