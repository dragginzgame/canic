use super::options;
use crate::cli::clap::passthrough_subcommand;
use clap::Command as ClapCommand;

pub(super) fn usage() -> String {
    let mut command = backup_command();
    command.render_help().to_string()
}

pub(super) fn status_usage() -> String {
    let mut command = options::backup_status_command();
    command.render_help().to_string()
}

pub(super) fn list_usage() -> String {
    let mut command = options::backup_list_command();
    command.render_help().to_string()
}

pub(super) fn create_usage() -> String {
    let mut command = options::backup_create_command();
    command.render_help().to_string()
}

pub(super) fn inspect_usage() -> String {
    let mut command = options::backup_inspect_command();
    command.render_help().to_string()
}

pub(super) fn verify_usage() -> String {
    let mut command = options::backup_verify_command();
    command.render_help().to_string()
}

pub(super) fn backup_command() -> ClapCommand {
    ClapCommand::new("backup")
        .bin_name("canic backup")
        .about("Plan, inspect, and verify backup artifacts")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("create")
                .about("Plan a topology-aware fleet backup")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("list")
                .about("List backup directories under a backup root")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("inspect")
                .about("Inspect a backup or dry-run plan layout")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("verify")
                .about("Verify layout, journal agreement, and durable artifact checksums")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("status")
                .about("Summarize resumable download journal state")
                .disable_help_flag(true),
        ))
}
