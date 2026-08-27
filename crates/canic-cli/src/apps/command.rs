//! Module: apps::command
//! Responsibility: build `canic app` Clap command definitions and usage text.
//! Does not own: command dispatch, filesystem mutation, report rendering, or host operations.
//! Boundary: passive CLI surface construction for the app command family.

use crate::{
    cli::{
        clap::{flag_arg, passthrough_subcommand, render_usage, value_arg},
        globals::internal_environment_arg,
    },
    scaffold,
};
use clap::Command as ClapCommand;

const APP_HELP_AFTER: &str = "\
Examples:
  canic app list
  canic app create demo

Leaf help identifies read-only commands and mutations that support --dry-run.";
const APP_LIST_HELP_AFTER: &str = "\
Examples:
  canic app list

Commands that operate on one app take the app name as a positional argument.";
const APP_CHECK_HELP_AFTER: &str = "\
Examples:
  canic app check test";
const APP_DELETE_HELP_AFTER: &str = "\
Examples:
  canic app delete demo
  canic app delete demo --dry-run

This removes the matching config-defined app directory after you type the
app name exactly. --dry-run validates and prints the target without
prompting or deleting files.";
const APP_ROLE_HELP_AFTER: &str = "\
Examples:
  canic app role inspect demo app
  canic app role attach demo store --component-spec default

Inspect and list are read-only; mutations support --dry-run.";
const APP_ROLE_LIST_HELP_AFTER: &str = "\
Examples:
  canic app role list demo";
const APP_ROLE_INSPECT_HELP_AFTER: &str = "\
Examples:
  canic app role inspect demo app";
const APP_ROLE_DECLARE_HELP_AFTER: &str = "\
Examples:
  canic app role declare demo store --package store
  canic app role declare demo store --package store --dry-run";
const APP_ROLE_ATTACH_HELP_AFTER: &str = "\
Examples:
  canic app role attach demo store --component-spec default
  canic app role attach demo worker --component-spec default --kind replica
  canic app role attach demo store --component-spec default --dry-run";
const APP_ROLE_RENAME_HELP_AFTER: &str = "\
Examples:
  canic app role rename demo hub router
  canic app role rename demo hub router --dry-run";

pub(super) fn app_command() -> ClapCommand {
    ClapCommand::new("app")
        .bin_name("canic app")
        .about("Manage Canic apps")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("check")
                .about("Check icp.yaml for one Canic app")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("config")
                .about("Inspect selected app config")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("create")
                .about("Create a minimal Canic app")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("delete")
                .about("Delete a config-defined Canic app")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("list")
                .about("List config-defined Canic apps")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("role")
                .about("Manage app role lifecycle")
                .disable_help_flag(true),
        ))
        .after_help(APP_HELP_AFTER)
}

pub(super) fn app_role_command() -> ClapCommand {
    ClapCommand::new("role")
        .bin_name("canic app role")
        .about("Manage app role lifecycle")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("attach")
                .about("Attach a declared role to direct topology")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("declare")
                .about("Declare an existing package-backed role")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("inspect")
                .about("Inspect one declared app role")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("list")
                .about("List declared app roles")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("rename")
                .about("Rename a declared app role")
                .disable_help_flag(true),
        ))
        .after_help(APP_ROLE_HELP_AFTER)
}

pub(super) fn app_role_declare_command() -> ClapCommand {
    ClapCommand::new("declare")
        .bin_name("canic app role declare")
        .about("Declare an existing package-backed role")
        .disable_help_flag(true)
        .arg(
            value_arg("app")
                .value_name("app")
                .required(true)
                .help("Config-defined app name"),
        )
        .arg(
            value_arg("role")
                .value_name("role")
                .required(true)
                .help("Local role name"),
        )
        .arg(
            clap::Arg::new("package")
                .long("package")
                .value_name("path")
                .required(true)
                .help("Package path recorded in [roles.<role>]"),
        )
        .arg(
            flag_arg("dry-run")
                .long("dry-run")
                .help("Validate and print planned config writes without changing files"),
        )
        .after_help(APP_ROLE_DECLARE_HELP_AFTER)
}

