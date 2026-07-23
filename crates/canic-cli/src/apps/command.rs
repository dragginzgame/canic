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
use canic_host::adoption::AdoptionProfileV1;
use clap::Command as ClapCommand;

const APP_HELP_AFTER: &str = "\
Examples:
  canic app list
  canic app adoption report demo --profile brownfield
  canic app role declare demo store --package store
  canic app role attach demo store --subnet default
  canic app role rename demo hub router
  canic app role list demo
  canic app role inspect demo app
  canic app config demo
  canic app create demo
  canic app check test
  canic app delete demo

Mutation notes:
  canic app check/list/config/adoption/role list/role inspect are read-only.
  canic app create writes new local source/config files.
  canic app role declare/attach/rename update canic.toml; rename may also
  update matching package metadata.
  canic app delete removes the selected app directory.
  Mutating app commands that can be previewed expose --dry-run.";
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
  canic app role declare demo store --package store
  canic app role attach demo store --subnet default
  canic app role rename demo hub router
  canic app role list demo
  canic app role inspect demo app

Mutation notes:
  list and inspect are read-only.
  declare and attach update canic.toml.
  rename updates canic.toml and may update matching package metadata.
  declare, attach, and rename support --dry-run.";
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
  canic app role attach demo store --subnet default
  canic app role attach demo worker --subnet default --kind replica
  canic app role attach demo store --subnet default --dry-run";
const APP_ROLE_RENAME_HELP_AFTER: &str = "\
Examples:
  canic app role rename demo hub router
  canic app role rename demo hub router --dry-run";
const APP_ADOPTION_HELP_AFTER: &str = "\
Examples:
  canic app adoption report demo --profile brownfield
  canic app adoption report demo --profile minimal --json
  canic app adoption report demo --profile minimal --evidence-envelope

Adoption commands are read-only. They report recommendations and never update
app config, package manifests, topology, deployments, or controllers.";
const APP_ADOPTION_REPORT_HELP_AFTER: &str = "\
Examples:
  canic app adoption report demo --profile brownfield
  canic app adoption report demo --profile minimal --json
  canic app adoption report demo --profile minimal --evidence-envelope
  canic app adoption report demo --profile partial --deployment-check check.json
  canic app adoption report demo --profile partial --inventory inventory.json
  canic app adoption report demo --profile partial --cargo-metadata cargo-metadata.json
  canic app adoption report demo --profile partial --evidence-envelope --build-provenance build-provenance.json
  canic app adoption report demo --profile partial --output adoption-report.txt

Profiles: brownfield, partial, standalone, leaf-only, hybrid-external-wasm,
minimal. --json emits the raw experimental adoption report payload.
--evidence-envelope emits the stable CI/GitOps evidence envelope with the raw
adoption payload nested inside. The report is read-only; --output writes only
the requested report artifact. Evidence inputs are JSON files and are
read-only. Use either --inventory or --deployment-check, not both. Use either
--package-metadata or --cargo-metadata, not both. Deployment-check evidence
also supplies plan role artifacts when present. --build-provenance is
fingerprinted only in envelope output.";
pub(super) const JSON_ARG: &str = "json";
pub(super) const EVIDENCE_ENVELOPE_ARG: &str = "evidence-envelope";

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
            ClapCommand::new("create")
                .about("Create a minimal Canic app")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("list")
                .about("List config-defined Canic apps")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("config")
                .about("Inspect selected app config")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("adoption")
                .about("Report safe onboarding recommendations")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("role")
                .about("Manage app role lifecycle")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("delete")
                .about("Delete a config-defined Canic app")
                .disable_help_flag(true),
        ))
        .after_help(APP_HELP_AFTER)
}

pub(super) fn app_adoption_command() -> ClapCommand {
    ClapCommand::new("adoption")
        .bin_name("canic app adoption")
        .about("Report safe onboarding recommendations")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("report")
                .about("Generate a read-only adoption report")
                .disable_help_flag(true),
        ))
        .after_help(APP_ADOPTION_HELP_AFTER)
}

pub(super) fn app_adoption_report_command() -> ClapCommand {
    ClapCommand::new("report")
        .bin_name("canic app adoption report")
        .about("Generate a read-only adoption report")
        .disable_help_flag(true)
        .arg(
            value_arg("app")
                .value_name("app")
                .required(true)
                .help("Config-defined app name"),
        )
        .arg(
            clap::Arg::new("profile")
                .long("profile")
                .value_name("profile")
                .required(true)
                .value_parser(clap::value_parser!(AdoptionProfileV1))
                .help("Adoption profile to evaluate"),
        )
        .arg(
            flag_arg(JSON_ARG)
                .long(JSON_ARG)
                .conflicts_with(EVIDENCE_ENVELOPE_ARG)
                .help("Print raw adoption report JSON output"),
        )
        .arg(
            flag_arg(EVIDENCE_ENVELOPE_ARG)
                .long(EVIDENCE_ENVELOPE_ARG)
                .help("Print the stable CI/GitOps evidence envelope"),
        )
        .arg(
            clap::Arg::new("inventory")
                .long("inventory")
                .value_name("path")
                .conflicts_with("deployment-check")
                .help("Read DeploymentInventoryV1 JSON evidence from this path"),
        )
        .arg(
            clap::Arg::new("deployment-check")
                .long("deployment-check")
                .value_name("path")
                .help("Read inventory evidence from a DeploymentCheckV1 JSON artifact"),
        )
        .arg(
            clap::Arg::new("artifact-manifest")
                .long("artifact-manifest")
                .value_name("path")
                .help("Read RoleArtifactManifestV1 JSON evidence from this path"),
        )
        .arg(
            clap::Arg::new("package-metadata")
                .long("package-metadata")
                .value_name("path")
                .conflicts_with("cargo-metadata")
                .help("Read AdoptionPackageMetadataV1 JSON array evidence from this path"),
        )
        .arg(
            clap::Arg::new("cargo-metadata")
                .long("cargo-metadata")
                .value_name("path")
                .help("Read package metadata evidence from cargo metadata JSON"),
        )
        .arg(
            clap::Arg::new("build-provenance")
                .long("build-provenance")
                .value_name("path")
                .help(
                    "Fingerprint a BuildProvenanceV1 evidence envelope; requires --evidence-envelope",
                ),
        )
        .arg(
            clap::Arg::new("output")
                .long("output")
                .value_name("path")
                .help("Write the report artifact to this path"),
        )
        .after_help(APP_ADOPTION_REPORT_HELP_AFTER)
}

pub(super) fn app_role_command() -> ClapCommand {
    ClapCommand::new("role")
        .bin_name("canic app role")
        .about("Manage app role lifecycle")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("declare")
                .about("Declare an existing package-backed role")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("attach")
                .about("Attach a declared role to direct topology")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("rename")
                .about("Rename a declared app role")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("list")
                .about("List declared app roles")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("inspect")
                .about("Inspect one declared app role")
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
            clap::Arg::new("subnet")
                .long("subnet")
                .value_name("subnet")
                .required(true)
                .help("Subnet to attach the role under"),
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

pub(super) fn adoption_usage() -> String {
    render_usage(app_adoption_command)
}

pub(super) fn adoption_report_usage() -> String {
    render_usage(app_adoption_report_command)
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
