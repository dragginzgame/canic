use super::{
    DeployCommandError, current_observed_at,
    output_format::{CompareOutputFormat, parse_compare_output_format},
    print_json, read_json_file, value_arg,
};
use crate::{
    cli::{
        clap::{parse_matches, render_usage, required_path, string_option, typed_option},
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::deployment_truth::{
    DeploymentCheckV1, DeploymentComparisonReportV1, deployment_comparison_report_from_checks,
    deployment_comparison_report_text, validate_deployment_comparison_report,
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

const DEPLOY_COMPARE_HELP_AFTER: &str = "\
Examples:
  canic deploy compare --left staging-check.json --right prod-check.json
  canic deploy compare --left staging-check.json --right prod-check.json --format text

Compares two existing DeploymentCheckV1 JSON artifacts. It does not query live
state, install code, or mutate deployments. Each input check's embedded
diff/report is revalidated against its plan and inventory before comparison
status is rendered.";

const LEFT_ARG: &str = "left";
const RIGHT_ARG: &str = "right";
const LEFT_LABEL_ARG: &str = "left-label";
const RIGHT_LABEL_ARG: &str = "right-label";
const FORMAT_ARG: &str = "format";

///
/// DeployCompareOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeployCompareOptions {
    pub(super) left: PathBuf,
    pub(super) right: PathBuf,
    pub(super) left_label: Option<String>,
    pub(super) right_label: Option<String>,
    pub(super) format: CompareOutputFormat,
}

pub(super) fn run<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = DeployCompareOptions::parse(args)?;
    let format = options.format;
    let report = build_report(options)?;
    match format {
        CompareOutputFormat::Json => print_json(&report)?,
        CompareOutputFormat::Text => println!("{}", deployment_comparison_report_text(&report)),
    }
    Ok(())
}

fn build_report(
    options: DeployCompareOptions,
) -> Result<DeploymentComparisonReportV1, DeployCommandError> {
    let left = read_json_file::<DeploymentCheckV1>(&options.left)?;
    let right = read_json_file::<DeploymentCheckV1>(&options.right)?;
    build_report_from_checks(
        &left,
        &right,
        options.left_label.as_deref(),
        options.right_label.as_deref(),
    )
}

pub(super) fn build_report_from_checks(
    left: &DeploymentCheckV1,
    right: &DeploymentCheckV1,
    left_label: Option<&str>,
    right_label: Option<&str>,
) -> Result<DeploymentComparisonReportV1, DeployCommandError> {
    let left_label = left_label.unwrap_or(left.plan.deployment_identity.deployment_name.as_str());
    let right_label =
        right_label.unwrap_or(right.plan.deployment_identity.deployment_name.as_str());
    let report = deployment_comparison_report_from_checks(
        local_report_id(left_label, right_label),
        current_observed_at()?,
        left_label,
        right_label,
        left,
        right,
    );
    validate_deployment_comparison_report(&report)
        .map_err(|err| DeployCommandError::Check(Box::new(err)))?;
    Ok(report)
}

fn local_report_id(left_label: &str, right_label: &str) -> String {
    format!("local:{left_label}:{right_label}:deployment-comparison")
}

impl DeployCompareOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        Ok(Self {
            left: required_path(&matches, LEFT_ARG),
            right: required_path(&matches, RIGHT_ARG),
            left_label: string_option(&matches, LEFT_LABEL_ARG),
            right_label: string_option(&matches, RIGHT_LABEL_ARG),
            format: typed_option(&matches, FORMAT_ARG).unwrap_or(CompareOutputFormat::Json),
        })
    }
}

pub(super) fn command() -> ClapCommand {
    ClapCommand::new("compare")
        .bin_name("canic deploy compare")
        .about("Compare two deployment truth check artifacts")
        .disable_help_flag(true)
        .override_usage("canic deploy compare --left <file> --right <file>")
        .arg(input_file_arg(
            LEFT_ARG,
            "Left DeploymentCheckV1 JSON artifact",
        ))
        .arg(input_file_arg(
            RIGHT_ARG,
            "Right DeploymentCheckV1 JSON artifact",
        ))
        .arg(label_arg(
            LEFT_LABEL_ARG,
            "Optional display label for the left artifact",
        ))
        .arg(label_arg(
            RIGHT_LABEL_ARG,
            "Optional display label for the right artifact",
        ))
        .arg(format_arg())
        .after_help(DEPLOY_COMPARE_HELP_AFTER)
}

fn input_file_arg(name: &'static str, help: &'static str) -> clap::Arg {
    value_arg(name)
        .long(name)
        .value_name("file")
        .required(true)
        .help(help)
}

fn label_arg(name: &'static str, help: &'static str) -> clap::Arg {
    value_arg(name).long(name).value_name("label").help(help)
}

fn format_arg() -> clap::Arg {
    value_arg(FORMAT_ARG)
        .long(FORMAT_ARG)
        .value_name("json|text")
        .num_args(1)
        .value_parser(clap::builder::ValueParser::new(parse_compare_output_format))
        .help("Output format; defaults to json")
}

pub(super) fn usage() -> String {
    render_usage(command)
}
