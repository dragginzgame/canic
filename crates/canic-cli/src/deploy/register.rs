use super::{DeployCommandError, value_arg};
use crate::{
    cli::{
        clap::{parse_matches, string_option},
        defaults::local_network,
        globals::internal_network_arg,
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::install_root::{RegisterDeploymentStateOptions, register_deployment_state};
use clap::{ArgAction, Command as ClapCommand};
use std::{ffi::OsString, path::PathBuf};

const DEPLOY_REGISTER_HELP_AFTER: &str = "\
Examples:
  canic deploy register demo --fleet-template demo --root aaaaa-aa --allow-unverified
  canic --network local deploy register demo --fleet-template demo --root uxrrr-q7777-77774-qaaaq-cai --allow-unverified

Registers minimal deployment-target local state for an existing root canister.
This is an explicit 0.46 hard-cut recovery path. It does not migrate legacy
fleet-template state, query live inventory, copy receipts, record
artifact/controller truth, install code, or mutate canisters. Registered roots are marked
not_verified until a later verification path records live evidence. The
--allow-unverified flag is required so unverified registration remains an
explicit operator acknowledgement.";

const DEPLOYMENT_ARG: &str = "deployment";
const FLEET_TEMPLATE_ARG: &str = "fleet-template";
const ROOT_ARG: &str = "root";
const ALLOW_UNVERIFIED_ARG: &str = "allow-unverified";

///
/// DeployRegisterOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeployRegisterOptions {
    pub(super) deployment: String,
    pub(super) fleet_template: String,
    pub(super) root: String,
    pub(super) network: String,
    pub(super) allow_unverified: bool,
}

pub(super) fn run<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = DeployRegisterOptions::parse(args)?;
    let state_path = register_deployment_state(options.into_register_options(None))
        .map_err(DeployCommandError::from)?;
    println!("Registered deployment state: {}", state_path.display());
    println!("root_verification: not_verified");
    Ok(())
}

impl DeployRegisterOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        Ok(Self {
            deployment: string_option(&matches, DEPLOYMENT_ARG).expect("clap requires deployment"),
            fleet_template: string_option(&matches, FLEET_TEMPLATE_ARG)
                .expect("clap requires fleet-template"),
            root: string_option(&matches, ROOT_ARG).expect("clap requires root"),
            network: string_option(&matches, "network").unwrap_or_else(local_network),
            allow_unverified: matches.get_flag(ALLOW_UNVERIFIED_ARG),
        })
    }

    pub(super) fn into_register_options(
        self,
        icp_root: Option<PathBuf>,
    ) -> RegisterDeploymentStateOptions {
        RegisterDeploymentStateOptions {
            deployment_name: self.deployment,
            fleet_template: self.fleet_template,
            root_canister_id: self.root,
            network: self.network,
            allow_unverified: self.allow_unverified,
            icp_root,
            workspace_root: None,
        }
    }
}

pub(super) fn command() -> ClapCommand {
    ClapCommand::new("register")
        .bin_name("canic deploy register")
        .about("Register minimal deployment-target state")
        .disable_help_flag(true)
        .override_usage(
            "canic deploy register <deployment> --fleet-template <fleet> --root <principal> --allow-unverified",
        )
        .arg(deployment_arg())
        .arg(fleet_template_arg())
        .arg(root_arg())
        .arg(allow_unverified_arg())
        .arg(internal_network_arg())
        .after_help(DEPLOY_REGISTER_HELP_AFTER)
}

fn deployment_arg() -> clap::Arg {
    value_arg(DEPLOYMENT_ARG)
        .required(true)
        .help("Deployment target name to register")
}

fn fleet_template_arg() -> clap::Arg {
    value_arg(FLEET_TEMPLATE_ARG)
        .long(FLEET_TEMPLATE_ARG)
        .value_name("fleet")
        .required(true)
        .help("Reusable fleet template this deployment target uses")
}

fn root_arg() -> clap::Arg {
    value_arg(ROOT_ARG)
        .long(ROOT_ARG)
        .value_name("principal")
        .required(true)
        .help("Existing root canister principal for this deployment")
}

fn allow_unverified_arg() -> clap::Arg {
    clap::Arg::new(ALLOW_UNVERIFIED_ARG)
        .long(ALLOW_UNVERIFIED_ARG)
        .action(ArgAction::SetTrue)
        .required(true)
        .help("Acknowledge that the registered root is not live-verified")
}

pub(super) fn usage() -> String {
    render_usage(command)
}

fn render_usage(command: fn() -> ClapCommand) -> String {
    let mut command = command();
    command.render_help().to_string()
}
