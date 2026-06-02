mod authority;
mod catalog;
mod check;
mod compare;
mod external;
mod install;
mod output_format;
mod register;
mod resume_report;
mod root;
mod truth;

use crate::{
    cli::{
        clap::{
            parse_matches, parse_subcommand, passthrough_subcommand, path_option, string_option,
            value_arg,
        },
        defaults::local_network,
        globals::internal_network_arg,
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::{
    canister_build::CanisterBuildProfile,
    deployment_truth::{
        ArtifactPromotionExecutionReceiptRequest, ArtifactPromotionExecutionReceiptV1,
        ArtifactPromotionPlanRequest, ArtifactPromotionPlanV1,
        ArtifactPromotionProvenanceReportRequest, ArtifactPromotionProvenanceReportV1,
        BuildMaterializationEvidenceV1, DeploymentCheckV1, DeploymentExecutionPreflightV1,
        DeploymentPlanV1, DeploymentReceiptV1, PromotionArtifactIdentityReportRequest,
        PromotionArtifactIdentityReportV1, PromotionMaterializationIdentityReportRequest,
        PromotionMaterializationIdentityReportV1, PromotionPlanTransformEvidenceRequest,
        PromotionPlanTransformEvidenceV1, PromotionPlanTransformRequest, PromotionPlanTransformV1,
        PromotionPlanTransformWithMaterializationRequest, PromotionPolicyCheckRequest,
        PromotionPolicyCheckV1, PromotionReadinessRequest, PromotionReadinessV1,
        PromotionTargetExecutionLineageRequest, PromotionTargetExecutionLineageV1,
        PromotionWasmStoreCatalogEntryV1, PromotionWasmStoreCatalogVerificationRequest,
        PromotionWasmStoreCatalogVerificationV1, PromotionWasmStoreIdentityReportRequest,
        PromotionWasmStoreIdentityReportV1, RolePromotionInputV1, RolePromotionPolicyV1,
        StagingReceiptV1, artifact_promotion_execution_receipt,
        artifact_promotion_execution_receipt_text, artifact_promotion_plan,
        artifact_promotion_plan_text, artifact_promotion_provenance_report,
        artifact_promotion_provenance_report_text, check_promotion_policy,
        check_promotion_readiness, promoted_deployment_plan_transform_from_inputs,
        promoted_deployment_plan_transform_from_inputs_with_materialization,
        promotion_artifact_identity_report_from_inputs, promotion_artifact_identity_report_text,
        promotion_materialization_identity_report_from_evidence,
        promotion_materialization_identity_report_text, promotion_plan_transform_evidence,
        promotion_plan_transform_evidence_text, promotion_plan_transform_text,
        promotion_policy_check_text, promotion_readiness_text, promotion_target_execution_lineage,
        promotion_target_execution_lineage_text, promotion_wasm_store_catalog_verification,
        promotion_wasm_store_catalog_verification_text,
        promotion_wasm_store_identity_report_from_staging,
        promotion_wasm_store_identity_report_text,
    },
    icp_config::resolve_current_canic_icp_root,
    install_root::{InstallRootOptions, check_install_deployment_truth},
};
use clap::Command as ClapCommand;
use output_format::{PromotionOutputFormat, parse_promotion_output_format};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::{
    ffi::OsString,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

const DEFAULT_ROOT_TARGET: &str = "root";
const DEFAULT_READY_TIMEOUT_SECONDS: u64 = 120;
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
const DEPLOY_PROMOTE_HELP_AFTER: &str = "\
Examples:
  canic deploy promote plan --request promotion-plan.json
  canic deploy promote check --request promotion-check.json
  canic deploy promote diff --request promotion-diff.json
  canic deploy promote inspect readiness --request promotion-readiness.json
  canic deploy promote inspect artifact-identity --request promotion-artifacts.json
  canic deploy promote inspect provenance --request promotion-provenance.json
  canic deploy promote inspect readiness --request promotion-readiness.json --format text

0.44 promotion commands are passive report builders. They do not install,
stage artifacts, query wasm_store, or mutate deployment/controller state.";
const DEPLOY_PROMOTE_INSPECT_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect readiness --request promotion-readiness.json
  canic deploy promote inspect artifact-identity --request promotion-artifacts.json
  canic deploy promote inspect transform --request promotion-transform.json
  canic deploy promote inspect transform-evidence --request transform-evidence.json
  canic deploy promote inspect target-lineage --request target-lineage.json
  canic deploy promote inspect provenance --request promotion-provenance.json
  canic deploy promote inspect wasm-store-identity --request wasm-store-identity.json
  canic deploy promote inspect catalog-verification --request catalog-verification.json
  canic deploy promote inspect materialization-identity --request materialization.json
  canic deploy promote inspect policy --request promotion-policy.json
  canic deploy promote inspect execution-receipt --request promotion-execution-receipt.json

Advanced promotion inspection commands expose archived/passive artifact DTOs.
They do not install, stage artifacts, query wasm_store, or mutate deployment or
controller state.";
const DEPLOY_PROMOTE_READINESS_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect readiness --request promotion-readiness.json
  canic deploy promote inspect readiness --request promotion-readiness.json --format text

Reads a PromotionReadinessRequest-shaped JSON file and prints
PromotionReadinessV1 JSON by default, or passive text with --format text.";
const DEPLOY_PROMOTE_CHECK_HELP_AFTER: &str = "\
Examples:
  canic deploy promote check --request promotion-check.json
  canic deploy promote check --request promotion-check.json --format text

Reads a PromotionReadinessRequest-shaped JSON file and prints a passive
PromotionReadinessV1 check report by default, or passive text with
--format text.";
const DEPLOY_PROMOTE_ARTIFACT_IDENTITY_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect artifact-identity --request promotion-artifacts.json
  canic deploy promote inspect artifact-identity --request promotion-artifacts.json --format text

Reads a PromotionArtifactIdentityReportRequest-shaped JSON file and prints
PromotionArtifactIdentityReportV1 JSON by default, or passive text with
--format text.";
const DEPLOY_PROMOTE_TRANSFORM_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect transform --request promotion-transform.json
  canic deploy promote inspect transform --request promotion-transform.json --format text

Reads a PromotionPlanTransformRequest-shaped JSON file and prints
PromotionPlanTransformV1 JSON by default, or passive text with --format text.";
const DEPLOY_PROMOTE_DIFF_HELP_AFTER: &str = "\
Examples:
  canic deploy promote diff --request promotion-diff.json
  canic deploy promote diff --request promotion-diff.json --format text

Reads a PromotionPlanTransformRequest-shaped JSON file and prints a passive
PromotionPlanTransformV1 diff report by default, or passive text with
--format text.";
const DEPLOY_PROMOTE_TRANSFORM_EVIDENCE_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect transform-evidence --request transform-evidence.json
  canic deploy promote inspect transform-evidence --request transform-evidence.json --format text

Reads a PromotionPlanTransformEvidenceRequest-shaped JSON file and prints
PromotionPlanTransformEvidenceV1 JSON by default, or passive text with
--format text.";
const DEPLOY_PROMOTE_TARGET_LINEAGE_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect target-lineage --request target-lineage.json
  canic deploy promote inspect target-lineage --request target-lineage.json --format text

Reads a PromotionTargetExecutionLineageRequest-shaped JSON file and prints
PromotionTargetExecutionLineageV1 JSON by default, or passive text with
--format text.";
const DEPLOY_PROMOTE_PLAN_HELP_AFTER: &str = "\
Examples:
  canic deploy promote plan --request promotion-plan.json
  canic deploy promote plan --request promotion-plan.json --format text

Reads an ArtifactPromotionPlanRequest-shaped JSON file and prints
ArtifactPromotionPlanV1 JSON by default, or passive text with --format text.";
const DEPLOY_PROMOTE_PROVENANCE_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect provenance --request promotion-provenance.json
  canic deploy promote inspect provenance --request promotion-provenance.json --format text

Reads an ArtifactPromotionProvenanceReportRequest-shaped JSON file and prints
ArtifactPromotionProvenanceReportV1 JSON by default, or passive text with
--format text.";
const DEPLOY_PROMOTE_WASM_STORE_IDENTITY_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect wasm-store-identity --request wasm-store-identity.json
  canic deploy promote inspect wasm-store-identity --request wasm-store-identity.json --format text

Reads a PromotionWasmStoreIdentityReportRequest-shaped JSON file and prints
PromotionWasmStoreIdentityReportV1 JSON by default, or passive text with
--format text.";
const DEPLOY_PROMOTE_CATALOG_VERIFICATION_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect catalog-verification --request catalog-verification.json
  canic deploy promote inspect catalog-verification --request catalog-verification.json --format text

Reads a PromotionWasmStoreCatalogVerificationRequest-shaped JSON file and
prints PromotionWasmStoreCatalogVerificationV1 JSON by default, or passive
text with --format text.";
const DEPLOY_PROMOTE_EXECUTION_RECEIPT_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect execution-receipt --request promotion-execution-receipt.json
  canic deploy promote inspect execution-receipt --request promotion-execution-receipt.json --format text

Reads an ArtifactPromotionExecutionReceiptRequest-shaped JSON file and prints
ArtifactPromotionExecutionReceiptV1 JSON by default, or passive text with
--format text.";
const DEPLOY_PROMOTE_POLICY_CHECK_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect policy --request promotion-policy.json
  canic deploy promote inspect policy --request promotion-policy.json --format text

Reads a PromotionPolicyCheckRequest-shaped JSON file and prints
PromotionPolicyCheckV1 JSON by default, or passive text with --format text.";
const DEPLOY_PROMOTE_MATERIALIZATION_IDENTITY_HELP_AFTER: &str = "\
Examples:
  canic deploy promote inspect materialization-identity --request materialization.json
  canic deploy promote inspect materialization-identity --request materialization.json --format text

Reads a PromotionMaterializationIdentityReportRequest-shaped JSON file and
prints PromotionMaterializationIdentityReportV1 JSON by default, or passive
text with --format text.";
///
/// DeployCommandError
///
#[derive(Debug, ThisError)]
pub enum DeployCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Check(#[from] Box<dyn std::error::Error>),

    #[error("deployment truth check blocked: {0}")]
    Blocked(String),
}

///
/// DeployTruthOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeployTruthOptions {
    pub(super) deployment: String,
    pub(super) network: String,
    pub(super) profile: Option<CanisterBuildProfile>,
}

///
/// DeployPromoteReportOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeployPromoteReportOptions {
    request: PathBuf,
    format: PromotionOutputFormat,
}

#[derive(Deserialize)]
struct PromotionReadinessFile {
    readiness_id: String,
    target_plan: DeploymentPlanV1,
    inputs: Vec<RolePromotionInputV1>,
}

#[derive(Deserialize)]
struct PromotionArtifactIdentityFile {
    report_id: String,
    inputs: Vec<RolePromotionInputV1>,
}

#[derive(Deserialize)]
struct PromotionPlanTransformFile {
    promoted_plan_id: String,
    target_plan: DeploymentPlanV1,
    inputs: Vec<RolePromotionInputV1>,
    materialization_evidence: Option<Vec<BuildMaterializationEvidenceV1>>,
}

#[derive(Deserialize)]
struct PromotionPlanTransformEvidenceFile {
    evidence_id: String,
    generated_at: String,
    transform: PromotionPlanTransformV1,
}

#[derive(Deserialize)]
struct PromotionTargetExecutionLineageFile {
    lineage_id: String,
    generated_at: String,
    transform: PromotionPlanTransformV1,
    execution_preflight: DeploymentExecutionPreflightV1,
}

#[derive(Deserialize)]
struct ArtifactPromotionPlanFile {
    plan_id: String,
    generated_at: String,
    readiness: PromotionReadinessV1,
    artifact_identity_report: PromotionArtifactIdentityReportV1,
    transform: PromotionPlanTransformV1,
    target_execution_lineage: Option<PromotionTargetExecutionLineageV1>,
}

#[derive(Deserialize)]
struct ArtifactPromotionProvenanceFile {
    report_id: String,
    artifact_promotion_plan: ArtifactPromotionPlanV1,
    wasm_store_identity_report: Option<PromotionWasmStoreIdentityReportV1>,
    wasm_store_catalog_verification: Option<PromotionWasmStoreCatalogVerificationV1>,
    materialization_identity_report: Option<PromotionMaterializationIdentityReportV1>,
}

#[derive(Deserialize)]
struct PromotionWasmStoreIdentityFile {
    report_id: String,
    staging_receipts: Vec<StagingReceiptV1>,
}

#[derive(Deserialize)]
struct PromotionWasmStoreCatalogVerificationFile {
    verification_id: String,
    wasm_store_identity_report: PromotionWasmStoreIdentityReportV1,
    catalog_entries: Vec<PromotionWasmStoreCatalogEntryV1>,
}

#[derive(Deserialize)]
struct ArtifactPromotionExecutionReceiptFile {
    receipt_id: String,
    provenance_report: ArtifactPromotionProvenanceReportV1,
    deployment_receipt: DeploymentReceiptV1,
}

#[derive(Deserialize)]
struct PromotionPolicyCheckFile {
    check_id: String,
    inputs: Vec<RolePromotionInputV1>,
    policies: Vec<RolePromotionPolicyV1>,
}

#[derive(Deserialize)]
struct PromotionMaterializationIdentityFile {
    report_id: String,
    evidence: Vec<BuildMaterializationEvidenceV1>,
}

pub fn run<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(deploy_command(), args)
        .map_err(|_| DeployCommandError::Usage(usage()))?
    {
        None => {
            println!("{}", usage());
            Ok(())
        }
        Some((command, args)) => match command.as_str() {
            "authority" => authority::run(args),
            "catalog" => catalog::run(args),
            "external" => external::run(args),
            "promote" => run_promote(args),
            "root" => root::run(args),
            "install" => install::run(args),
            "register" => register::run(args),
            "compare" => compare::run(args),
            "plan" => truth::run_plan(args),
            "inventory" => truth::run_inventory(args),
            "diff" => truth::run_diff(args),
            "report" => truth::run_report(args),
            "resume-report" => resume_report::run(args),
            "check" => check::run(args),
            _ => unreachable!("deploy dispatch command only defines known commands"),
        },
    }
}

