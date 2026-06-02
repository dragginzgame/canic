mod command;
mod options;
mod reports;

pub(super) use command::{
    deploy_promote_artifact_identity_command, deploy_promote_catalog_verification_command,
    deploy_promote_check_command, deploy_promote_command, deploy_promote_diff_command,
    deploy_promote_execution_receipt_command, deploy_promote_inspect_command,
    deploy_promote_materialization_identity_command, deploy_promote_plan_command,
    deploy_promote_policy_check_command, deploy_promote_provenance_command,
    deploy_promote_readiness_command, deploy_promote_target_lineage_command,
    deploy_promote_transform_command, deploy_promote_transform_evidence_command,
    deploy_promote_wasm_store_identity_command, promote_artifact_identity_usage,
    promote_catalog_verification_usage, promote_check_usage, promote_diff_usage,
    promote_execution_receipt_usage, promote_inspect_usage, promote_materialization_identity_usage,
    promote_plan_usage, promote_policy_check_usage, promote_provenance_usage,
    promote_readiness_usage, promote_target_lineage_usage, promote_transform_evidence_usage,
    promote_transform_usage, promote_usage, promote_wasm_store_identity_usage,
};
pub(super) use options::DeployPromoteReportOptions;

use super::{DeployCommandError, output_format::PromotionOutputFormat, print_json, read_json_file};
use crate::{
    cli::{clap::parse_subcommand, help::print_help_or_version},
    version_text,
};
use canic_host::deployment_truth::{
    artifact_promotion_execution_receipt_text, artifact_promotion_plan_text,
    artifact_promotion_provenance_report_text, promotion_artifact_identity_report_text,
    promotion_materialization_identity_report_text, promotion_plan_transform_evidence_text,
    promotion_plan_transform_text, promotion_policy_check_text, promotion_readiness_text,
    promotion_target_execution_lineage_text, promotion_wasm_store_catalog_verification_text,
    promotion_wasm_store_identity_report_text,
};
use clap::Command as ClapCommand;
use reports::{
    build_artifact_promotion_execution_receipt, build_artifact_promotion_plan,
    build_artifact_promotion_provenance_report, build_promotion_artifact_identity_report,
    build_promotion_materialization_identity_report, build_promotion_plan_transform,
    build_promotion_plan_transform_evidence, build_promotion_policy_check,
    build_promotion_readiness, build_promotion_target_execution_lineage,
    build_promotion_wasm_store_catalog_verification, build_promotion_wasm_store_identity_report,
};
use serde::de::DeserializeOwned;
use std::ffi::OsString;

pub(super) fn run<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, promote_usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(deploy_promote_command(), args)
        .map_err(|_| DeployCommandError::Usage(promote_usage()))?
    {
        Some((command, args)) if command == "inspect" => run_inspect(args),
        Some((command, args)) if command == "plan" => run_plan(args),
        Some((command, args)) if command == "check" => run_check(args),
        Some((command, args)) if command == "diff" => run_diff(args),
        _ => {
            println!("{}", promote_usage());
            Ok(())
        }
    }
}

fn run_inspect<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, promote_inspect_usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(deploy_promote_inspect_command(), args)
        .map_err(|_| DeployCommandError::Usage(promote_inspect_usage()))?
    {
        Some((command, args)) if command == "readiness" => run_readiness(args),
        Some((command, args)) if command == "artifact-identity" => run_artifact_identity(args),
        Some((command, args)) if command == "transform" => run_transform(args),
        Some((command, args)) if command == "transform-evidence" => run_transform_evidence(args),
        Some((command, args)) if command == "target-lineage" => run_target_lineage(args),
        Some((command, args)) if command == "provenance" => run_provenance(args),
        Some((command, args)) if command == "wasm-store-identity" => run_wasm_store_identity(args),
        Some((command, args)) if command == "catalog-verification" => {
            run_catalog_verification(args)
        }
        Some((command, args)) if command == "execution-receipt" => run_execution_receipt(args),
        Some((command, args)) if command == "policy" => run_policy_check(args),
        Some((command, args)) if command == "materialization-identity" => {
            run_materialization_identity(args)
        }
        _ => {
            println!("{}", promote_inspect_usage());
            Ok(())
        }
    }
}

fn run_readiness<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        deploy_promote_readiness_command,
        promote_readiness_usage,
        build_promotion_readiness,
        promotion_readiness_text,
    )
}

fn run_check<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        deploy_promote_check_command,
        promote_check_usage,
        build_promotion_readiness,
        promotion_readiness_text,
    )
}

fn run_artifact_identity<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        deploy_promote_artifact_identity_command,
        promote_artifact_identity_usage,
        build_promotion_artifact_identity_report,
        promotion_artifact_identity_report_text,
    )
}

fn run_transform<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        deploy_promote_transform_command,
        promote_transform_usage,
        build_promotion_plan_transform,
        promotion_plan_transform_text,
    )
}

fn run_diff<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        deploy_promote_diff_command,
        promote_diff_usage,
        build_promotion_plan_transform,
        promotion_plan_transform_text,
    )
}

fn run_transform_evidence<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        deploy_promote_transform_evidence_command,
        promote_transform_evidence_usage,
        build_promotion_plan_transform_evidence,
        promotion_plan_transform_evidence_text,
    )
}

fn run_target_lineage<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        deploy_promote_target_lineage_command,
        promote_target_lineage_usage,
        build_promotion_target_execution_lineage,
        promotion_target_execution_lineage_text,
    )
}

fn run_plan<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        deploy_promote_plan_command,
        promote_plan_usage,
        build_artifact_promotion_plan,
        artifact_promotion_plan_text,
    )
}

fn run_provenance<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        deploy_promote_provenance_command,
        promote_provenance_usage,
        build_artifact_promotion_provenance_report,
        artifact_promotion_provenance_report_text,
    )
}

fn run_wasm_store_identity<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        deploy_promote_wasm_store_identity_command,
        promote_wasm_store_identity_usage,
        build_promotion_wasm_store_identity_report,
        promotion_wasm_store_identity_report_text,
    )
}

fn run_catalog_verification<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        deploy_promote_catalog_verification_command,
        promote_catalog_verification_usage,
        build_promotion_wasm_store_catalog_verification,
        promotion_wasm_store_catalog_verification_text,
    )
}

fn run_execution_receipt<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        deploy_promote_execution_receipt_command,
        promote_execution_receipt_usage,
        build_artifact_promotion_execution_receipt,
        artifact_promotion_execution_receipt_text,
    )
}

fn run_policy_check<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        deploy_promote_policy_check_command,
        promote_policy_check_usage,
        build_promotion_policy_check,
        promotion_policy_check_text,
    )
}

fn run_materialization_identity<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        deploy_promote_materialization_identity_command,
        promote_materialization_identity_usage,
        build_promotion_materialization_identity_report,
        promotion_materialization_identity_report_text,
    )
}

fn run_output<I, T, R>(
    args: I,
    command: impl FnOnce() -> ClapCommand,
    usage: fn() -> String,
    build: impl FnOnce(R) -> Result<T, DeployCommandError>,
    render_text: impl FnOnce(&T) -> String,
) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
    T: serde::Serialize,
    R: DeserializeOwned,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = DeployPromoteReportOptions::parse(args, command, usage)?;
    let request = read_json_file::<R>(&options.request)?;
    let output = build(request)?;
    match options.format {
        PromotionOutputFormat::Json => print_json(&output)?,
        PromotionOutputFormat::Text => println!("{}", render_text(&output)),
    }
    Ok(())
}
