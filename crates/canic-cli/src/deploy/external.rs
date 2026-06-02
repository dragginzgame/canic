use super::{
    DeployCommandError, DeployTruthOptions, deploy_truth_leaf_command, load_deployment_check,
    output_format::{ExternalOutputFormat, parse_external_output_format},
    print_json, read_json_file, value_arg,
};
use crate::{
    cli::{
        clap::{
            parse_matches, parse_subcommand, passthrough_subcommand, path_option, string_option,
        },
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::deployment_truth::{
    CriticalExternalFixReportV1, DeploymentCheckV1, ExternalLifecycleCheckV1,
    ExternalLifecycleHandoffV1, ExternalLifecyclePendingReportV1, ExternalLifecyclePlanV1,
    ExternalUpgradeCompletionReportRequest, ExternalUpgradeCompletionReportV1,
    ExternalUpgradeConsentEvidenceRequest, ExternalUpgradeConsentEvidenceV1,
    ExternalUpgradeProposalReportV1, ExternalUpgradeVerificationCheckRequest,
    ExternalUpgradeVerificationCheckV1, ExternalUpgradeVerificationPolicyRequest,
    ExternalUpgradeVerificationPolicyV1, ExternalUpgradeVerificationReportRequest,
    ExternalUpgradeVerificationReportV1, critical_external_fix_report_from_pending,
    critical_external_fix_report_text, external_lifecycle_check_from_reports,
    external_lifecycle_check_text, external_lifecycle_handoff_from_reports,
    external_lifecycle_handoff_text, external_lifecycle_pending_report_from_plan,
    external_lifecycle_pending_report_text, external_lifecycle_plan_from_check,
    external_lifecycle_plan_text, external_upgrade_completion_report_from_evidence,
    external_upgrade_completion_report_text, external_upgrade_consent_evidence_from_receipt,
    external_upgrade_consent_evidence_text, external_upgrade_proposal_report_from_lifecycle_plan,
    external_upgrade_proposal_report_text, external_upgrade_verification_check_from_policy,
    external_upgrade_verification_check_text, external_upgrade_verification_observation_from_check,
    external_upgrade_verification_policy_from_proposal, external_upgrade_verification_policy_text,
    external_upgrade_verification_report_from_receipt, external_upgrade_verification_report_text,
    validate_external_upgrade_verification_check_for_deployment_check,
    validate_external_upgrade_verification_check_for_policy,
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

const DEPLOY_EXTERNAL_HELP_AFTER: &str = "\
Examples:
  canic deploy external plan demo
  canic deploy external check demo
  canic deploy external handoff demo
  canic deploy external proposals demo
  canic deploy external pending demo
  canic deploy external critical-fix --fix-id fix-2026-05 --severity critical demo
  canic deploy external inspect consent --request external-consent.json
  canic deploy external inspect verification-policy --request external-verification-policy.json
  canic deploy external inspect verification-check --request external-verification-check.json
  canic deploy external verify --request external-verification.json
  canic deploy external plan --format text demo
  canic deploy external verify --request external-verification.json --format text
  canic --network local deploy external critical-fix --fix-id fix-2026-05 --severity high --profile fast demo

0.45 external lifecycle commands are passive reports. They do not request
consent, execute external upgrades, install code, or mutate deployment state.";
const DEPLOY_EXTERNAL_INSPECT_HELP_AFTER: &str = "\
Examples:
  canic deploy external inspect consent --request external-consent.json
  canic deploy external inspect consent --request external-consent.json --format text
  canic deploy external inspect verification-policy --request external-verification-policy.json
  canic deploy external inspect verification-policy --request external-verification-policy.json --format text
  canic deploy external inspect verification-check --request external-verification-check.json
  canic deploy external inspect verification-check --request external-verification-check.json --format text
  canic deploy external inspect completion --request external-completion.json
  canic deploy external inspect completion --request external-completion.json --format text

Advanced external lifecycle inspection commands expose archived/passive DTOs.
They do not request consent, execute external upgrades, install code, or mutate
deployment state.";
const DEPLOY_EXTERNAL_CONSENT_HELP_AFTER: &str = "\
Examples:
  canic deploy external inspect consent --request external-consent.json
  canic deploy external inspect consent --request external-consent.json --format text

Reads an ExternalUpgradeConsentEvidenceRequest-shaped JSON file and prints
ExternalUpgradeConsentEvidenceV1 JSON by default, or host-owned passive text
with --format text. Consent evidence records reported consent/action state; it
does not verify live completion.";
const DEPLOY_EXTERNAL_VERIFICATION_POLICY_HELP_AFTER: &str = "\
Examples:
  canic deploy external inspect verification-policy --request external-verification-policy.json
  canic deploy external inspect verification-policy --request external-verification-policy.json --format text

Reads an ExternalUpgradeVerificationPolicyRequest-shaped JSON file and prints
ExternalUpgradeVerificationPolicyV1 JSON by default, or host-owned passive text
with --format text. Verification policies describe required live-inventory
postconditions; they do not query live inventory or verify completion.";
const DEPLOY_EXTERNAL_VERIFICATION_CHECK_HELP_AFTER: &str = "\
Examples:
  canic deploy external inspect verification-check --request external-verification-check.json
  canic deploy external inspect verification-check --request external-verification-check.json --format text

Reads an ExternalUpgradeVerificationCheckRequest-shaped JSON file and prints
ExternalUpgradeVerificationCheckV1 JSON by default, or host-owned passive text
with --format text. Verification checks evaluate supplied observation facts or
an embedded DeploymentCheckV1 inventory artifact against a verification policy;
they do not query live inventory or execute external lifecycle work.";
const DEPLOY_EXTERNAL_COMPLETION_HELP_AFTER: &str = "\
Examples:
  canic deploy external inspect completion --request external-completion.json
  canic deploy external inspect completion --request external-completion.json --format text

Reads an ExternalUpgradeCompletionReportRequest-shaped JSON file and prints
ExternalUpgradeCompletionReportV1 JSON by default, or host-owned passive text
with --format text. Completion reports combine proposal, consent evidence, and
verification-check evidence; only deployment-truth inventory verification can
mark external lifecycle work verified complete.";
const DEPLOY_EXTERNAL_PLAN_HELP_AFTER: &str = "\
Examples:
  canic deploy external plan demo
  canic deploy external plan --format text demo
  canic --network local deploy external plan --profile fast demo

Prints ExternalLifecyclePlanV1 JSON by default, or host-owned passive text with
--format text. No consent delivery, external execution, install, or mutation is
attempted.";
const DEPLOY_EXTERNAL_CHECK_HELP_AFTER: &str = "\
Examples:
  canic deploy external check demo
  canic deploy external check --format text demo
  canic --network local deploy external check --profile fast demo

Prints ExternalLifecycleCheckV1 JSON by default, or host-owned passive text
with --format text. External lifecycle checks summarize direct, pending,
blocked, and residual-exposure status without requesting consent, executing
external upgrades, or mutating state.";
const DEPLOY_EXTERNAL_HANDOFF_HELP_AFTER: &str = "\
Examples:
  canic deploy external handoff demo
  canic deploy external handoff --format text demo
  canic --network local deploy external handoff --profile fast demo

Prints ExternalLifecycleHandoffV1 JSON by default, or host-owned passive text
with --format text. Handoff packets package pending external proposals into
operator coordination instructions; they do not deliver consent, execute
external upgrades, or mutate state.";
const DEPLOY_EXTERNAL_PROPOSALS_HELP_AFTER: &str = "\
Examples:
  canic deploy external proposals demo
  canic deploy external proposals --format text demo
  canic --network local deploy external proposals --profile fast demo

Prints ExternalUpgradeProposalReportV1 JSON by default, or host-owned passive
text with --format text. Proposals are derived from the local lifecycle plan
and do not grant consent or execute upgrades.";
const DEPLOY_EXTERNAL_PENDING_HELP_AFTER: &str = "\
Examples:
  canic deploy external pending demo
  canic deploy external pending --format text demo
  canic --network local deploy external pending --profile fast demo

Prints ExternalLifecyclePendingReportV1 JSON by default, or host-owned passive
text with --format text. Pending reports summarize unresolved external actions,
blocked subjects, and residual exposure without requesting consent or executing
upgrades.";
const DEPLOY_EXTERNAL_CRITICAL_FIX_HELP_AFTER: &str = "\
Examples:
  canic deploy external critical-fix --fix-id fix-2026-05 --severity critical demo
  canic deploy external critical-fix --fix-id fix-2026-05 --severity critical --format text demo
  canic --network local deploy external critical-fix --fix-id fix-2026-05 --severity high --profile fast demo

Prints CriticalExternalFixReportV1 JSON by default, or host-owned passive text
with --format text. Critical-fix reports summarize directly patchable roles,
external blockers, required external actions, protected-call implications, and
residual exposure without claiming deployment completion or mutating state.";
const DEPLOY_EXTERNAL_VERIFY_HELP_AFTER: &str = "\
Examples:
  canic deploy external verify --request external-verification.json
  canic deploy external verify --request external-verification.json --format text

Reads an ExternalUpgradeVerificationReportRequest-shaped JSON file and prints
ExternalUpgradeVerificationReportV1 JSON by default, or host-owned passive text
with --format text. Verification reports package proposal/receipt structural
evidence only; live inventory remains the source of truth for deployment
state.";

///
/// DeployExternalOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeployExternalOptions {
    pub(super) truth: DeployTruthOptions,
    pub(super) format: ExternalOutputFormat,
}

///
/// DeployExternalCriticalFixOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeployExternalCriticalFixOptions {
    pub(super) truth: DeployTruthOptions,
    pub(super) format: ExternalOutputFormat,
    pub(super) fix_id: String,
    pub(super) severity: String,
}

///
/// DeployExternalVerifyOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeployExternalVerifyOptions {
    pub(super) request: PathBuf,
    pub(super) format: ExternalOutputFormat,
}