pub(super) fn app_role_attach_command() -> ClapCommand {
    ClapCommand::new("attach")
        .bin_name("canic app role attach")
        .about("Attach a declared role to direct topology")
        .disable_help_flag(true)
        .arg(
            value_arg("app")
                .value_name("app")
                .required(true)
                .help("Config-defined app name"),
        )
        .arg(
            value_arg("role")
                .value_name("role")
                .required(true)
                .help("Local role name"),
        )
        .arg(
            clap::Arg::new("component-spec")
                .long("component-spec")
                .value_name("component-spec")
                .required(true)
                .help("Component Spec to attach the role under"),
        )
        .arg(
            clap::Arg::new("kind")
                .long("kind")
                .value_name("kind")
                .default_value("singleton")
                .help("Canister kind: singleton, shard, replica, or instance"),
        )
        .arg(
            flag_arg("dry-run")
                .long("dry-run")
                .help("Validate and print planned config writes without changing files"),
        )
        .after_help(APP_ROLE_ATTACH_HELP_AFTER)
}

pub(super) fn app_role_rename_command() -> ClapCommand {
    ClapCommand::new("rename")
        .bin_name("canic app role rename")
        .about("Rename a declared app role")
        .disable_help_flag(true)
        .arg(
            value_arg("app")
                .value_name("app")
                .required(true)
                .help("Config-defined app name"),
        )
        .arg(
            value_arg("old-role")
                .value_name("old-role")
                .required(true)
                .help("Existing local role name"),
        )
        .arg(
            value_arg("new-role")
                .value_name("new-role")
                .required(true)
                .help("New local role name"),
        )
        .arg(flag_arg("dry-run").long("dry-run").help(
            "Validate and print planned config/package metadata writes without changing files",
        ))
        .after_help(APP_ROLE_RENAME_HELP_AFTER)
}

pub(super) fn app_role_list_command() -> ClapCommand {
    ClapCommand::new("list")
        .bin_name("canic app role list")
        .about("List declared app roles")
        .disable_help_flag(true)
        .arg(
            value_arg("app")
                .value_name("app")
                .required(true)
                .help("Config-defined app name"),
        )
        .after_help(APP_ROLE_LIST_HELP_AFTER)
}

pub(super) fn app_role_inspect_command() -> ClapCommand {
    ClapCommand::new("inspect")
        .bin_name("canic app role inspect")
        .about("Inspect one declared app role")
        .disable_help_flag(true)
        .arg(
            value_arg("app")
                .value_name("app")
                .required(true)
                .help("Config-defined app name"),
        )
        .arg(
            value_arg("role")
                .value_name("role")
                .required(true)
                .help("Local role name"),
        )
        .after_help(APP_ROLE_INSPECT_HELP_AFTER)
}

pub(super) fn app_list_command() -> ClapCommand {
    ClapCommand::new("list")
        .bin_name("canic app list")
        .about("List config-defined Canic apps")
        .disable_help_flag(true)
        .arg(internal_environment_arg())
        .after_help(APP_LIST_HELP_AFTER)
}

pub(super) fn app_check_command() -> ClapCommand {
    ClapCommand::new("check")
        .bin_name("canic app check")
        .about("Check icp.yaml for one Canic app")
        .disable_help_flag(true)
        .arg(
            value_arg("app")
                .value_name("name")
                .required(true)
                .help("Config-defined app name to check"),
        )
        .after_help(APP_CHECK_HELP_AFTER)
}

pub(super) fn app_delete_command() -> ClapCommand {
    ClapCommand::new("delete")
        .bin_name("canic app delete")
        .about("Delete a config-defined Canic app directory")
        .disable_help_flag(true)
        .arg(
            value_arg("app")
                .value_name("name")
                .required(true)
                .help("Config-defined app name to delete"),
        )
        .arg(
            flag_arg("dry-run")
                .long("dry-run")
                .help("Validate and print the delete target without removing files"),
        )
        .after_help(APP_DELETE_HELP_AFTER)
}

pub(super) fn usage() -> String {
    render_usage(app_command)
}

pub(super) fn list_usage() -> String {
    render_usage(app_list_command)
}

pub(super) fn check_usage() -> String {
    render_usage(app_check_command)
}

pub(super) fn create_usage() -> String {
    scaffold::app_create_usage()
}

pub(super) fn delete_usage() -> String {
    render_usage(app_delete_command)
}

pub(super) fn role_usage() -> String {
    render_usage(app_role_command)
}

pub(super) fn role_list_usage() -> String {
    render_usage(app_role_list_command)
}

pub(super) fn role_inspect_usage() -> String {
    render_usage(app_role_inspect_command)
}

pub(super) fn role_declare_usage() -> String {
    render_usage(app_role_declare_command)
}

pub(super) fn role_attach_usage() -> String {
    render_usage(app_role_attach_command)
}

pub(super) fn role_rename_usage() -> String {
    render_usage(app_role_rename_command)
}
