mod builders;
mod command;
mod options;

pub(super) use builders::{
    build_critical_fix_report, build_lifecycle_check, build_lifecycle_handoff,
    build_lifecycle_pending_report, build_lifecycle_plan, build_upgrade_completion_report,
    build_upgrade_consent_evidence, build_upgrade_proposal_report,
    build_upgrade_verification_check, build_upgrade_verification_policy,
    build_upgrade_verification_report,
};
pub(super) use command::{
    check_command, check_usage, command, completion_command, completion_usage, consent_command,
    consent_usage, critical_fix_command, critical_fix_usage, handoff_command, handoff_usage,
    inspect_command, inspect_usage, pending_command, pending_usage, plan_command, plan_usage,
    proposals_command, proposals_usage, usage, verification_check_command,
    verification_check_usage, verification_policy_command, verification_policy_usage,
    verify_command, verify_usage,
};
pub(super) use options::{
    DeployExternalCriticalFixOptions, DeployExternalInspectOptions, DeployExternalOptions,
    DeployExternalVerifyOptions,
};

use super::{
    DeployCommandError, load_deployment_check, output_format::ExternalOutputFormat, print_json,
    read_json_file,
};
use crate::{
    cli::{clap::parse_subcommand, help::print_help_or_version},
    version_text,
};
use canic_host::deployment_truth::{
    DeploymentCheckV1, ExternalUpgradeCompletionReportRequest,
    ExternalUpgradeConsentEvidenceRequest, ExternalUpgradeVerificationCheckRequest,
    ExternalUpgradeVerificationPolicyRequest, ExternalUpgradeVerificationReportRequest,
    critical_external_fix_report_text, external_lifecycle_check_text,
    external_lifecycle_handoff_text, external_lifecycle_pending_report_text,
    external_lifecycle_plan_text, external_upgrade_completion_report_text,
    external_upgrade_consent_evidence_text, external_upgrade_proposal_report_text,
    external_upgrade_verification_check_text, external_upgrade_verification_policy_text,
    external_upgrade_verification_report_text,
};
use clap::Command as ClapCommand;
use std::ffi::OsString;

pub(super) fn run<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(command(), args).map_err(|_| DeployCommandError::Usage(usage()))? {
        Some((command, args)) if command == "plan" => run_plan(args),
        Some((command, args)) if command == "check" => run_check(args),
        Some((command, args)) if command == "handoff" => run_handoff(args),
        Some((command, args)) if command == "proposals" => run_proposals(args),
        Some((command, args)) if command == "pending" => run_pending(args),
        Some((command, args)) if command == "critical-fix" => run_critical_fix(args),
        Some((command, args)) if command == "inspect" => run_inspect(args),
        Some((command, args)) if command == "verify" => run_verify(args),
        _ => {
            println!("{}", usage());
            Ok(())
        }
    }
}

fn run_inspect<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, inspect_usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(inspect_command(), args)
        .map_err(|_| DeployCommandError::Usage(inspect_usage()))?
    {
        Some((command, args)) if command == "consent" => run_inspect_consent(args),
        Some((command, args)) if command == "verification-policy" => {
            run_inspect_verification_policy(args)
        }
        Some((command, args)) if command == "verification-check" => {
            run_inspect_verification_check(args)
        }
        Some((command, args)) if command == "completion" => run_inspect_completion(args),
        _ => {
            println!("{}", inspect_usage());
            Ok(())
        }
    }
}

fn run_plan<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        plan_command,
        plan_usage,
        build_lifecycle_plan,
        external_lifecycle_plan_text,
    )
}

fn run_check<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        check_command,
        check_usage,
        build_lifecycle_check,
        external_lifecycle_check_text,
    )
}

fn run_handoff<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        handoff_command,
        handoff_usage,
        build_lifecycle_handoff,
        external_lifecycle_handoff_text,
    )
}

fn run_proposals<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        proposals_command,
        proposals_usage,
        build_upgrade_proposal_report,
        external_upgrade_proposal_report_text,
    )
}

fn run_pending<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_output(
        args,
        pending_command,
        pending_usage,
        build_lifecycle_pending_report,
        external_lifecycle_pending_report_text,
    )
}