///
/// DeployExternalInspectOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeployExternalInspectOptions {
    pub(super) request: PathBuf,
    pub(super) format: ExternalOutputFormat,
}

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

pub(super) fn build_lifecycle_plan(check: &DeploymentCheckV1) -> ExternalLifecyclePlanV1 {
    external_lifecycle_plan_from_check(
        local_lifecycle_plan_id(check),
        local_lifecycle_authority_report_id(check),
        check,
    )
}

pub(super) fn build_upgrade_proposal_report(
    check: &DeploymentCheckV1,
) -> ExternalUpgradeProposalReportV1 {
    let lifecycle_plan = build_lifecycle_plan(check);
    external_upgrade_proposal_report_from_lifecycle_plan(
        local_proposal_report_id(check),
        &lifecycle_plan,
        check,
    )
}

pub(super) fn build_lifecycle_pending_report(
    check: &DeploymentCheckV1,
) -> ExternalLifecyclePendingReportV1 {
    let lifecycle_plan = build_lifecycle_plan(check);
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        local_proposal_report_id(check),
        &lifecycle_plan,
        check,
    );
    external_lifecycle_pending_report_from_plan(
        local_pending_report_id(check),
        &lifecycle_plan,
        &proposal_report,
    )
}

pub(super) fn build_lifecycle_check(check: &DeploymentCheckV1) -> ExternalLifecycleCheckV1 {
    let lifecycle_plan = build_lifecycle_plan(check);
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        local_proposal_report_id(check),
        &lifecycle_plan,
        check,
    );
    let pending_report = external_lifecycle_pending_report_from_plan(
        local_pending_report_id(check),
        &lifecycle_plan,
        &proposal_report,
    );
    external_lifecycle_check_from_reports(
        local_check_id(check),
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    )
}