fn run_promote<I>(args: I) -> Result<(), DeployCommandError>
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
        Some((command, args)) if command == "inspect" => run_promote_inspect(args),
        Some((command, args)) if command == "plan" => run_promote_plan(args),
        Some((command, args)) if command == "check" => run_promote_check(args),
        Some((command, args)) if command == "diff" => run_promote_diff(args),
        _ => {
            println!("{}", promote_usage());
            Ok(())
        }
    }
}

fn run_promote_inspect<I>(args: I) -> Result<(), DeployCommandError>
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
        Some((command, args)) if command == "readiness" => run_promote_readiness(args),
        Some((command, args)) if command == "artifact-identity" => {
            run_promote_artifact_identity(args)
        }
        Some((command, args)) if command == "transform" => run_promote_transform(args),
        Some((command, args)) if command == "transform-evidence" => {
            run_promote_transform_evidence(args)
        }
        Some((command, args)) if command == "target-lineage" => run_promote_target_lineage(args),
        Some((command, args)) if command == "provenance" => run_promote_provenance(args),
        Some((command, args)) if command == "wasm-store-identity" => {
            run_promote_wasm_store_identity(args)
        }
        Some((command, args)) if command == "catalog-verification" => {
            run_promote_catalog_verification(args)
        }
        Some((command, args)) if command == "execution-receipt" => {
            run_promote_execution_receipt(args)
        }
        Some((command, args)) if command == "policy" => run_promote_policy_check(args),
        Some((command, args)) if command == "materialization-identity" => {
            run_promote_materialization_identity(args)
        }
        _ => {
            println!("{}", promote_inspect_usage());
            Ok(())
        }
    }
}

