use super::{
    DeployCommandError, DeployTruthOptions, deploy_truth_leaf_command, load_deployment_check,
    print_json,
};
use crate::{cli::help::print_help_or_version, version_text};
use canic_host::deployment_truth::DeploymentCheckV1;
use clap::Command as ClapCommand;
use std::ffi::OsString;

#[derive(Clone, Copy)]
struct TruthCommand {
    name: &'static str,
    about: &'static str,
    help_after: &'static str,
}

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

const PLAN_COMMAND: TruthCommand = TruthCommand {
    name: "plan",
    about: "Print the local deployment plan JSON",
    help_after: DEPLOY_PLAN_HELP_AFTER,
};
const INVENTORY_COMMAND: TruthCommand = TruthCommand {
    name: "inventory",
    about: "Print the local deployment inventory JSON",
    help_after: DEPLOY_INVENTORY_HELP_AFTER,
};
const DIFF_COMMAND: TruthCommand = TruthCommand {
    name: "diff",
    about: "Print the local deployment diff JSON",
    help_after: DEPLOY_DIFF_HELP_AFTER,
};
const REPORT_COMMAND: TruthCommand = TruthCommand {
    name: "report",
    about: "Print the local deployment safety report JSON",
    help_after: DEPLOY_REPORT_HELP_AFTER,
};

pub(super) fn run_plan<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(args, plan_command, plan_usage, |check| &check.plan)
}

pub(super) fn run_inventory<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(args, inventory_command, inventory_usage, |check| {
        &check.inventory
    })
}

pub(super) fn run_diff<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(args, diff_command, diff_usage, |check| &check.diff)
}

pub(super) fn run_report<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(args, report_command, report_usage, |check| &check.report)
}

fn run_output<I, T>(
    args: I,
    command: impl FnOnce() -> ClapCommand,
    usage: fn() -> String,
    select: impl for<'a> FnOnce(&'a DeploymentCheckV1) -> &'a T,
) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
    T: serde::Serialize,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(args, command, usage)?)?;
    print_json(select(&check))?;
    Ok(())
}

pub(super) fn plan_command() -> ClapCommand {
    truth_command(PLAN_COMMAND)
}

pub(super) fn inventory_command() -> ClapCommand {
    truth_command(INVENTORY_COMMAND)
}

pub(super) fn diff_command() -> ClapCommand {
    truth_command(DIFF_COMMAND)
}

pub(super) fn report_command() -> ClapCommand {
    truth_command(REPORT_COMMAND)
}

fn truth_command(spec: TruthCommand) -> ClapCommand {
    deploy_truth_leaf_command(spec.name, spec.about).after_help(spec.help_after)
}

pub(super) fn plan_usage() -> String {
    render_usage(plan_command)
}

pub(super) fn inventory_usage() -> String {
    render_usage(inventory_command)
}

pub(super) fn diff_usage() -> String {
    render_usage(diff_command)
}

pub(super) fn report_usage() -> String {
    render_usage(report_command)
}

fn render_usage(command: fn() -> ClapCommand) -> String {
    let mut command = command();
    command.render_help().to_string()
}
