mod enforce;
mod error;
mod io;
mod options;

use crate::{
    cli::clap::{parse_subcommand, passthrough_subcommand},
    cli::help::print_help_or_version,
    version_text,
};
use canic_backup::restore::{
    RestoreApplyCommandConfig, RestoreApplyDryRun, RestorePlan, RestorePlanner, RestoreRunResponse,
    RestoreRunnerCommandExecutor, RestoreRunnerCommandOutput, RestoreRunnerConfig,
};
use canic_host::icp;
use clap::Command as ClapCommand;
use std::ffi::OsString;

const RESTORE_HELP_AFTER: &str = "\
Examples:
  canic backup create test
  canic backup list
  canic restore prepare 1 --require-verified --require-restore-ready
  canic restore status 1
  canic restore run 1 --execute --max-steps 1 --require-no-attention";

use enforce::{
    enforce_restore_plan_requirements, enforce_restore_run_requirements,
    enforce_restore_status_requirements,
};
use io::{
    RestorePrepareReport, default_restore_apply_journal_path, default_restore_plan_path,
    read_manifest_source, read_mapping, read_plan, restore_apply_backup_dir,
    restore_apply_plan_path, restore_prepare_backup_dir, restore_run_journal_path,
    restore_status_journal_path, verify_backup_layout_if_required,
    verify_prepared_journal_backup_root, write_apply_dry_run, write_apply_journal_file,
    write_apply_journal_if_requested, write_plan, write_plan_file, write_prepare_report,
    write_restore_run, write_restore_status,
};

pub use error::RestoreCommandError;
pub use options::{
    RestoreApplyOptions, RestorePlanOptions, RestorePrepareOptions, RestoreRunOptions,
    RestoreStatusOptions,
};

pub fn run<I>(args: I) -> Result<(), RestoreCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let Some((command, args)) = parse_subcommand(restore_command(), args)
        .map_err(|_| RestoreCommandError::Usage(usage()))?
    else {
        return Err(RestoreCommandError::Usage(usage()));
    };

    match command.as_str() {
        "plan" => {
            if print_help_or_version(&args, plan_usage, version_text()) {
                return Ok(());
            }
            let options = RestorePlanOptions::parse(args)?;
            let plan = plan_restore(&options)?;
            write_plan(&options, &plan)?;
            enforce_restore_plan_requirements(&options, &plan)?;
            Ok(())
        }
        "prepare" => {
            if print_help_or_version(&args, prepare_usage, version_text()) {
                return Ok(());
            }
            let options = RestorePrepareOptions::parse(args)?;
            let report = restore_prepare(&options)?;
            write_prepare_report(&options, &report)?;
            Ok(())
        }
        "apply" => {
            if print_help_or_version(&args, apply_usage, version_text()) {
                return Ok(());
            }
            let options = RestoreApplyOptions::parse(args)?;
            let dry_run = restore_apply_dry_run(&options)?;
            write_apply_dry_run(&options, &dry_run)?;
            write_apply_journal_if_requested(&options, &dry_run)?;
            Ok(())
        }
        "run" => {
            if print_help_or_version(&args, run_usage, version_text()) {
                return Ok(());
            }
            let options = RestoreRunOptions::parse(args)?;
            let run = if options.execute {
                restore_run_execute_result(&options)?
            } else if options.retry_failed {
                canic_backup::restore::RestoreRunnerOutcome {
                    response: restore_run_retry_failed(&options)?,
                    error: None,
                }
            } else if options.unclaim_pending {
                canic_backup::restore::RestoreRunnerOutcome {
                    response: restore_run_unclaim_pending(&options)?,
                    error: None,
                }
            } else {
                canic_backup::restore::RestoreRunnerOutcome {
                    response: restore_run_dry_run(&options)?,
                    error: None,
                }
            };
            write_restore_run(&options, &run.response)?;
            if let Some(error) = run.error {
                return Err(error.into());
            }
            enforce_restore_run_requirements(&options, &run.response)?;
            Ok(())
        }
        "status" => {
            if print_help_or_version(&args, status_usage, version_text()) {
                return Ok(());
            }
            let options = RestoreStatusOptions::parse(args)?;
            let response = restore_status(&options)?;
            write_restore_status(&options, &response)?;
            enforce_restore_status_requirements(&options, &response)?;
            Ok(())
        }
        _ => unreachable!("restore dispatch command only defines known commands"),
    }
}

fn restore_prepare(
    options: &RestorePrepareOptions,
) -> Result<RestorePrepareReport, RestoreCommandError> {
    let backup_dir = restore_prepare_backup_dir(options)?;
    let plan_path = options
        .plan_out
        .clone()
        .unwrap_or_else(|| default_restore_plan_path(&backup_dir));
    let journal_path = options
        .journal_out
        .clone()
        .unwrap_or_else(|| default_restore_apply_journal_path(&backup_dir));
    let plan_options = RestorePlanOptions {
        backup_ref: None,
        manifest: None,
        backup_dir: Some(backup_dir.clone()),
        mapping: options.mapping.clone(),
        out: None,
        require_verified: options.require_verified,
        require_restore_ready: options.require_restore_ready,
    };
    let plan = plan_restore(&plan_options)?;
    enforce_restore_plan_requirements(&plan_options, &plan)?;
    write_plan_file(&plan_path, &plan)?;

    let dry_run = RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &backup_dir)?;
    write_apply_journal_file(&journal_path, &dry_run)?;

    Ok(RestorePrepareReport {
        backup_dir: backup_dir.display().to_string(),
        plan_path: plan_path.display().to_string(),
        journal_path: journal_path.display().to_string(),
        backup_id: plan.backup_id,
        ready: dry_run.ready,
        readiness_reasons: dry_run.readiness_reasons,
        members: dry_run.member_count,
        operations: dry_run.rendered_operations,
    })
}

