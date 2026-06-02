use super::{
    DeployCommandError, DeployTruthOptions, deploy_truth_leaf_command, load_deployment_check,
    print_json,
};
use crate::{cli::help::print_help_or_version, version_text};
use clap::Command as ClapCommand;
use std::ffi::OsString;

const DEPLOY_PLAN_HELP_AFTER: &str = "\
Examples:
  canic deploy plan demo
  canic --network local deploy plan --profile fast demo

Prints the local DeploymentPlanV1 JSON without installing or mutating state.";
const DEPLOY_INVENTORY_HELP_AFTER: &str = "\
Examples:
  canic deploy inventory demo
  canic --network local deploy inventory --profile fast demo

Prints the local DeploymentInventoryV1 JSON without installing or mutating state.";
const DEPLOY_DIFF_HELP_AFTER: &str = "\
Examples:
  canic deploy diff demo
  canic --network local deploy diff --profile fast demo

Prints the local DeploymentDiffV1 JSON without installing or mutating state.";
const DEPLOY_REPORT_HELP_AFTER: &str = "\
Examples:
  canic deploy report demo
  canic --network local deploy report --profile fast demo

Prints the local SafetyReportV1 JSON without installing or mutating state.";

pub(super) fn run_plan<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, plan_usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(args, plan_command, plan_usage)?)?;
    print_json(&check.plan)?;
    Ok(())
}

pub(super) fn run_inventory<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, inventory_usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(
        args,
        inventory_command,
        inventory_usage,
    )?)?;
    print_json(&check.inventory)?;
    Ok(())
}

pub(super) fn run_diff<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, diff_usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(args, diff_command, diff_usage)?)?;
    print_json(&check.diff)?;
    Ok(())
}

pub(super) fn run_report<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, report_usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(
        args,
        report_command,
        report_usage,
    )?)?;
    print_json(&check.report)?;
    Ok(())
}

pub(super) fn plan_command() -> ClapCommand {
    deploy_truth_leaf_command("plan", "Print the local deployment plan JSON")
        .after_help(DEPLOY_PLAN_HELP_AFTER)
}

pub(super) fn inventory_command() -> ClapCommand {
    deploy_truth_leaf_command("inventory", "Print the local deployment inventory JSON")
        .after_help(DEPLOY_INVENTORY_HELP_AFTER)
}

pub(super) fn diff_command() -> ClapCommand {
    deploy_truth_leaf_command("diff", "Print the local deployment diff JSON")
        .after_help(DEPLOY_DIFF_HELP_AFTER)
}

pub(super) fn report_command() -> ClapCommand {
    deploy_truth_leaf_command("report", "Print the local deployment safety report JSON")
        .after_help(DEPLOY_REPORT_HELP_AFTER)
}

pub(super) fn plan_usage() -> String {
    let mut command = plan_command();
    command.render_help().to_string()
}

pub(super) fn inventory_usage() -> String {
    let mut command = inventory_command();
    command.render_help().to_string()
}

pub(super) fn diff_usage() -> String {
    let mut command = diff_command();
    command.render_help().to_string()
}

pub(super) fn report_usage() -> String {
    let mut command = report_command();
    command.render_help().to_string()
}