fn run_promote_readiness<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_promote_output(
        args,
        deploy_promote_readiness_command,
        promote_readiness_usage,
        build_promotion_readiness,
        promotion_readiness_text,
    )
}

fn run_promote_check<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_promote_output(
        args,
        deploy_promote_check_command,
        promote_check_usage,
        build_promotion_readiness,
        promotion_readiness_text,
    )
}

fn run_promote_artifact_identity<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_promote_output(
        args,
        deploy_promote_artifact_identity_command,
        promote_artifact_identity_usage,
        build_promotion_artifact_identity_report,
        promotion_artifact_identity_report_text,
    )
}

fn run_promote_transform<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_promote_output(
        args,
        deploy_promote_transform_command,
        promote_transform_usage,
        build_promotion_plan_transform,
        promotion_plan_transform_text,
    )
}

fn run_promote_diff<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_promote_output(
        args,
        deploy_promote_diff_command,
        promote_diff_usage,
        build_promotion_plan_transform,
        promotion_plan_transform_text,
    )
}

fn run_promote_transform_evidence<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_promote_output(
        args,
        deploy_promote_transform_evidence_command,
        promote_transform_evidence_usage,
        build_promotion_plan_transform_evidence,
        promotion_plan_transform_evidence_text,
    )
}

fn run_promote_target_lineage<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_promote_output(
        args,
        deploy_promote_target_lineage_command,
        promote_target_lineage_usage,
        build_promotion_target_execution_lineage,
        promotion_target_execution_lineage_text,
    )
}