pub(super) fn build_lifecycle_handoff(check: &DeploymentCheckV1) -> ExternalLifecycleHandoffV1 {
    let lifecycle_plan = build_lifecycle_plan(check);
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        local_proposal_report_id(check),
        &lifecycle_plan,
        check,
    );
    let pending_report = external_lifecycle_pending_report_from_plan(
        local_pending_report_id(check),
        &lifecycle_plan,
        &proposal_report,
    );
    let lifecycle_check = external_lifecycle_check_from_reports(
        local_check_id(check),
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    );
    external_lifecycle_handoff_from_reports(
        local_handoff_id(check),
        &lifecycle_check,
        &proposal_report,
        &pending_report,
    )
}

pub(super) fn build_critical_fix_report(
    check: &DeploymentCheckV1,
    fix_id: &str,
    severity: &str,
) -> CriticalExternalFixReportV1 {
    let lifecycle_plan = build_lifecycle_plan(check);
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        local_proposal_report_id(check),
        &lifecycle_plan,
        check,
    );
    let pending_report = external_lifecycle_pending_report_from_plan(
        local_pending_report_id(check),
        &lifecycle_plan,
        &proposal_report,
    );
    critical_external_fix_report_from_pending(
        local_critical_fix_report_id(check),
        fix_id,
        severity,
        &lifecycle_plan,
        &pending_report,
    )
}

