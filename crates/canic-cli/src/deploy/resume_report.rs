use super::{
    DeployCommandError, DeployTruthOptions, command::deploy_truth_leaf_command_with_bin_name,
    load_deployment_check, print_json, read_json_file, value_arg,
};
use crate::{
    cli::{
        clap::{parse_matches, path_option, render_usage},
        help::print_help_or_version,
    },
    version_text,
};
use canic_core::ids::FleetKey;
use canic_host::{
    deployment_truth::{DeploymentReceiptV1, compare_plan_inventory_and_receipt},
    fleet_catalog::read_fleet_catalog_entry_from_root,
    icp_config::resolve_current_canic_icp_root,
    install_root::latest_deployment_truth_receipt_path_from_root,
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

const DEPLOY_RESUME_REPORT_HELP_AFTER: &str = "\
Examples:
  canic deploy inspect resume-report demo
  canic deploy inspect resume-report --receipt receipt.json demo
  canic --environment local deploy inspect resume-report --receipt receipt.json --profile fast demo

Prints the passive ResumeSafetyV1 JSON for the current deployment truth check
and a prior DeploymentReceiptV1. When --receipt is omitted, Canic uses the
latest receipt under
.canic/networks/<canonical-network-id>/fleets/<fleet-id>/deployment-receipts/.
The Fleet name is resolved through the canonical catalog. It does not resume,
install, or mutate state.";

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
    let receipt: DeploymentReceiptV1 = read_json_file(&receipt_path)?;
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
        self.receipt_path_from_root(&icp_root)
    }

    pub(super) fn receipt_path_from_root(
        &self,
        icp_root: &std::path::Path,
    ) -> Result<PathBuf, DeployCommandError> {
        let fleet = read_fleet_catalog_entry_from_root(
            icp_root,
            &self.truth.environment,
            &self.truth.fleet,
        )
        .map_err(|error| DeployCommandError::Check(Box::new(error)))?
        .ok_or_else(|| {
            DeployCommandError::Usage(format!(
                "Fleet {} is not present in the canonical catalog for environment profile {}; pass --receipt <file>",
                self.truth.fleet, self.truth.environment
            ))
        })?;
        let fleet = FleetKey {
            canonical_network_id: fleet.canonical_network_id,
            fleet_id: fleet.fleet_id,
        };

        latest_deployment_truth_receipt_path_from_root(icp_root, fleet)
            .map_err(DeployCommandError::from)?
            .ok_or_else(|| {
                DeployCommandError::Usage(format!(
                    "no deployment receipt found under {} for Fleet {}; pass --receipt <file>",
                    icp_root
                        .join(".canic")
                        .join("networks")
                        .join(fleet.canonical_network_id.to_string())
                        .join("fleets")
                        .join(fleet.fleet_id.to_string())
                        .join("deployment-receipts")
                        .display(),
                    self.truth.fleet
                ))
            })
    }
}

pub(super) fn command() -> ClapCommand {
    deploy_truth_leaf_command_with_bin_name(
        "resume-report",
        "canic deploy inspect resume-report",
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