fn run_promote_plan<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_promote_output(
        args,
        deploy_promote_plan_command,
        promote_plan_usage,
        build_artifact_promotion_plan,
        artifact_promotion_plan_text,
    )
}

fn run_promote_provenance<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_promote_output(
        args,
        deploy_promote_provenance_command,
        promote_provenance_usage,
        build_artifact_promotion_provenance_report,
        artifact_promotion_provenance_report_text,
    )
}

fn run_promote_wasm_store_identity<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_promote_output(
        args,
        deploy_promote_wasm_store_identity_command,
        promote_wasm_store_identity_usage,
        build_promotion_wasm_store_identity_report,
        promotion_wasm_store_identity_report_text,
    )
}

fn run_promote_catalog_verification<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_promote_output(
        args,
        deploy_promote_catalog_verification_command,
        promote_catalog_verification_usage,
        build_promotion_wasm_store_catalog_verification,
        promotion_wasm_store_catalog_verification_text,
    )
}

fn run_promote_execution_receipt<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_promote_output(
        args,
        deploy_promote_execution_receipt_command,
        promote_execution_receipt_usage,
        build_artifact_promotion_execution_receipt,
        artifact_promotion_execution_receipt_text,
    )
}

fn run_promote_policy_check<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_promote_output(
        args,
        deploy_promote_policy_check_command,
        promote_policy_check_usage,
        build_promotion_policy_check,
        promotion_policy_check_text,
    )
}