pub(super) fn build_upgrade_consent_evidence(
    request: ExternalUpgradeConsentEvidenceRequest,
) -> Result<ExternalUpgradeConsentEvidenceV1, DeployCommandError> {
    external_upgrade_consent_evidence_from_receipt(
        request.evidence_id,
        &request.proposal,
        &request.receipt,
    )
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

pub(super) fn build_upgrade_verification_policy(
    request: ExternalUpgradeVerificationPolicyRequest,
) -> ExternalUpgradeVerificationPolicyV1 {
    external_upgrade_verification_policy_from_proposal(request.policy_id, &request.proposal)
}

pub(super) fn build_upgrade_verification_check(
    request: ExternalUpgradeVerificationCheckRequest,
) -> Result<ExternalUpgradeVerificationCheckV1, DeployCommandError> {
    let observation = match (request.observation, request.deployment_check) {
        (Some(observation), None) => observation,
        (None, Some(deployment_check)) => {
            let observation = external_upgrade_verification_observation_from_check(
                &request.policy,
                &deployment_check,
            )
            .map_err(|err| DeployCommandError::Check(Box::new(err)))?;
            let check = external_upgrade_verification_check_from_policy(
                request.check_id,
                &request.policy,
                observation,
            );
            validate_external_upgrade_verification_check_for_deployment_check(
                &check,
                &request.policy,
                &deployment_check,
            )
            .map_err(|err| DeployCommandError::Check(Box::new(err)))?;
            return Ok(check);
        }
        (Some(_), Some(_)) => {
            return Err(DeployCommandError::Blocked(
                "external verification check request must provide either observation or deployment_check, not both"
                    .to_string(),
            ));
        }
        (None, None) => {
            return Err(DeployCommandError::Blocked(
                "external verification check request must provide observation or deployment_check"
                    .to_string(),
            ));
        }
    };
    let check = external_upgrade_verification_check_from_policy(
        request.check_id,
        &request.policy,
        observation,
    );
    validate_external_upgrade_verification_check_for_policy(&check, &request.policy)
        .map_err(|err| DeployCommandError::Check(Box::new(err)))?;
    Ok(check)
}

pub(super) fn build_upgrade_completion_report(
    request: ExternalUpgradeCompletionReportRequest,
) -> Result<ExternalUpgradeCompletionReportV1, DeployCommandError> {
    external_upgrade_completion_report_from_evidence(
        request.report_id,
        &request.proposal,
        &request.consent_evidence,
        &request.verification_check,
    )
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

pub(super) fn build_upgrade_verification_report(
    request: ExternalUpgradeVerificationReportRequest,
) -> Result<ExternalUpgradeVerificationReportV1, DeployCommandError> {
    external_upgrade_verification_report_from_receipt(
        request.report_id,
        &request.proposal,
        &request.receipt,
    )
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

fn local_lifecycle_plan_id(check: &DeploymentCheckV1) -> String {
    local_artifact_id(check, "external-lifecycle-plan")
}

fn local_check_id(check: &DeploymentCheckV1) -> String {
    local_artifact_id(check, "external-lifecycle-check")
}

fn local_handoff_id(check: &DeploymentCheckV1) -> String {
    local_artifact_id(check, "external-lifecycle-handoff")
}

fn local_lifecycle_authority_report_id(check: &DeploymentCheckV1) -> String {
    local_artifact_id(check, "lifecycle-authority-report")
}

fn local_proposal_report_id(check: &DeploymentCheckV1) -> String {
    local_artifact_id(check, "external-upgrade-proposals")
}

fn local_pending_report_id(check: &DeploymentCheckV1) -> String {
    local_artifact_id(check, "external-lifecycle-pending")
}

fn local_critical_fix_report_id(check: &DeploymentCheckV1) -> String {
    local_artifact_id(check, "critical-external-fix")
}

fn local_artifact_id(check: &DeploymentCheckV1, suffix: &str) -> String {
    format!(
        "local:{}:{}:{suffix}",
        check.plan.runtime_variant, check.plan.deployment_identity.deployment_name
    )
}

impl DeployExternalOptions {
    pub(super) fn parse<I>(
        args: I,
        command: impl FnOnce() -> ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        Ok(Self {
            truth: DeployTruthOptions::from_matches(&matches, usage)?,
            format: parse_external_output_format(
                string_option(&matches, "format").as_deref(),
                usage,
            )?,
        })
    }
}

impl DeployExternalCriticalFixOptions {
    pub(super) fn parse<I>(
        args: I,
        command: impl FnOnce() -> ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        Ok(Self {
            truth: DeployTruthOptions::from_matches(&matches, usage)?,
            format: parse_external_output_format(
                string_option(&matches, "format").as_deref(),
                usage,
            )?,
            fix_id: string_option(&matches, "fix-id").expect("clap requires fix-id"),
            severity: string_option(&matches, "severity").expect("clap requires severity"),
        })
    }
}

impl DeployExternalVerifyOptions {
    pub(super) fn parse<I>(
        args: I,
        command: impl FnOnce() -> ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        Ok(Self {
            request: path_option(&matches, "request").expect("clap requires request"),
            format: parse_external_output_format(
                string_option(&matches, "format").as_deref(),
                usage,
            )?,
        })
    }
}

impl DeployExternalInspectOptions {
    pub(super) fn parse<I>(
        args: I,
        command: impl FnOnce() -> ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        Ok(Self {
            request: path_option(&matches, "request").expect("clap requires request"),
            format: parse_external_output_format(
                string_option(&matches, "format").as_deref(),
                usage,
            )?,
        })
    }
}

pub(super) fn command() -> ClapCommand {
    ClapCommand::new("external")
        .bin_name("canic deploy external")
        .about("Build passive external lifecycle reports")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("plan")
                .about("Build a passive external lifecycle plan")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("check")
                .about("Build a passive external lifecycle check")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("handoff")
                .about("Build a passive external lifecycle handoff packet")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("proposals")
                .about("Build passive external upgrade proposals")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("pending")
                .about("Build a passive external lifecycle pending report")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("critical-fix")
                .about("Build a passive critical external fix report")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("inspect")
                .about("Inspect passive external lifecycle internals")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("verify")
                .about("Build a passive external upgrade verification report")
                .disable_help_flag(true),
        ))
        .after_help(DEPLOY_EXTERNAL_HELP_AFTER)
}

pub(super) fn inspect_command() -> ClapCommand {
    ClapCommand::new("inspect")
        .bin_name("canic deploy external inspect")
        .about("Inspect passive external lifecycle internals")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("consent")
                .about("Build passive external consent evidence")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("verification-policy")
                .about("Build passive external verification policy")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("verification-check")
                .about("Build passive external verification check")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("completion")
                .about("Build passive external completion report")
                .disable_help_flag(true),
        ))
        .after_help(DEPLOY_EXTERNAL_INSPECT_HELP_AFTER)
}

