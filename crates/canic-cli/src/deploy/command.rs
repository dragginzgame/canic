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
        name: "catalog",
        about: "List or inspect known deployment targets",
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
        name: "install",
        about: "Install through the current runner using a supplied deployment plan",
    },
    DeploySubcommand {
        name: "register",
        about: "Register minimal deployment-target state",
    },
    DeploySubcommand {
        name: "compare",
        about: "Compare two deployment truth check artifacts",
    },
    DeploySubcommand {
        name: "check",
        about: "Print the local deployment truth check JSON",
    },
    DeploySubcommand {
        name: "diff",
        about: "Print the local deployment diff JSON",
    },
    DeploySubcommand {
        name: "inventory",
        about: "Print the local deployment inventory JSON",
    },
    DeploySubcommand {
        name: "plan",
        about: "Print the local deployment plan JSON",
    },
    DeploySubcommand {
        name: "report",
        about: "Print the local deployment safety report JSON",
    },
    DeploySubcommand {
        name: "resume-report",
        about: "Print passive resume safety JSON from a receipt",
    },
];

const DEPLOY_HELP_AFTER: &str = "\
Examples:
  canic deploy plan demo
  canic deploy inventory demo
  canic deploy register demo --fleet-template demo --root aaaaa-aa --allow-unverified
  canic deploy compare --left staging-check.json --right prod-check.json
  canic deploy diff demo
  canic deploy report demo
  canic deploy check demo
  canic deploy catalog list
  canic deploy catalog inspect demo-local
  canic deploy authority check demo
  canic deploy authority evidence demo
  canic deploy authority report demo
  canic deploy authority receipt demo
  canic deploy external plan demo
  canic deploy external check demo
  canic deploy external handoff demo
  canic deploy external proposals demo
  canic deploy external pending demo
  canic deploy external critical-fix --fix-id fix-2026-05 --severity critical demo
  canic deploy external inspect consent --request external-consent.json
  canic deploy external inspect verification-policy --request external-verification-policy.json
  canic deploy external inspect verification-check --request external-verification-check.json
  canic deploy external inspect completion --request external-completion.json
  canic deploy external verify --request external-verification.json
  canic deploy root inspect --request root-verification.json
  canic deploy root verify demo-local --from-check deployment-check.json
  canic deploy promote plan --request promotion-plan.json
  canic deploy promote check --request promotion-check.json
  canic deploy promote diff --request promotion-diff.json
  canic deploy install demo-local --plan promoted-plan.json
  canic deploy promote inspect readiness --request promotion-readiness.json
  canic deploy promote inspect artifact-identity --request promotion-artifacts.json
  canic deploy promote inspect provenance --request promotion-provenance.json
  canic deploy resume-report demo
  canic deploy resume-report --receipt receipt.json demo
  canic deploy check --profile fast demo

Deployment truth commands are read-only checks. Plan-mediated deployment-target
mutation flows through `canic deploy install <deployment> --plan <file>`.
`canic install <fleet>` remains the fleet-template bootstrap entrypoint.
Authority commands are dry-run reconciliation reports and do not mutate
controller state.";

pub fn deploy_command() -> ClapCommand {
    DEPLOY_COMMANDS
        .iter()
        .fold(
            ClapCommand::new("deploy")
                .bin_name("canic deploy")
                .about("Check deployment truth before mutation")
                .disable_help_flag(true),
            |command, subcommand| command.subcommand(deploy_passthrough_command(*subcommand)),
        )
        .after_help(DEPLOY_HELP_AFTER)
}

pub fn deploy_truth_leaf_command(name: &'static str, about: &'static str) -> ClapCommand {
    ClapCommand::new(name)
        .bin_name(format!("canic deploy {name}"))
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
