mod enforce;
mod error;
mod io;
mod options;

use crate::version_text;
use canic_backup::restore::{
    RestoreApplyCommandConfig, RestoreApplyDryRun, RestorePlan, RestorePlanner, RestoreRunResponse,
    RestoreRunnerCommandExecutor, RestoreRunnerCommandOutput, RestoreRunnerConfig,
};
use canic_host::dfx;
use clap::Command as ClapCommand;
use std::ffi::OsString;

use enforce::{enforce_restore_plan_requirements, enforce_restore_run_requirements};
use io::{
    read_manifest_source, read_mapping, read_plan, verify_backup_layout_if_required,
    write_apply_dry_run, write_apply_journal_if_requested, write_plan, write_restore_run,
};

pub use error::RestoreCommandError;
pub use options::{RestoreApplyOptions, RestorePlanOptions, RestoreRunOptions};

/// Run a restore subcommand.
pub fn run<I>(args: I) -> Result<(), RestoreCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let Some(command) = args.next().and_then(|arg| arg.into_string().ok()) else {
        return Err(RestoreCommandError::Usage(usage()));
    };

    match command.as_str() {
        "plan" => {
            let options = RestorePlanOptions::parse(args)?;
            let plan = plan_restore(&options)?;
            write_plan(&options, &plan)?;
            enforce_restore_plan_requirements(&options, &plan)?;
            Ok(())
        }
        "apply" => {
            let options = RestoreApplyOptions::parse(args)?;
            let dry_run = restore_apply_dry_run(&options)?;
            write_apply_dry_run(&options, &dry_run)?;
            write_apply_journal_if_requested(&options, &dry_run)?;
            Ok(())
        }
        "run" => {
            let options = RestoreRunOptions::parse(args)?;
            let run = if options.execute {
                restore_run_execute_result(&options)?
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
        "help" | "--help" | "-h" => {
            println!("{}", usage());
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("{}", version_text());
            Ok(())
        }
        _ => Err(RestoreCommandError::UnknownOption(command)),
    }
}

/// Build a no-mutation restore plan from a manifest and optional mapping.
pub fn plan_restore(options: &RestorePlanOptions) -> Result<RestorePlan, RestoreCommandError> {
    verify_backup_layout_if_required(options)?;

    let manifest = read_manifest_source(options)?;
    let mapping = options.mapping.as_ref().map(read_mapping).transpose()?;

    RestorePlanner::plan(&manifest, mapping.as_ref()).map_err(RestoreCommandError::from)
}

/// Build a no-mutation restore apply dry-run from a restore plan.
pub fn restore_apply_dry_run(
    options: &RestoreApplyOptions,
) -> Result<RestoreApplyDryRun, RestoreCommandError> {
    let plan = read_plan(&options.plan)?;
    if let Some(backup_dir) = &options.backup_dir {
        return RestoreApplyDryRun::try_from_plan_with_artifacts(&plan, backup_dir)
            .map_err(RestoreCommandError::from);
    }

    Ok(RestoreApplyDryRun::from_plan(&plan))
}

/// Build a no-mutation native restore runner preview from a journal file.
pub fn restore_run_dry_run(
    options: &RestoreRunOptions,
) -> Result<RestoreRunResponse, RestoreCommandError> {
    canic_backup::restore::restore_run_dry_run(&restore_runner_config(options))
        .map_err(RestoreCommandError::from)
}

/// Recover an interrupted restore runner by unclaiming the pending operation.
pub fn restore_run_unclaim_pending(
    options: &RestoreRunOptions,
) -> Result<RestoreRunResponse, RestoreCommandError> {
    canic_backup::restore::restore_run_unclaim_pending(&restore_runner_config(options))
        .map_err(RestoreCommandError::from)
}

// Execute ready restore apply operations and retain any deferred runner error.
fn restore_run_execute_result(
    options: &RestoreRunOptions,
) -> Result<canic_backup::restore::RestoreRunnerOutcome, RestoreCommandError> {
    let mut executor = HostRestoreCommandExecutor;
    canic_backup::restore::restore_run_execute_result_with_executor(
        &restore_runner_config(options),
        &mut executor,
    )
    .map_err(RestoreCommandError::from)
}

///
/// HostRestoreCommandExecutor
///

struct HostRestoreCommandExecutor;

impl RestoreRunnerCommandExecutor for HostRestoreCommandExecutor {
    /// Execute restore runner commands through the host-side dfx/process boundary.
    fn execute(
        &mut self,
        command: &canic_backup::restore::RestoreApplyRunnerCommand,
    ) -> Result<RestoreRunnerCommandOutput, std::io::Error> {
        let output = dfx::run_raw_output(&command.program, &command.args)?;
        Ok(RestoreRunnerCommandOutput {
            success: output.success,
            status: output.status,
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

// Build command-preview configuration from common dfx/network inputs.
fn restore_command_config(program: &str, network: Option<&str>) -> RestoreApplyCommandConfig {
    RestoreApplyCommandConfig {
        program: program.to_string(),
        network: network.map(str::to_string),
    }
}

// Build the lower-level restore runner configuration from CLI flags.
fn restore_runner_config(options: &RestoreRunOptions) -> RestoreRunnerConfig {
    RestoreRunnerConfig {
        journal: options.journal.clone(),
        command: restore_command_config(&options.dfx, options.network.as_deref()),
        max_steps: options.max_steps,
        updated_at: None,
    }
}

// Return restore command usage text.
fn usage() -> String {
    let mut command = restore_command();
    command.render_help().to_string()
}

// Return restore plan usage text.
fn plan_usage() -> String {
    let mut command = options::restore_plan_command();
    command.render_help().to_string()
}

// Return restore apply usage text.
fn apply_usage() -> String {
    let mut command = options::restore_apply_command();
    command.render_help().to_string()
}

// Return restore run usage text.
fn run_usage() -> String {
    let mut command = options::restore_run_command();
    command.render_help().to_string()
}

// Build the restore command-family parser for help rendering.
fn restore_command() -> ClapCommand {
    ClapCommand::new("restore")
        .bin_name("canic restore")
        .about("Plan, apply, and run snapshot restores")
        .disable_help_flag(true)
        .subcommand(
            ClapCommand::new("plan")
                .about("Build a no-mutation restore plan")
                .disable_help_flag(true),
        )
        .subcommand(
            ClapCommand::new("apply")
                .about("Render restore operations and optionally write an apply journal")
                .disable_help_flag(true),
        )
        .subcommand(
            ClapCommand::new("run")
                .about("Preview, execute, or recover the native restore runner")
                .disable_help_flag(true),
        )
}

#[cfg(test)]
mod tests;