pub(super) fn plan_command() -> ClapCommand {
    deploy_truth_leaf_command("plan", "Print the local external lifecycle plan")
        .arg(format_arg())
        .bin_name("canic deploy external plan")
        .after_help(DEPLOY_EXTERNAL_PLAN_HELP_AFTER)
}

pub(super) fn check_command() -> ClapCommand {
    deploy_truth_leaf_command("check", "Print the local external lifecycle check")
        .arg(format_arg())
        .bin_name("canic deploy external check")
        .after_help(DEPLOY_EXTERNAL_CHECK_HELP_AFTER)
}

pub(super) fn handoff_command() -> ClapCommand {
    deploy_truth_leaf_command("handoff", "Print the local external lifecycle handoff")
        .arg(format_arg())
        .bin_name("canic deploy external handoff")
        .after_help(DEPLOY_EXTERNAL_HANDOFF_HELP_AFTER)
}

pub(super) fn proposals_command() -> ClapCommand {
    deploy_truth_leaf_command("proposals", "Print local external upgrade proposals")
        .arg(format_arg())
        .bin_name("canic deploy external proposals")
        .after_help(DEPLOY_EXTERNAL_PROPOSALS_HELP_AFTER)
}

pub(super) fn pending_command() -> ClapCommand {
    deploy_truth_leaf_command(
        "pending",
        "Print the local external lifecycle pending report",
    )
    .arg(format_arg())
    .bin_name("canic deploy external pending")
    .after_help(DEPLOY_EXTERNAL_PENDING_HELP_AFTER)
}

