use super::value_arg;
use crate::cli::{
    clap::{passthrough_subcommand, render_usage},
    globals::internal_environment_arg,
};
use canic_host::canister_build::CanisterBuildProfile;
use clap::Command as ClapCommand;

pub(super) const FLEET_ARG: &str = "fleet";
pub(super) const PROFILE_ARG: &str = "profile";

#[derive(Clone, Copy)]
struct DeploySubcommand {
    name: &'static str,
    about: &'static str,
}

const DEPLOY_COMMANDS: &[DeploySubcommand] = &[
    DeploySubcommand {
        name: "check",
        about: "Print the local deployment truth check",
    },
    DeploySubcommand {
        name: "inspect",
        about: "Inspect raw deployment truth artifacts",
    },
    DeploySubcommand {
        name: "plan",
        about: "Explain the deterministic plan without deployment mutation",
    },
];

const DEPLOY_HELP_AFTER: &str = "\
Examples:
  canic deploy check demo
  canic deploy plan demo --app demo --fleet-input deployments/demo.toml

Deploy commands do not perform IC update calls; fresh Fleet creation uses
`canic install`.";

pub fn deploy_command() -> ClapCommand {
    DEPLOY_COMMANDS
        .iter()
        .fold(
            ClapCommand::new("deploy")
                .bin_name("canic deploy")
                .about("Plan and check deployment truth before mutation")
                .disable_help_flag(true),
            |command, subcommand| command.subcommand(deploy_passthrough_command(*subcommand)),
        )
        .after_help(DEPLOY_HELP_AFTER)
}

pub fn deploy_truth_leaf_command(name: &'static str, about: &'static str) -> ClapCommand {
    deploy_truth_leaf_command_with_bin_name(name, format!("canic deploy {name}"), about)
}

pub(super) fn deploy_truth_leaf_command_with_bin_name(
    name: &'static str,
    bin_name: impl Into<String>,
    about: &'static str,
) -> ClapCommand {
    ClapCommand::new(name)
        .bin_name(bin_name.into())
        .about(about)
        .disable_help_flag(true)
        .arg(
            value_arg(FLEET_ARG)
                .value_name(FLEET_ARG)
                .required(true)
                .help("Installed Fleet name to check"),
        )
        .arg(
            value_arg(PROFILE_ARG)
                .long(PROFILE_ARG)
                .value_name("debug|fast|release")
                .num_args(1)
                .value_parser(clap::value_parser!(CanisterBuildProfile))
                .help("Expected canister wasm build profile"),
        )
        .arg(internal_environment_arg())
}

pub fn usage() -> String {
    render_usage(deploy_command)
}

fn deploy_passthrough_command(spec: DeploySubcommand) -> ClapCommand {
    passthrough_subcommand(
        ClapCommand::new(spec.name)
            .about(spec.about)
            .disable_help_flag(true),
    )
}