pub fn plan_restore(options: &RestorePlanOptions) -> Result<RestorePlan, RestoreCommandError> {
    verify_backup_layout_if_required(options)?;

    let manifest = read_manifest_source(options)?;
    let mapping = options.mapping.as_ref().map(read_mapping).transpose()?;

    RestorePlanner::plan(&manifest, mapping.as_ref()).map_err(RestoreCommandError::from)
}

pub fn restore_apply_dry_run(
    options: &RestoreApplyOptions,
) -> Result<RestoreApplyDryRun, RestoreCommandError> {
    let plan = read_plan(&restore_apply_plan_path(options)?)?;
    if let Some(backup_dir) = restore_apply_backup_dir(options)? {
        return RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, &backup_dir)
            .map_err(RestoreCommandError::from);
    }

    Ok(RestoreApplyDryRun::from_plan(&plan))
}

pub fn restore_run_dry_run(
    options: &RestoreRunOptions,
) -> Result<RestoreRunResponse, RestoreCommandError> {
    canic_backup::restore::restore_run_dry_run(&restore_runner_config(options)?)
        .map_err(RestoreCommandError::from)
}

pub fn restore_run_unclaim_pending(
    options: &RestoreRunOptions,
) -> Result<RestoreRunResponse, RestoreCommandError> {
    canic_backup::restore::restore_run_unclaim_pending(&restore_runner_config(options)?)
        .map_err(RestoreCommandError::from)
}

pub fn restore_run_retry_failed(
    options: &RestoreRunOptions,
) -> Result<RestoreRunResponse, RestoreCommandError> {
    canic_backup::restore::restore_run_retry_failed(&restore_runner_config(options)?)
        .map_err(RestoreCommandError::from)
}

pub fn restore_status(
    options: &RestoreStatusOptions,
) -> Result<RestoreRunResponse, RestoreCommandError> {
    canic_backup::restore::restore_run_dry_run(&restore_status_runner_config(options)?)
        .map_err(RestoreCommandError::from)
}

// Execute ready restore apply operations and retain any deferred runner error.
fn restore_run_execute_result(
    options: &RestoreRunOptions,
) -> Result<canic_backup::restore::RestoreRunnerOutcome, RestoreCommandError> {
    let mut executor = HostRestoreCommandExecutor;
    canic_backup::restore::restore_run_execute_result_with_executor(
        &restore_runner_config(options)?,
        &mut executor,
    )
    .map_err(RestoreCommandError::from)
}

///
/// HostRestoreCommandExecutor
///

struct HostRestoreCommandExecutor;

impl RestoreRunnerCommandExecutor for HostRestoreCommandExecutor {
    fn execute(
        &mut self,
        command: &canic_backup::restore::RestoreApplyRunnerCommand,
    ) -> Result<RestoreRunnerCommandOutput, std::io::Error> {
        let output = icp::run_raw_output(&command.program, &command.args)?;
        Ok(RestoreRunnerCommandOutput {
            success: output.success,
            status: output.status,
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

fn restore_command_config(program: &str, network: Option<&str>) -> RestoreApplyCommandConfig {
    RestoreApplyCommandConfig {
        program: program.to_string(),
        network: network.map(str::to_string),
    }
}

fn restore_runner_config(
    options: &RestoreRunOptions,
) -> Result<RestoreRunnerConfig, RestoreCommandError> {
    let journal = restore_run_journal_path(options)?;
    verify_prepared_journal_backup_root(options.backup_ref.as_deref(), &journal)?;

    Ok(RestoreRunnerConfig {
        journal,
        command: restore_command_config(&options.icp, options.network.as_deref()),
        max_steps: options.max_steps,
        updated_at: None,
    })
}

fn restore_status_runner_config(
    options: &RestoreStatusOptions,
) -> Result<RestoreRunnerConfig, RestoreCommandError> {
    let journal = restore_status_journal_path(options)?;
    verify_prepared_journal_backup_root(options.backup_ref.as_deref(), &journal)?;

    Ok(RestoreRunnerConfig {
        journal,
        command: restore_command_config(&options.icp, options.network.as_deref()),
        max_steps: None,
        updated_at: None,
    })
}

fn usage() -> String {
    let mut command = restore_command();
    command.render_help().to_string()
}

fn plan_usage() -> String {
    let mut command = options::restore_plan_command();
    command.render_help().to_string()
}

fn apply_usage() -> String {
    let mut command = options::restore_apply_command();
    command.render_help().to_string()
}

fn prepare_usage() -> String {
    let mut command = options::restore_prepare_command();
    command.render_help().to_string()
}

fn run_usage() -> String {
    let mut command = options::restore_run_command();
    command.render_help().to_string()
}

fn status_usage() -> String {
    let mut command = options::restore_status_command();
    command.render_help().to_string()
}

fn restore_command() -> ClapCommand {
    ClapCommand::new("restore")
        .bin_name("canic restore")
        .about("Plan, apply, and run snapshot restores")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("plan")
                .about("Build a no-mutation restore plan")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("prepare")
                .about("Prepare a backup layout for restore")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("apply")
                .about("Render restore operations and optionally write an apply journal")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("run")
                .about("Preview, execute, or recover the native restore runner")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("status")
                .about("Summarize restore runner journal state")
                .disable_help_flag(true),
        ))
        .after_help(RESTORE_HELP_AFTER)
}

#[cfg(test)]
mod tests;