pub(super) fn critical_fix_command() -> ClapCommand {
    deploy_truth_leaf_command(
        "critical-fix",
        "Print the local critical external fix report",
    )
    .arg(format_arg())
    .arg(
        value_arg("fix-id")
            .long("fix-id")
            .value_name("id")
            .required(true)
            .help("Critical fix identifier to record in the report"),
    )
    .arg(
        value_arg("severity")
            .long("severity")
            .value_name("severity")
            .required(true)
            .help("Critical fix severity label to record in the report"),
    )
    .bin_name("canic deploy external critical-fix")
    .after_help(DEPLOY_EXTERNAL_CRITICAL_FIX_HELP_AFTER)
}

pub(super) fn verify_command() -> ClapCommand {
    ClapCommand::new("verify")
        .bin_name("canic deploy external verify")
        .about("Build a passive external upgrade verification report")
        .disable_help_flag(true)
        .override_usage("canic deploy external verify --request <file>")
        .arg(
            value_arg("request")
                .long("request")
                .value_name("file")
                .required(true)
                .help("ExternalUpgradeVerificationReportRequest JSON file to verify"),
        )
        .arg(format_arg())
        .after_help(DEPLOY_EXTERNAL_VERIFY_HELP_AFTER)
}

pub(super) fn consent_command() -> ClapCommand {
    ClapCommand::new("consent")
        .bin_name("canic deploy external inspect consent")
        .about("Build passive external consent evidence")
        .disable_help_flag(true)
        .override_usage("canic deploy external inspect consent --request <file>")
        .arg(
            value_arg("request")
                .long("request")
                .value_name("file")
                .required(true)
                .help("ExternalUpgradeConsentEvidenceRequest JSON file to inspect"),
        )
        .arg(format_arg())
        .after_help(DEPLOY_EXTERNAL_CONSENT_HELP_AFTER)
}