fn run_promote_materialization_identity<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_promote_output(
        args,
        deploy_promote_materialization_identity_command,
        promote_materialization_identity_usage,
        build_promotion_materialization_identity_report,
        promotion_materialization_identity_report_text,
    )
}

fn run_promote_output<I, T, R>(
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

fn build_promotion_readiness(
    request: PromotionReadinessFile,
) -> Result<PromotionReadinessV1, DeployCommandError> {
    check_promotion_readiness(&PromotionReadinessRequest {
        readiness_id: request.readiness_id,
        target_plan: request.target_plan,
        inputs: request.inputs,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

fn build_promotion_artifact_identity_report(
    request: PromotionArtifactIdentityFile,
) -> Result<PromotionArtifactIdentityReportV1, DeployCommandError> {
    promotion_artifact_identity_report_from_inputs(PromotionArtifactIdentityReportRequest {
        report_id: request.report_id,
        inputs: request.inputs,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

fn build_promotion_plan_transform(
    request: PromotionPlanTransformFile,
) -> Result<PromotionPlanTransformV1, DeployCommandError> {
    if let Some(materialization_evidence) = request.materialization_evidence {
        return promoted_deployment_plan_transform_from_inputs_with_materialization(
            &PromotionPlanTransformWithMaterializationRequest {
                promoted_plan_id: request.promoted_plan_id,
                target_plan: request.target_plan,
                inputs: request.inputs,
                materialization_evidence,
            },
        )
        .map_err(|err| DeployCommandError::Check(Box::new(err)));
    }

    promoted_deployment_plan_transform_from_inputs(&PromotionPlanTransformRequest {
        promoted_plan_id: request.promoted_plan_id,
        target_plan: request.target_plan,
        inputs: request.inputs,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

fn build_promotion_plan_transform_evidence(
    request: PromotionPlanTransformEvidenceFile,
) -> Result<PromotionPlanTransformEvidenceV1, DeployCommandError> {
    promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: request.evidence_id,
        generated_at: request.generated_at,
        transform: request.transform,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

fn build_promotion_target_execution_lineage(
    request: PromotionTargetExecutionLineageFile,
) -> Result<PromotionTargetExecutionLineageV1, DeployCommandError> {
    promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: request.lineage_id,
        generated_at: request.generated_at,
        transform: request.transform,
        execution_preflight: request.execution_preflight,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

fn build_artifact_promotion_plan(
    request: ArtifactPromotionPlanFile,
) -> Result<ArtifactPromotionPlanV1, DeployCommandError> {
    artifact_promotion_plan(ArtifactPromotionPlanRequest {
        plan_id: request.plan_id,
        generated_at: request.generated_at,
        readiness: request.readiness,
        artifact_identity_report: request.artifact_identity_report,
        transform: request.transform,
        target_execution_lineage: request.target_execution_lineage,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

fn build_artifact_promotion_provenance_report(
    request: ArtifactPromotionProvenanceFile,
) -> Result<ArtifactPromotionProvenanceReportV1, DeployCommandError> {
    artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
        report_id: request.report_id,
        artifact_promotion_plan: request.artifact_promotion_plan,
        wasm_store_identity_report: request.wasm_store_identity_report,
        wasm_store_catalog_verification: request.wasm_store_catalog_verification,
        materialization_identity_report: request.materialization_identity_report,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

fn build_promotion_wasm_store_identity_report(
    request: PromotionWasmStoreIdentityFile,
) -> Result<PromotionWasmStoreIdentityReportV1, DeployCommandError> {
    promotion_wasm_store_identity_report_from_staging(PromotionWasmStoreIdentityReportRequest {
        report_id: request.report_id,
        staging_receipts: request.staging_receipts,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

fn build_promotion_wasm_store_catalog_verification(
    request: PromotionWasmStoreCatalogVerificationFile,
) -> Result<PromotionWasmStoreCatalogVerificationV1, DeployCommandError> {
    promotion_wasm_store_catalog_verification(PromotionWasmStoreCatalogVerificationRequest {
        verification_id: request.verification_id,
        wasm_store_identity_report: request.wasm_store_identity_report,
        catalog_entries: request.catalog_entries,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

fn build_artifact_promotion_execution_receipt(
    request: ArtifactPromotionExecutionReceiptFile,
) -> Result<ArtifactPromotionExecutionReceiptV1, DeployCommandError> {
    artifact_promotion_execution_receipt(ArtifactPromotionExecutionReceiptRequest {
        receipt_id: request.receipt_id,
        provenance_report: request.provenance_report,
        deployment_receipt: request.deployment_receipt,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

fn build_promotion_policy_check(
    request: PromotionPolicyCheckFile,
) -> Result<PromotionPolicyCheckV1, DeployCommandError> {
    check_promotion_policy(PromotionPolicyCheckRequest {
        check_id: request.check_id,
        inputs: request.inputs,
        policies: request.policies,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

fn build_promotion_materialization_identity_report(
    request: PromotionMaterializationIdentityFile,
) -> Result<PromotionMaterializationIdentityReportV1, DeployCommandError> {
    promotion_materialization_identity_report_from_evidence(
        PromotionMaterializationIdentityReportRequest {
            report_id: request.report_id,
            evidence: request.evidence,
        },
    )
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

pub(super) fn load_deployment_check(
    options: DeployTruthOptions,
) -> Result<DeploymentCheckV1, DeployCommandError> {
    let icp_root = resolve_current_canic_icp_root().ok();
    check_install_deployment_truth(
        &options.into_install_root_options_with_icp_root(icp_root),
        current_observed_at()?,
    )
    .map_err(DeployCommandError::from)
}

pub(super) fn print_json<T>(value: &T) -> Result<(), DeployCommandError>
where
    T: serde::Serialize,
{
    let json = serde_json::to_string_pretty(value).map_err(Box::<dyn std::error::Error>::from)?;
    println!("{json}");
    Ok(())
}

pub(super) fn read_json_file<T>(path: &PathBuf) -> Result<T, DeployCommandError>
where
    T: DeserializeOwned,
{
    let bytes = fs::read(path).map_err(Box::<dyn std::error::Error>::from)?;
    serde_json::from_slice(&bytes)
        .map_err(Box::<dyn std::error::Error>::from)
        .map_err(DeployCommandError::from)
}

impl DeployPromoteReportOptions {
    fn parse<I>(
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
            format: parse_promotion_output_format(
                string_option(&matches, "format").as_deref(),
                usage,
            )?,
        })
    }
}

impl DeployTruthOptions {
    fn parse<I>(
        args: I,
        command: impl FnOnce() -> ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        Self::from_matches(&matches, usage)
    }

    pub(super) fn from_matches(
        matches: &clap::ArgMatches,
        usage: fn() -> String,
    ) -> Result<Self, DeployCommandError> {
        Ok(Self {
            deployment: string_option(matches, "deployment").expect("clap requires deployment"),
            network: string_option(matches, "network").unwrap_or_else(local_network),
            profile: string_option(matches, "profile")
                .as_deref()
                .map(|profile| parse_profile(profile, usage))
                .transpose()?,
        })
    }

    fn into_install_root_options_with_icp_root(
        self,
        icp_root: Option<std::path::PathBuf>,
    ) -> InstallRootOptions {
        InstallRootOptions {
            root_canister: DEFAULT_ROOT_TARGET.to_string(),
            root_build_target: DEFAULT_ROOT_TARGET.to_string(),
            network: self.network,
            deployment_name: Some(self.deployment),
            icp_root,
            build_profile: self.profile,
            ready_timeout_seconds: DEFAULT_READY_TIMEOUT_SECONDS,
            config_path: None,
            expected_fleet: None,
            interactive_config_selection: false,
            deployment_plan_override: None,
            artifact_promotion_plan_override: None,
        }
    }
}

fn deploy_command() -> ClapCommand {
    ClapCommand::new("deploy")
        .bin_name("canic deploy")
        .about("Check deployment truth before mutation")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("authority")
                .about("Dry-run controller authority reconciliation")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("catalog")
                .about("List or inspect known deployment targets")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("external")
                .about("Build passive external lifecycle reports")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("promote")
                .about("Build passive artifact promotion reports")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("root")
                .about("Inspect or verify deployment-root evidence")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("install")
                .about("Install through the current runner using a supplied deployment plan")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("register")
                .about("Register minimal deployment-target state")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("compare")
                .about("Compare two deployment truth check artifacts")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("check")
                .about("Print the local deployment truth check JSON")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("diff")
                .about("Print the local deployment diff JSON")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("inventory")
                .about("Print the local deployment inventory JSON")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("plan")
                .about("Print the local deployment plan JSON")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("report")
                .about("Print the local deployment safety report JSON")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("resume-report")
                .about("Print passive resume safety JSON from a receipt")
                .disable_help_flag(true),
        ))
        .after_help(DEPLOY_HELP_AFTER)
}

fn deploy_promote_command() -> ClapCommand {
    ClapCommand::new("promote")
        .bin_name("canic deploy promote")
        .about("Build passive artifact promotion reports")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("plan")
                .about("Build a passive artifact promotion plan")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("check")
                .about("Build a passive artifact promotion readiness check")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("diff")
                .about("Build a passive artifact promotion diff")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("inspect")
                .about("Inspect passive artifact promotion internals")
                .disable_help_flag(true),
        ))
        .after_help(DEPLOY_PROMOTE_HELP_AFTER)
}

fn deploy_promote_inspect_command() -> ClapCommand {
    ClapCommand::new("inspect")
        .bin_name("canic deploy promote inspect")
        .about("Inspect passive artifact promotion internals")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("readiness")
                .about("Build a passive promotion readiness report")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("artifact-identity")
                .about("Build a passive promotion artifact identity report")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("transform")
                .about("Build a passive promoted-plan transform")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("transform-evidence")
                .about("Build passive promoted-plan transform evidence")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("target-lineage")
                .about("Build passive target execution lineage")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("provenance")
                .about("Build a passive artifact promotion provenance report")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("wasm-store-identity")
                .about("Build a passive wasm-store identity report")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("catalog-verification")
                .about("Build a passive wasm-store catalog verification report")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("execution-receipt")
                .about("Build a passive artifact promotion execution receipt wrapper")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("policy")
                .about("Build a passive promotion policy check")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("materialization-identity")
                .about("Build a passive source/build materialization identity report")
                .disable_help_flag(true),
        ))
        .after_help(DEPLOY_PROMOTE_INSPECT_HELP_AFTER)
}

fn deploy_promote_readiness_command() -> ClapCommand {
    deploy_promote_report_command(
        "readiness",
        "Build a passive promotion readiness report",
        "canic deploy promote inspect readiness",
    )
    .after_help(DEPLOY_PROMOTE_READINESS_HELP_AFTER)
}

fn deploy_promote_check_command() -> ClapCommand {
    deploy_promote_report_command(
        "check",
        "Build a passive artifact promotion readiness check",
        "canic deploy promote check",
    )
    .after_help(DEPLOY_PROMOTE_CHECK_HELP_AFTER)
}

fn deploy_promote_artifact_identity_command() -> ClapCommand {
    deploy_promote_report_command(
        "artifact-identity",
        "Build a passive promotion artifact identity report",
        "canic deploy promote inspect artifact-identity",
    )
    .after_help(DEPLOY_PROMOTE_ARTIFACT_IDENTITY_HELP_AFTER)
}

fn deploy_promote_transform_command() -> ClapCommand {
    deploy_promote_report_command(
        "transform",
        "Build a passive promoted-plan transform",
        "canic deploy promote inspect transform",
    )
    .after_help(DEPLOY_PROMOTE_TRANSFORM_HELP_AFTER)
}

fn deploy_promote_diff_command() -> ClapCommand {
    deploy_promote_report_command(
        "diff",
        "Build a passive artifact promotion diff",
        "canic deploy promote diff",
    )
    .after_help(DEPLOY_PROMOTE_DIFF_HELP_AFTER)
}

fn deploy_promote_transform_evidence_command() -> ClapCommand {
    deploy_promote_report_command(
        "transform-evidence",
        "Build passive promoted-plan transform evidence",
        "canic deploy promote inspect transform-evidence",
    )
    .after_help(DEPLOY_PROMOTE_TRANSFORM_EVIDENCE_HELP_AFTER)
}

fn deploy_promote_target_lineage_command() -> ClapCommand {
    deploy_promote_report_command(
        "target-lineage",
        "Build passive target execution lineage",
        "canic deploy promote inspect target-lineage",
    )
    .after_help(DEPLOY_PROMOTE_TARGET_LINEAGE_HELP_AFTER)
}

fn deploy_promote_plan_command() -> ClapCommand {
    deploy_promote_report_command(
        "plan",
        "Build a passive artifact promotion plan",
        "canic deploy promote plan",
    )
    .after_help(DEPLOY_PROMOTE_PLAN_HELP_AFTER)
}

fn deploy_promote_provenance_command() -> ClapCommand {
    deploy_promote_report_command(
        "provenance",
        "Build a passive artifact promotion provenance report",
        "canic deploy promote inspect provenance",
    )
    .after_help(DEPLOY_PROMOTE_PROVENANCE_HELP_AFTER)
}

fn deploy_promote_wasm_store_identity_command() -> ClapCommand {
    deploy_promote_report_command(
        "wasm-store-identity",
        "Build a passive wasm-store identity report",
        "canic deploy promote inspect wasm-store-identity",
    )
    .after_help(DEPLOY_PROMOTE_WASM_STORE_IDENTITY_HELP_AFTER)
}

fn deploy_promote_catalog_verification_command() -> ClapCommand {
    deploy_promote_report_command(
        "catalog-verification",
        "Build a passive wasm-store catalog verification report",
        "canic deploy promote inspect catalog-verification",
    )
    .after_help(DEPLOY_PROMOTE_CATALOG_VERIFICATION_HELP_AFTER)
}

fn deploy_promote_execution_receipt_command() -> ClapCommand {
    deploy_promote_report_command(
        "execution-receipt",
        "Build a passive artifact promotion execution receipt wrapper",
        "canic deploy promote inspect execution-receipt",
    )
    .after_help(DEPLOY_PROMOTE_EXECUTION_RECEIPT_HELP_AFTER)
}

fn deploy_promote_policy_check_command() -> ClapCommand {
    deploy_promote_report_command(
        "policy",
        "Build a passive promotion policy check",
        "canic deploy promote inspect policy",
    )
    .after_help(DEPLOY_PROMOTE_POLICY_CHECK_HELP_AFTER)
}

fn deploy_promote_materialization_identity_command() -> ClapCommand {
    deploy_promote_report_command(
        "materialization-identity",
        "Build a passive source/build materialization identity report",
        "canic deploy promote inspect materialization-identity",
    )
    .after_help(DEPLOY_PROMOTE_MATERIALIZATION_IDENTITY_HELP_AFTER)
}

fn deploy_promote_report_command(
    name: &'static str,
    about: &'static str,
    bin_name: &'static str,
) -> ClapCommand {
    ClapCommand::new(name)
        .bin_name(bin_name)
        .about(about)
        .disable_help_flag(true)
        .arg(
            value_arg("request")
                .long("request")
                .value_name("file")
                .required(true)
                .help("Request JSON file for the passive promotion report"),
        )
        .arg(promotion_format_arg())
}

pub(super) fn deploy_truth_leaf_command(name: &'static str, about: &'static str) -> ClapCommand {
    ClapCommand::new(name)
        .bin_name(format!("canic deploy {name}"))
        .about(about)
        .disable_help_flag(true)
        .arg(
            value_arg("deployment")
                .value_name("deployment")
                .required(true)
                .help("Deployment target name to check"),
        )
        .arg(
            value_arg("profile")
                .long("profile")
                .value_name("debug|fast|release")
                .num_args(1)
                .help("Expected canister wasm build profile"),
        )
        .arg(internal_network_arg())
}

fn promotion_format_arg() -> clap::Arg {
    value_arg("format")
        .long("format")
        .value_name("json|text")
        .num_args(1)
        .help("Output format; defaults to json")
}

fn usage() -> String {
    let mut command = deploy_command();
    command.render_help().to_string()
}

fn promote_usage() -> String {
    let mut command = deploy_promote_command();
    command.render_help().to_string()
}

fn promote_inspect_usage() -> String {
    let mut command = deploy_promote_inspect_command();
    command.render_help().to_string()
}

fn promote_readiness_usage() -> String {
    let mut command = deploy_promote_readiness_command();
    command.render_help().to_string()
}

fn promote_check_usage() -> String {
    let mut command = deploy_promote_check_command();
    command.render_help().to_string()
}

fn promote_artifact_identity_usage() -> String {
    let mut command = deploy_promote_artifact_identity_command();
    command.render_help().to_string()
}

fn promote_transform_usage() -> String {
    let mut command = deploy_promote_transform_command();
    command.render_help().to_string()
}

fn promote_diff_usage() -> String {
    let mut command = deploy_promote_diff_command();
    command.render_help().to_string()
}

fn promote_transform_evidence_usage() -> String {
    let mut command = deploy_promote_transform_evidence_command();
    command.render_help().to_string()
}

fn promote_target_lineage_usage() -> String {
    let mut command = deploy_promote_target_lineage_command();
    command.render_help().to_string()
}

fn promote_plan_usage() -> String {
    let mut command = deploy_promote_plan_command();
    command.render_help().to_string()
}

fn promote_provenance_usage() -> String {
    let mut command = deploy_promote_provenance_command();
    command.render_help().to_string()
}

fn promote_wasm_store_identity_usage() -> String {
    let mut command = deploy_promote_wasm_store_identity_command();
    command.render_help().to_string()
}

fn promote_catalog_verification_usage() -> String {
    let mut command = deploy_promote_catalog_verification_command();
    command.render_help().to_string()
}

fn promote_execution_receipt_usage() -> String {
    let mut command = deploy_promote_execution_receipt_command();
    command.render_help().to_string()
}

fn promote_policy_check_usage() -> String {
    let mut command = deploy_promote_policy_check_command();
    command.render_help().to_string()
}

fn promote_materialization_identity_usage() -> String {
    let mut command = deploy_promote_materialization_identity_command();
    command.render_help().to_string()
}

fn parse_profile(
    value: &str,
    usage: fn() -> String,
) -> Result<CanisterBuildProfile, DeployCommandError> {
    match value {
        "debug" => Ok(CanisterBuildProfile::Debug),
        "fast" => Ok(CanisterBuildProfile::Fast),
        "release" => Ok(CanisterBuildProfile::Release),
        _ => Err(DeployCommandError::Usage(format!(
            "invalid build profile: {value}\n\n{}",
            usage()
        ))),
    }
}

pub(super) fn current_observed_at() -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!(
        "unix:{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
    ))
}

#[cfg(test)]
mod tests;
