use super::options;
use crate::cli::clap::{passthrough_subcommand, render_usage};
use clap::Command as ClapCommand;

pub(super) fn usage() -> String {
    render_usage(backup_command)
}

pub(super) fn status_usage() -> String {
    render_usage(options::backup_status_command)
}

pub(super) fn list_usage() -> String {
    render_usage(options::backup_list_command)
}

pub(super) fn create_usage() -> String {
    render_usage(options::backup_create_command)
}

pub(super) fn inspect_usage() -> String {
    render_usage(options::backup_inspect_command)
}

pub(super) fn prune_usage() -> String {
    render_usage(options::backup_prune_command)
}

pub(super) fn verify_usage() -> String {
    render_usage(options::backup_verify_command)
}

pub(super) fn backup_command() -> ClapCommand {
    ClapCommand::new("backup")
        .bin_name("canic backup")
        .about("Plan, inspect, and verify backup artifacts")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("create")
                .about("Plan a topology-aware deployment backup")
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
            ClapCommand::new("manifest")
                .about("Validate backup manifests")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("prune")
                .about("Remove selected backup directories")
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