pub(super) fn verification_policy_command() -> ClapCommand {
    ClapCommand::new("verification-policy")
        .bin_name("canic deploy external inspect verification-policy")
        .about("Build passive external verification policy")
        .disable_help_flag(true)
        .override_usage("canic deploy external inspect verification-policy --request <file>")
        .arg(
            value_arg("request")
                .long("request")
                .value_name("file")
                .required(true)
                .help("ExternalUpgradeVerificationPolicyRequest JSON file to inspect"),
        )
        .arg(format_arg())
        .after_help(DEPLOY_EXTERNAL_VERIFICATION_POLICY_HELP_AFTER)
}

pub(super) fn verification_check_command() -> ClapCommand {
    ClapCommand::new("verification-check")
        .bin_name("canic deploy external inspect verification-check")
        .about("Build passive external verification check")
        .disable_help_flag(true)
        .override_usage("canic deploy external inspect verification-check --request <file>")
        .arg(
            value_arg("request")
                .long("request")
                .value_name("file")
                .required(true)
                .help("ExternalUpgradeVerificationCheckRequest JSON file to inspect"),
        )
        .arg(format_arg())
        .after_help(DEPLOY_EXTERNAL_VERIFICATION_CHECK_HELP_AFTER)
}

pub(super) fn completion_command() -> ClapCommand {
    ClapCommand::new("completion")
        .bin_name("canic deploy external inspect completion")
        .about("Build passive external completion report")
        .disable_help_flag(true)
        .override_usage("canic deploy external inspect completion --request <file>")
        .arg(
            value_arg("request")
                .long("request")
                .value_name("file")
                .required(true)
                .help("ExternalUpgradeCompletionReportRequest JSON file to inspect"),
        )
        .arg(format_arg())
        .after_help(DEPLOY_EXTERNAL_COMPLETION_HELP_AFTER)
}

fn format_arg() -> clap::Arg {
    value_arg("format")
        .long("format")
        .value_name("json|text")
        .num_args(1)
        .help("Output format; defaults to json")
}

pub(super) fn usage() -> String {
    let mut command = command();
    command.render_help().to_string()
}

pub(super) fn plan_usage() -> String {
    let mut command = plan_command();
    command.render_help().to_string()
}

pub(super) fn check_usage() -> String {
    let mut command = check_command();
    command.render_help().to_string()
}

pub(super) fn handoff_usage() -> String {
    let mut command = handoff_command();
    command.render_help().to_string()
}

pub(super) fn proposals_usage() -> String {
    let mut command = proposals_command();
    command.render_help().to_string()
}

pub(super) fn pending_usage() -> String {
    let mut command = pending_command();
    command.render_help().to_string()
}

pub(super) fn critical_fix_usage() -> String {
    let mut command = critical_fix_command();
    command.render_help().to_string()
}

pub(super) fn inspect_usage() -> String {
    let mut command = inspect_command();
    command.render_help().to_string()
}

pub(super) fn consent_usage() -> String {
    let mut command = consent_command();
    command.render_help().to_string()
}

pub(super) fn verification_policy_usage() -> String {
    let mut command = verification_policy_command();
    command.render_help().to_string()
}

pub(super) fn verification_check_usage() -> String {
    let mut command = verification_check_command();
    command.render_help().to_string()
}

pub(super) fn completion_usage() -> String {
    let mut command = completion_command();
    command.render_help().to_string()
}

pub(super) fn verify_usage() -> String {
    let mut command = verify_command();
    command.render_help().to_string()
}
