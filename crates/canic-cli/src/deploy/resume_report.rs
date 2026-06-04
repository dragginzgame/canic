use super::{
    DeployCommandError, DeployTruthOptions, deploy_truth_leaf_command, load_deployment_check,
    print_json, read_json_file, value_arg,
};
use crate::{
    cli::{
        clap::{parse_matches, path_option},
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::{
    deployment_truth::{DeploymentReceiptV1, compare_plan_inventory_and_receipt},
    icp_config::resolve_current_canic_icp_root,
    install_root::latest_deployment_truth_receipt_path_from_root,
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

const DEPLOY_RESUME_REPORT_HELP_AFTER: &str = "\
Examples:
  canic deploy resume-report demo
  canic deploy resume-report --receipt receipt.json demo
  canic --network local deploy resume-report --receipt receipt.json --profile fast demo

Prints the passive ResumeSafetyV1 JSON for the current deployment truth check
and a prior DeploymentReceiptV1. When --receipt is omitted, Canic uses the
latest local receipt under .canic/<network>/deployment-receipts/<deployment>. It
does not resume, install, or mutate state.";

const RECEIPT_ARG: &str = "receipt";

///
/// DeployResumeReportOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeployResumeReportOptions {
    pub(super) truth: DeployTruthOptions,
    pub(super) receipt: Option<PathBuf>,
}

pub(super) fn run<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = DeployResumeReportOptions::parse(args)?;
    let receipt_path = options.receipt_path()?;
    let receipt = read_deployment_receipt(&receipt_path)?;
    let check = load_deployment_check(options.truth)?;
    let diff = compare_plan_inventory_and_receipt(&check.plan, &check.inventory, &receipt);
    print_json(&diff.resume_safety)?;
    Ok(())
}

impl DeployResumeReportOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        Ok(Self {
            truth: DeployTruthOptions::from_matches(&matches),
            receipt: path_option(&matches, RECEIPT_ARG),
        })
    }

    fn receipt_path(&self) -> Result<PathBuf, DeployCommandError> {
        if let Some(path) = &self.receipt {
            return Ok(path.clone());
        }

        let icp_root = resolve_current_canic_icp_root().map_err(|err| {
            DeployCommandError::Usage(format!(
                "could not discover current Canic project root for latest deployment receipt: {err}; pass --receipt <file>"
            ))
        })?;

        latest_deployment_truth_receipt_path_from_root(
            &icp_root,
            &self.truth.network,
            &self.truth.deployment,
        )
        .map_err(DeployCommandError::from)?
        .ok_or_else(|| {
            DeployCommandError::Usage(format!(
                "no deployment receipt found under {} for deployment {}; pass --receipt <file>",
                icp_root
                    .join(".canic")
                    .join(&self.truth.network)
                    .join("deployment-receipts")
                    .join(&self.truth.deployment)
                    .display(),
                self.truth.deployment
            ))
        })
    }
}

pub(super) fn command() -> ClapCommand {
    deploy_truth_leaf_command(
        "resume-report",
        "Print passive resume safety JSON from a prior deployment receipt",
    )
    .arg(receipt_arg())
    .after_help(DEPLOY_RESUME_REPORT_HELP_AFTER)
}

fn receipt_arg() -> clap::Arg {
    value_arg(RECEIPT_ARG)
        .long(RECEIPT_ARG)
        .value_name("file")
        .help("DeploymentReceiptV1 JSON file to compare with current deployment truth")
}

pub(super) fn usage() -> String {
    render_usage(command)
}

fn render_usage(command: fn() -> ClapCommand) -> String {
    let mut command = command();
    command.render_help().to_string()
}

fn read_deployment_receipt(path: &PathBuf) -> Result<DeploymentReceiptV1, DeployCommandError> {
    read_json_file(path)
}
