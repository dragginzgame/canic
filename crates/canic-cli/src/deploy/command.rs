use super::value_arg;
use crate::cli::{
    clap::{passthrough_subcommand, render_usage},
    globals::internal_network_arg,
};
use canic_host::canister_build::CanisterBuildProfile;
use clap::Command as ClapCommand;

pub(super) const DEPLOYMENT_ARG: &str = "deployment";
pub(super) const PROFILE_ARG: &str = "profile";

#[derive(Clone, Copy)]
struct DeploySubcommand {
    name: &'static str,
    about: &'static str,
}

const DEPLOY_COMMANDS: &[DeploySubcommand] = &[
    DeploySubcommand {
        name: "authority",
        about: "Dry-run controller authority reconciliation",
    },
    DeploySubcommand {
        name: "external",
        about: "Build passive external lifecycle reports",
    },
    DeploySubcommand {
        name: "promote",
        about: "Build passive artifact promotion reports",
    },
    DeploySubcommand {
        name: "root",
        about: "Inspect or verify deployment-root evidence",
    },
    DeploySubcommand {
        name: "plan",
        about: "Explain the deterministic deployment plan without mutation",
    },
    DeploySubcommand {
        name: "install",
        about: "Install through the current runner using a supplied deployment plan",
    },
    DeploySubcommand {
        name: "register",
        about: "Register minimal deployment-target state",
    },
    DeploySubcommand {
        name: "check",
        about: "Print the local deployment truth check",
    },
    DeploySubcommand {
        name: "inspect",
        about: "Inspect raw deployment truth artifacts",
    },
];

const DEPLOY_HELP_AFTER: &str = "\
Examples:
  canic deploy check demo
  canic deploy check demo --format text
  canic deploy plan demo
  canic deploy plan demo --json
  canic deploy inspect plan demo
  canic deploy inspect compare --left staging-check.json --right prod-check.json
  canic deploy inspect catalog list
  canic deploy inspect root --request root-verification.json
  canic deploy inspect resume-report --receipt receipt.json demo
  canic deploy register demo --fleet-template demo --root aaaaa-aa --allow-unverified
  canic deploy install demo-local --plan promoted-plan.json
  canic deploy authority check demo
  canic deploy external plan demo
  canic deploy promote plan --request promotion-plan.json
  canic deploy root verify demo-local --from-check deployment-check.json

Use `canic deploy inspect help` for raw plan, inventory, diff, report,
comparison, local catalog, root-verification, and resume-safety JSON artifacts.
Use `canic deploy plan <deployment>` for the operator planning report.
Use `canic inspect` for live runtime-observed canister status.
Plan-mediated deployment-target mutation flows through `canic deploy install
<deployment> --plan <file>`. `canic install <fleet>` remains the fleet-template
bootstrap entrypoint.";

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
            value_arg(DEPLOYMENT_ARG)
                .value_name(DEPLOYMENT_ARG)
                .required(true)
                .help("Deployment target name to check"),
        )
        .arg(
            value_arg(PROFILE_ARG)
                .long(PROFILE_ARG)
                .value_name("debug|fast|release")
                .num_args(1)
                .value_parser(clap::value_parser!(CanisterBuildProfile))
                .help("Expected canister wasm build profile"),
        )
        .arg(internal_network_arg())
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