fn run_critical_fix<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, critical_fix_usage, version_text()) {
        return Ok(());
    }

    let options =
        DeployExternalCriticalFixOptions::parse(args, critical_fix_command, critical_fix_usage)?;
    let check = load_deployment_check(options.truth)?;
    let report =
        build_critical_fix_report(&check, options.fix_id.as_str(), options.severity.as_str());
    match options.format {
        ExternalOutputFormat::Json => print_json(&report)?,
        ExternalOutputFormat::Text => println!("{}", critical_external_fix_report_text(&report)),
    }
    Ok(())
}

fn run_inspect_consent<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, consent_usage, version_text()) {
        return Ok(());
    }

    let options = DeployExternalInspectOptions::parse(args, consent_command, consent_usage)?;
    let request = read_json_file::<ExternalUpgradeConsentEvidenceRequest>(&options.request)?;
    let evidence = build_upgrade_consent_evidence(request)?;
    match options.format {
        ExternalOutputFormat::Json => print_json(&evidence)?,
        ExternalOutputFormat::Text => {
            println!("{}", external_upgrade_consent_evidence_text(&evidence));
        }
    }
    Ok(())
}

fn run_inspect_verification_policy<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, verification_policy_usage, version_text()) {
        return Ok(());
    }

    let options = DeployExternalInspectOptions::parse(
        args,
        verification_policy_command,
        verification_policy_usage,
    )?;
    let request = read_json_file::<ExternalUpgradeVerificationPolicyRequest>(&options.request)?;
    let policy = build_upgrade_verification_policy(request);
    match options.format {
        ExternalOutputFormat::Json => print_json(&policy)?,
        ExternalOutputFormat::Text => {
            println!("{}", external_upgrade_verification_policy_text(&policy));
        }
    }
    Ok(())
}

fn run_inspect_verification_check<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, verification_check_usage, version_text()) {
        return Ok(());
    }

    let options = DeployExternalInspectOptions::parse(
        args,
        verification_check_command,
        verification_check_usage,
    )?;
    let request = read_json_file::<ExternalUpgradeVerificationCheckRequest>(&options.request)?;
    let check = build_upgrade_verification_check(request)?;
    match options.format {
        ExternalOutputFormat::Json => print_json(&check)?,
        ExternalOutputFormat::Text => {
            println!("{}", external_upgrade_verification_check_text(&check));
        }
    }
    Ok(())
}

fn run_inspect_completion<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, completion_usage, version_text()) {
        return Ok(());
    }

    let options = DeployExternalInspectOptions::parse(args, completion_command, completion_usage)?;
    let request = read_json_file::<ExternalUpgradeCompletionReportRequest>(&options.request)?;
    let report = build_upgrade_completion_report(request)?;
    match options.format {
        ExternalOutputFormat::Json => print_json(&report)?,
        ExternalOutputFormat::Text => {
            println!("{}", external_upgrade_completion_report_text(&report));
        }
    }
    Ok(())
}

fn run_verify<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, verify_usage, version_text()) {
        return Ok(());
    }

    let options = DeployExternalVerifyOptions::parse(args, verify_command, verify_usage)?;
    let request = read_json_file::<ExternalUpgradeVerificationReportRequest>(&options.request)?;
    let report = build_upgrade_verification_report(request)?;
    match options.format {
        ExternalOutputFormat::Json => print_json(&report)?,
        ExternalOutputFormat::Text => {
            println!("{}", external_upgrade_verification_report_text(&report));
        }
    }
    Ok(())
}

fn run_output<I, T>(
    args: I,
    command: impl FnOnce() -> ClapCommand,
    usage: fn() -> String,
    build: impl FnOnce(&DeploymentCheckV1) -> T,
    render_text: impl FnOnce(&T) -> String,
) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
    T: serde::Serialize,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = DeployExternalOptions::parse(args, command, usage)?;
    let check = load_deployment_check(options.truth)?;
    let output = build(&check);
    match options.format {
        ExternalOutputFormat::Json => print_json(&output)?,
        ExternalOutputFormat::Text => println!("{}", render_text(&output)),
    }
    Ok(())
}
