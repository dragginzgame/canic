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
        AuthorityDryRunEvidenceV1, BuildMaterializationEvidenceV1, DeploymentCheckV1,
        DeploymentExecutionPreflightV1, DeploymentPlanV1, DeploymentReceiptV1,
        PromotionArtifactIdentityReportRequest, PromotionArtifactIdentityReportV1,
        PromotionMaterializationIdentityReportRequest, PromotionMaterializationIdentityReportV1,
        PromotionPlanTransformEvidenceRequest, PromotionPlanTransformEvidenceV1,
        PromotionPlanTransformRequest, PromotionPlanTransformV1,
        PromotionPlanTransformWithMaterializationRequest, PromotionPolicyCheckRequest,
        PromotionPolicyCheckV1, PromotionReadinessRequest, PromotionReadinessV1,
        PromotionTargetExecutionLineageRequest, PromotionTargetExecutionLineageV1,
        PromotionWasmStoreCatalogEntryV1, PromotionWasmStoreCatalogVerificationRequest,
        PromotionWasmStoreCatalogVerificationV1, PromotionWasmStoreIdentityReportRequest,
        PromotionWasmStoreIdentityReportV1, RolePromotionInputV1, RolePromotionPolicyV1,
        SafetyReportV1, SafetyStatusV1, StagingReceiptV1, artifact_promotion_execution_receipt,
        artifact_promotion_execution_receipt_text, artifact_promotion_plan,
        artifact_promotion_plan_text, artifact_promotion_provenance_report,
        artifact_promotion_provenance_report_text,
        authority_dry_run_evidence_from_check_with_local_ids,
        authority_dry_run_receipt_from_check_with_local_id, authority_evidence_text,
        authority_plan_text, authority_receipt_text, authority_report_from_check_with_local_id,
        authority_report_text, build_authority_reconciliation_plan, check_promotion_policy,
        check_promotion_readiness, compare_plan_inventory_and_receipt,
        promoted_deployment_plan_transform_from_inputs,
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
    install_root::{
        InstallRootOptions, check_install_deployment_truth,
        latest_deployment_truth_receipt_path_from_root,
    },
};
use clap::Command as ClapCommand;
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
  canic deploy diff demo
  canic deploy report demo
  canic deploy check demo
  canic deploy authority check demo
  canic deploy authority evidence demo
  canic deploy authority report demo
  canic deploy authority receipt demo
  canic deploy promote plan --request promotion-plan.json
  canic deploy promote check --request promotion-check.json
  canic deploy promote diff --request promotion-diff.json
  canic deploy promote inspect readiness --request promotion-readiness.json
  canic deploy promote inspect artifact-identity --request promotion-artifacts.json
  canic deploy promote inspect provenance --request promotion-provenance.json
  canic deploy resume-report demo
  canic deploy resume-report --receipt receipt.json demo
  canic deploy check --profile fast demo

Deployment truth commands are read-only checks. Mutation still flows through
`canic install`. Authority commands are dry-run reconciliation reports and do
not mutate controller state.";
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
const DEPLOY_CHECK_HELP_AFTER: &str = "\
Examples:
  canic deploy check demo
  canic --network local deploy check --profile fast demo

Prints the local DeploymentCheckV1 JSON without installing or mutating state.";
const DEPLOY_AUTHORITY_HELP_AFTER: &str = "\
Examples:
  canic deploy authority check demo
  canic deploy authority evidence demo
  canic deploy authority report demo
  canic deploy authority receipt demo
  canic --network local deploy authority check --profile fast demo

0.42 authority commands are dry-run reports. They do not apply controller
changes. A successful command means the local authority artifact was produced,
not that the deployment is globally safe or that controller state was changed.";
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
const DEPLOY_AUTHORITY_EVIDENCE_HELP_AFTER: &str = "\
Examples:
  canic deploy authority evidence demo
  canic deploy authority evidence --format text demo
  canic --network local deploy authority evidence --profile fast demo

Prints AuthorityDryRunEvidenceV1 JSON by default, or a human-readable
read-only summary with --format text. No controller changes are attempted.
Success means evidence generation succeeded, not that every deployment safety
check is clean.";
const DEPLOY_AUTHORITY_CHECK_HELP_AFTER: &str = "\
Examples:
  canic deploy authority check demo
  canic deploy authority check --format text demo
  canic --network local deploy authority check --profile fast demo

Prints the local AuthorityReconciliationPlanV1 JSON by default, or a
human-readable read-only summary with --format text. No controller changes are
attempted. Success means the local plan was produced.";
const DEPLOY_AUTHORITY_REPORT_HELP_AFTER: &str = "\
Examples:
  canic deploy authority report demo
  canic deploy authority report --format text demo
  canic --network local deploy authority report --profile fast demo

Prints the local AuthorityReportV1 JSON by default, or a human-readable
read-only summary with --format text. No controller changes are attempted.
Authority status is authority-scoped; it is not a whole-deployment safety
verdict.";
const DEPLOY_AUTHORITY_RECEIPT_HELP_AFTER: &str = "\
Examples:
  canic deploy authority receipt demo
  canic deploy authority receipt --format text demo
  canic --network local deploy authority receipt --profile fast demo

Prints an evidence-only AuthorityReceiptV1 JSON by default, or a human-readable
read-only summary with --format text. No controller changes are attempted.
Success means the dry-run receipt was produced with zero attempted controller
actions.";
const DEPLOY_RESUME_REPORT_HELP_AFTER: &str = "\
Examples:
  canic deploy resume-report demo
  canic deploy resume-report --receipt receipt.json demo
  canic --network local deploy resume-report --receipt receipt.json --profile fast demo

Prints the passive ResumeSafetyV1 JSON for the current deployment truth check
and a prior DeploymentReceiptV1. When --receipt is omitted, Canic uses the
latest local receipt under .canic/<network>/deployment-receipts/<fleet>. It
does not resume, install, or mutate state.";

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
struct DeployTruthOptions {
    fleet: String,
    network: String,
    profile: Option<CanisterBuildProfile>,
}

///
/// DeployResumeReportOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeployResumeReportOptions {
    truth: DeployTruthOptions,
    receipt: Option<PathBuf>,
}

/// DeployAuthorityOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeployAuthorityOptions {
    truth: DeployTruthOptions,
    format: AuthorityOutputFormat,
}

///
/// DeployPromoteReportOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeployPromoteReportOptions {
    request: PathBuf,
    format: PromotionOutputFormat,
}

///
/// AuthorityOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AuthorityOutputFormat {
    Json,
    Text,
}

///
/// PromotionOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PromotionOutputFormat {
    Json,
    Text,
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
            "authority" => run_authority(args),
            "promote" => run_promote(args),
            "plan" => run_plan(args),
            "inventory" => run_inventory(args),
            "diff" => run_diff(args),
            "report" => run_report(args),
            "resume-report" => run_resume_report(args),
            "check" => run_check(args),
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

fn run_authority<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, authority_usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(deploy_authority_command(), args)
        .map_err(|_| DeployCommandError::Usage(authority_usage()))?
    {
        Some((command, args)) if command == "check" => run_authority_check(args),
        Some((command, args)) if command == "evidence" => run_authority_evidence(args),
        Some((command, args)) if command == "report" => run_authority_report(args),
        Some((command, args)) if command == "receipt" => run_authority_receipt(args),
        _ => {
            println!("{}", authority_usage());
            Ok(())
        }
    }
}

fn run_authority_evidence<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_authority_output(
        args,
        deploy_authority_evidence_command,
        authority_evidence_usage,
        build_authority_dry_run_evidence,
        authority_evidence_text,
    )
}

fn run_authority_receipt<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_authority_output(
        args,
        deploy_authority_receipt_command,
        authority_receipt_usage,
        build_authority_dry_run_receipt,
        authority_receipt_text,
    )
}

fn run_authority_report<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_authority_output(
        args,
        deploy_authority_report_command,
        authority_report_usage,
        |check| Ok(authority_report_from_check_with_local_id(check)),
        authority_report_text,
    )
}

fn run_authority_check<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_authority_output(
        args,
        deploy_authority_check_command,
        authority_check_usage,
        |check| Ok(build_authority_reconciliation_plan(check)),
        authority_plan_text,
    )
}

fn run_authority_output<I, T>(
    args: I,
    command: impl FnOnce() -> ClapCommand,
    usage: fn() -> String,
    build: impl FnOnce(&DeploymentCheckV1) -> Result<T, DeployCommandError>,
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

    let options = DeployAuthorityOptions::parse(args, command, usage)?;
    let check = load_deployment_check(options.truth)?;
    let output = build(&check)?;
    match options.format {
        AuthorityOutputFormat::Json => print_json(&output)?,
        AuthorityOutputFormat::Text => println!("{}", render_text(&output)),
    }
    Ok(())
}

fn build_authority_dry_run_evidence(
    check: &DeploymentCheckV1,
) -> Result<AuthorityDryRunEvidenceV1, DeployCommandError> {
    let generated_at = current_observed_at()?;
    authority_dry_run_evidence_from_check_with_local_ids(check, generated_at)
        .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

fn build_authority_dry_run_receipt(
    check: &DeploymentCheckV1,
) -> Result<canic_host::deployment_truth::AuthorityReceiptV1, DeployCommandError> {
    let generated_at = current_observed_at()?;
    authority_dry_run_receipt_from_check_with_local_id(check, generated_at)
        .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

fn run_plan<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, plan_usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(
        args,
        deploy_plan_command,
        plan_usage,
    )?)?;
    print_json(&check.plan)?;
    Ok(())
}

fn run_inventory<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, inventory_usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(
        args,
        deploy_inventory_command,
        inventory_usage,
    )?)?;
    print_json(&check.inventory)?;
    Ok(())
}

fn run_diff<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, diff_usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(
        args,
        deploy_diff_command,
        diff_usage,
    )?)?;
    print_json(&check.diff)?;
    Ok(())
}

fn run_report<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, report_usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(
        args,
        deploy_report_command,
        report_usage,
    )?)?;
    print_json(&check.report)?;
    Ok(())
}

fn run_check<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, check_usage, version_text()) {
        return Ok(());
    }

    let check = load_deployment_check(DeployTruthOptions::parse(
        args,
        deploy_check_command,
        check_usage,
    )?)?;
    print_json(&check)?;
    enforce_deployment_check_status(&check.report)
}

fn run_resume_report<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, resume_report_usage, version_text()) {
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

fn load_deployment_check(
    options: DeployTruthOptions,
) -> Result<DeploymentCheckV1, DeployCommandError> {
    let icp_root = resolve_current_canic_icp_root().ok();
    check_install_deployment_truth(
        &options.into_install_root_options_with_icp_root(icp_root),
        current_observed_at()?,
    )
    .map_err(DeployCommandError::from)
}

fn print_json<T>(value: &T) -> Result<(), DeployCommandError>
where
    T: serde::Serialize,
{
    let json = serde_json::to_string_pretty(value).map_err(Box::<dyn std::error::Error>::from)?;
    println!("{json}");
    Ok(())
}

fn read_deployment_receipt(path: &PathBuf) -> Result<DeploymentReceiptV1, DeployCommandError> {
    read_json_file(path)
}

fn read_json_file<T>(path: &PathBuf) -> Result<T, DeployCommandError>
where
    T: DeserializeOwned,
{
    let bytes = fs::read(path).map_err(Box::<dyn std::error::Error>::from)?;
    serde_json::from_slice(&bytes)
        .map_err(Box::<dyn std::error::Error>::from)
        .map_err(DeployCommandError::from)
}

fn enforce_deployment_check_status(report: &SafetyReportV1) -> Result<(), DeployCommandError> {
    if report.status == SafetyStatusV1::Blocked {
        return Err(DeployCommandError::Blocked(report.summary.clone()));
    }
    Ok(())
}

impl DeployResumeReportOptions {
    fn parse<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(deploy_resume_report_command(), args)
            .map_err(|_| DeployCommandError::Usage(resume_report_usage()))?;
        Ok(Self {
            truth: DeployTruthOptions::from_matches(&matches, resume_report_usage)?,
            receipt: path_option(&matches, "receipt"),
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
            &self.truth.fleet,
        )
        .map_err(DeployCommandError::from)?
        .ok_or_else(|| {
            DeployCommandError::Usage(format!(
                "no deployment receipt found under {} for fleet {}; pass --receipt <file>",
                icp_root
                    .join(".canic")
                    .join(&self.truth.network)
                    .join("deployment-receipts")
                    .join(&self.truth.fleet)
                    .display(),
                self.truth.fleet
            ))
        })
    }
}

impl DeployAuthorityOptions {
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
            truth: DeployTruthOptions::from_matches(&matches, usage)?,
            format: parse_authority_output_format(
                string_option(&matches, "format").as_deref(),
                usage,
            )?,
        })
    }
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

    fn from_matches(
        matches: &clap::ArgMatches,
        usage: fn() -> String,
    ) -> Result<Self, DeployCommandError> {
        Ok(Self {
            fleet: string_option(matches, "fleet").expect("clap requires fleet"),
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
            icp_root,
            build_profile: self.profile,
            ready_timeout_seconds: DEFAULT_READY_TIMEOUT_SECONDS,
            config_path: Some(default_fleet_config_path(&self.fleet)),
            expected_fleet: Some(self.fleet),
            interactive_config_selection: false,
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
            ClapCommand::new("promote")
                .about("Build passive artifact promotion reports")
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

fn deploy_authority_command() -> ClapCommand {
    ClapCommand::new("authority")
        .bin_name("canic deploy authority")
        .about("Dry-run controller authority reconciliation")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("check")
                .about("Print the local authority reconciliation plan")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("evidence")
                .about("Print the local authority dry-run evidence")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("report")
                .about("Print the local authority reconciliation report")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("receipt")
                .about("Print the local authority dry-run receipt")
                .disable_help_flag(true),
        ))
        .after_help(DEPLOY_AUTHORITY_HELP_AFTER)
}

fn deploy_authority_check_command() -> ClapCommand {
    deploy_truth_leaf_command("check", "Print the local authority reconciliation plan")
        .arg(authority_format_arg())
        .bin_name("canic deploy authority check")
        .after_help(DEPLOY_AUTHORITY_CHECK_HELP_AFTER)
}

fn deploy_authority_evidence_command() -> ClapCommand {
    deploy_truth_leaf_command("evidence", "Print the local authority dry-run evidence")
        .arg(authority_format_arg())
        .bin_name("canic deploy authority evidence")
        .after_help(DEPLOY_AUTHORITY_EVIDENCE_HELP_AFTER)
}

fn deploy_authority_report_command() -> ClapCommand {
    deploy_truth_leaf_command("report", "Print the local authority reconciliation report")
        .arg(authority_format_arg())
        .bin_name("canic deploy authority report")
        .after_help(DEPLOY_AUTHORITY_REPORT_HELP_AFTER)
}

fn deploy_authority_receipt_command() -> ClapCommand {
    deploy_truth_leaf_command("receipt", "Print the local authority dry-run receipt")
        .arg(authority_format_arg())
        .bin_name("canic deploy authority receipt")
        .after_help(DEPLOY_AUTHORITY_RECEIPT_HELP_AFTER)
}

fn deploy_plan_command() -> ClapCommand {
    deploy_truth_leaf_command("plan", "Print the local deployment plan JSON")
        .after_help(DEPLOY_PLAN_HELP_AFTER)
}

fn deploy_inventory_command() -> ClapCommand {
    deploy_truth_leaf_command("inventory", "Print the local deployment inventory JSON")
        .after_help(DEPLOY_INVENTORY_HELP_AFTER)
}

fn deploy_diff_command() -> ClapCommand {
    deploy_truth_leaf_command("diff", "Print the local deployment diff JSON")
        .after_help(DEPLOY_DIFF_HELP_AFTER)
}

fn deploy_report_command() -> ClapCommand {
    deploy_truth_leaf_command("report", "Print the local deployment safety report JSON")
        .after_help(DEPLOY_REPORT_HELP_AFTER)
}

fn deploy_check_command() -> ClapCommand {
    deploy_truth_leaf_command("check", "Print the local deployment truth check JSON")
        .after_help(DEPLOY_CHECK_HELP_AFTER)
}

fn deploy_resume_report_command() -> ClapCommand {
    deploy_truth_leaf_command(
        "resume-report",
        "Print passive resume safety JSON from a prior deployment receipt",
    )
    .arg(
        value_arg("receipt")
            .long("receipt")
            .value_name("file")
            .help("DeploymentReceiptV1 JSON file to compare with current deployment truth"),
    )
    .after_help(DEPLOY_RESUME_REPORT_HELP_AFTER)
}

fn deploy_truth_leaf_command(name: &'static str, about: &'static str) -> ClapCommand {
    ClapCommand::new(name)
        .bin_name(format!("canic deploy {name}"))
        .about(about)
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Config-defined fleet name to check"),
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

fn authority_format_arg() -> clap::Arg {
    value_arg("format")
        .long("format")
        .value_name("json|text")
        .num_args(1)
        .help("Output format; defaults to json")
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

fn plan_usage() -> String {
    let mut command = deploy_plan_command();
    command.render_help().to_string()
}

fn inventory_usage() -> String {
    let mut command = deploy_inventory_command();
    command.render_help().to_string()
}

fn diff_usage() -> String {
    let mut command = deploy_diff_command();
    command.render_help().to_string()
}

fn report_usage() -> String {
    let mut command = deploy_report_command();
    command.render_help().to_string()
}

fn check_usage() -> String {
    let mut command = deploy_check_command();
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

fn authority_usage() -> String {
    let mut command = deploy_authority_command();
    command.render_help().to_string()
}

fn authority_check_usage() -> String {
    let mut command = deploy_authority_check_command();
    command.render_help().to_string()
}

fn authority_evidence_usage() -> String {
    let mut command = deploy_authority_evidence_command();
    command.render_help().to_string()
}

fn authority_report_usage() -> String {
    let mut command = deploy_authority_report_command();
    command.render_help().to_string()
}

fn authority_receipt_usage() -> String {
    let mut command = deploy_authority_receipt_command();
    command.render_help().to_string()
}

fn resume_report_usage() -> String {
    let mut command = deploy_resume_report_command();
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

fn parse_promotion_output_format(
    value: Option<&str>,
    usage: fn() -> String,
) -> Result<PromotionOutputFormat, DeployCommandError> {
    match value.unwrap_or("json") {
        "json" => Ok(PromotionOutputFormat::Json),
        "text" => Ok(PromotionOutputFormat::Text),
        other => Err(DeployCommandError::Usage(format!(
            "invalid promotion output format: {other}\n\n{}",
            usage()
        ))),
    }
}

fn parse_authority_output_format(
    value: Option<&str>,
    usage: fn() -> String,
) -> Result<AuthorityOutputFormat, DeployCommandError> {
    match value.unwrap_or("json") {
        "json" => Ok(AuthorityOutputFormat::Json),
        "text" => Ok(AuthorityOutputFormat::Text),
        other => Err(DeployCommandError::Usage(format!(
            "invalid authority output format: {other}\n\n{}",
            usage()
        ))),
    }
}

fn default_fleet_config_path(fleet: &str) -> String {
    format!("fleets/{fleet}/canic.toml")
}

fn current_observed_at() -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!(
        "unix:{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_host::deployment_truth::{
        AuthorityProfileV1, CanisterControlClassV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        DeploymentDiffV1, DeploymentIdentityV1, DeploymentInventoryV1, DeploymentPlanV1,
        ExpectedCanisterV1, LocalDeploymentConfigV1, ObservationStatusV1, ObservedCanisterV1,
        ResumeSafetyV1, TrustDomainV1, VerifierReadinessExpectationV1,
        VerifierReadinessObservationV1,
    };

    #[test]
    fn deploy_check_parses_required_fleet() {
        let options =
            DeployTruthOptions::parse([OsString::from("demo")], deploy_check_command, check_usage)
                .expect("parse deploy check");

        assert_eq!(options.fleet, "demo");
        assert_eq!(options.network, "local");
        assert_eq!(options.profile, None);
    }

    #[test]
    fn deploy_check_accepts_internal_network_and_profile() {
        let options = DeployTruthOptions::parse(
            [
                OsString::from("--profile"),
                OsString::from("fast"),
                OsString::from("demo"),
                OsString::from("--__canic-network"),
                OsString::from("ic"),
            ],
            deploy_check_command,
            check_usage,
        )
        .expect("parse deploy check");

        assert_eq!(options.network, "ic");
        assert_eq!(options.profile, Some(CanisterBuildProfile::Fast));
    }

    #[test]
    fn deploy_check_rejects_invalid_profile() {
        assert!(matches!(
            DeployTruthOptions::parse(
                [
                    OsString::from("--profile"),
                    OsString::from("turbo"),
                    OsString::from("demo"),
                ],
                deploy_check_command,
                check_usage,
            ),
            Err(DeployCommandError::Usage(_))
        ));
    }

    #[test]
    fn deploy_check_status_rejects_blocked_report() {
        let report = SafetyReportV1 {
            schema_version: 1,
            report_id: "report-1".to_string(),
            diff_id: None,
            status: SafetyStatusV1::Blocked,
            summary: "deployment inventory has 1 blocking issue(s) and 0 warning(s)".to_string(),
            hard_failures: Vec::new(),
            warnings: Vec::new(),
            next_actions: Vec::new(),
        };

        assert!(matches!(
            enforce_deployment_check_status(&report),
            Err(DeployCommandError::Blocked(message))
                if message == "deployment inventory has 1 blocking issue(s) and 0 warning(s)"
        ));
    }

    #[test]
    fn deploy_check_status_allows_warning_report() {
        let report = SafetyReportV1 {
            schema_version: 1,
            report_id: "report-1".to_string(),
            diff_id: None,
            status: SafetyStatusV1::Warning,
            summary: "deployment inventory has 1 warning(s)".to_string(),
            hard_failures: Vec::new(),
            warnings: Vec::new(),
            next_actions: Vec::new(),
        };

        enforce_deployment_check_status(&report).expect("warning report should not fail check");
    }

    #[test]
    fn deploy_leaf_commands_parse_like_check() {
        let plan =
            DeployTruthOptions::parse([OsString::from("demo")], deploy_plan_command, plan_usage)
                .expect("parse deploy plan");
        let inventory = DeployTruthOptions::parse(
            [OsString::from("demo")],
            deploy_inventory_command,
            inventory_usage,
        )
        .expect("parse deploy inventory");
        let diff =
            DeployTruthOptions::parse([OsString::from("demo")], deploy_diff_command, diff_usage)
                .expect("parse deploy diff");
        let report = DeployTruthOptions::parse(
            [OsString::from("demo")],
            deploy_report_command,
            report_usage,
        )
        .expect("parse deploy report");
        let resume_report = DeployResumeReportOptions::parse([
            OsString::from("--receipt"),
            OsString::from("receipt.json"),
            OsString::from("demo"),
        ])
        .expect("parse deploy resume-report");

        assert_eq!(plan.fleet, "demo");
        assert_eq!(inventory.fleet, "demo");
        assert_eq!(diff.fleet, "demo");
        assert_eq!(report.fleet, "demo");
        assert_eq!(resume_report.truth.fleet, "demo");
        assert_eq!(resume_report.receipt, Some(PathBuf::from("receipt.json")));
    }

    #[test]
    fn deploy_authority_leaf_commands_default_to_json() {
        let authority_check = DeployAuthorityOptions::parse(
            [OsString::from("demo")],
            deploy_authority_check_command,
            authority_check_usage,
        )
        .expect("parse deploy authority check");
        let authority_evidence = DeployAuthorityOptions::parse(
            [OsString::from("demo")],
            deploy_authority_evidence_command,
            authority_evidence_usage,
        )
        .expect("parse deploy authority evidence");
        let authority_report = DeployAuthorityOptions::parse(
            [OsString::from("demo")],
            deploy_authority_report_command,
            authority_report_usage,
        )
        .expect("parse deploy authority report");
        let authority_receipt = DeployAuthorityOptions::parse(
            [OsString::from("demo")],
            deploy_authority_receipt_command,
            authority_receipt_usage,
        )
        .expect("parse deploy authority receipt");

        for options in [
            authority_check,
            authority_evidence,
            authority_report,
            authority_receipt,
        ] {
            assert_eq!(options.truth.fleet, "demo");
            assert_eq!(options.format, AuthorityOutputFormat::Json);
        }
    }

    #[test]
    fn deploy_authority_leaf_commands_parse_text_format() {
        let authority_check = DeployAuthorityOptions::parse(
            [
                OsString::from("--format"),
                OsString::from("text"),
                OsString::from("demo"),
            ],
            deploy_authority_check_command,
            authority_check_usage,
        )
        .expect("parse deploy authority check text");
        let authority_evidence = DeployAuthorityOptions::parse(
            [
                OsString::from("--format"),
                OsString::from("text"),
                OsString::from("demo"),
            ],
            deploy_authority_evidence_command,
            authority_evidence_usage,
        )
        .expect("parse deploy authority evidence text");
        let authority_report = DeployAuthorityOptions::parse(
            [
                OsString::from("--format"),
                OsString::from("text"),
                OsString::from("demo"),
            ],
            deploy_authority_report_command,
            authority_report_usage,
        )
        .expect("parse deploy authority report text");
        let authority_receipt = DeployAuthorityOptions::parse(
            [
                OsString::from("--format"),
                OsString::from("text"),
                OsString::from("demo"),
            ],
            deploy_authority_receipt_command,
            authority_receipt_usage,
        )
        .expect("parse deploy authority receipt text");

        assert_eq!(authority_check.truth.fleet, "demo");
        assert_eq!(authority_check.format, AuthorityOutputFormat::Text);
        assert_eq!(authority_evidence.truth.fleet, "demo");
        assert_eq!(authority_evidence.format, AuthorityOutputFormat::Text);
        assert_eq!(authority_report.truth.fleet, "demo");
        assert_eq!(authority_report.format, AuthorityOutputFormat::Text);
        assert_eq!(authority_receipt.truth.fleet, "demo");
        assert_eq!(authority_receipt.format, AuthorityOutputFormat::Text);
    }

    #[test]
    fn deploy_authority_command_help_does_not_claim_json_only_output() {
        let help = authority_usage();

        assert!(help.contains("Print the local authority reconciliation plan"));
        assert!(help.contains("Print the local authority dry-run evidence"));
        assert!(help.contains("Print the local authority reconciliation report"));
        assert!(help.contains("Print the local authority dry-run receipt"));
        assert!(
            help.contains("A successful command means the local authority artifact was produced")
        );
        assert!(help.contains("not that the deployment is globally safe"));
        assert!(help.contains("controller state"));
        assert!(help.contains("was changed"));
        assert!(!help.contains("authority reconciliation plan JSON"));
        assert!(!help.contains("authority dry-run evidence JSON"));
        assert!(!help.contains("authority reconciliation report JSON"));
        assert!(!help.contains("authority dry-run receipt JSON"));
    }

    #[test]
    fn deploy_promote_leaf_commands_default_to_json() {
        for (command, usage, request) in promote_leaf_commands() {
            let options = DeployPromoteReportOptions::parse(
                [OsString::from("--request"), OsString::from(request)],
                command,
                usage,
            )
            .expect("parse promote leaf command");

            assert_eq!(options.request, PathBuf::from(request));
            assert_eq!(options.format, PromotionOutputFormat::Json);
        }
    }

    #[test]
    fn deploy_promote_leaf_commands_parse_text_format() {
        for (command, usage, request) in promote_leaf_commands() {
            let options = DeployPromoteReportOptions::parse(
                [
                    OsString::from("--request"),
                    OsString::from(request),
                    OsString::from("--format"),
                    OsString::from("text"),
                ],
                command,
                usage,
            )
            .expect("parse promote leaf command text");

            assert_eq!(options.request, PathBuf::from(request));
            assert_eq!(options.format, PromotionOutputFormat::Text);
        }
    }

    #[test]
    fn deploy_promote_help_documents_passive_scope() {
        let help = promote_usage();
        let readiness_help = promote_readiness_usage();
        let check_help = promote_check_usage();
        let artifact_identity_help = promote_artifact_identity_usage();
        let transform_help = promote_transform_usage();
        let diff_help = promote_diff_usage();
        let transform_evidence_help = promote_transform_evidence_usage();
        let target_lineage_help = promote_target_lineage_usage();
        let plan_help = promote_plan_usage();
        let provenance_help = promote_provenance_usage();
        let wasm_store_identity_help = promote_wasm_store_identity_usage();
        let catalog_verification_help = promote_catalog_verification_usage();
        let execution_receipt_help = promote_execution_receipt_usage();
        let policy_help = promote_policy_check_usage();
        let materialization_help = promote_materialization_identity_usage();

        assert!(help.contains("Build passive artifact promotion reports"));
        assert!(help.contains("Build a passive artifact promotion readiness check"));
        assert!(help.contains("Build a passive artifact promotion diff"));
        assert!(help.contains("do not install"));
        assert!(help.contains("mutate deployment/controller state"));
        assert!(readiness_help.contains("PromotionReadinessRequest-shaped JSON"));
        assert!(check_help.contains("PromotionReadinessRequest-shaped JSON"));
        assert!(
            artifact_identity_help.contains("PromotionArtifactIdentityReportRequest-shaped JSON")
        );
        assert!(transform_help.contains("PromotionPlanTransformRequest-shaped JSON"));
        assert!(diff_help.contains("PromotionPlanTransformRequest-shaped JSON"));
        assert!(
            transform_evidence_help.contains("PromotionPlanTransformEvidenceRequest-shaped JSON")
        );
        assert!(target_lineage_help.contains("PromotionTargetExecutionLineageRequest-shaped JSON"));
        assert!(plan_help.contains("ArtifactPromotionPlanRequest-shaped JSON"));
        assert!(provenance_help.contains("ArtifactPromotionProvenanceReportRequest-shaped JSON"));
        assert!(
            wasm_store_identity_help
                .contains("PromotionWasmStoreIdentityReportRequest-shaped JSON")
        );
        assert!(
            catalog_verification_help
                .contains("PromotionWasmStoreCatalogVerificationRequest-shaped JSON")
        );
        assert!(
            execution_receipt_help.contains("ArtifactPromotionExecutionReceiptRequest-shaped JSON")
        );
        assert!(policy_help.contains("PromotionPolicyCheckRequest-shaped JSON"));
        assert!(
            materialization_help
                .contains("PromotionMaterializationIdentityReportRequest-shaped JSON")
        );
    }

    #[test]
    fn deploy_authority_leaf_help_documents_exit_status_scope() {
        let report_help = authority_report_usage();
        let receipt_help = authority_receipt_usage();
        let evidence_help = authority_evidence_usage();

        assert!(report_help.contains("Authority status is authority-scoped"));
        assert!(report_help.contains("whole-deployment safety"));
        assert!(receipt_help.contains("zero attempted"));
        assert!(receipt_help.contains("actions."));
        assert!(evidence_help.contains("evidence generation succeeded"));
    }

    #[test]
    fn deploy_authority_path_has_no_controller_mutation_primitives() {
        let source = include_str!("mod.rs");
        let authority_source = source_between(source, "fn run_authority<I>", "fn run_plan<I>");
        for forbidden in [
            "update_settings",
            "install_code",
            "create_canister",
            "delete_canister",
            "stop_canister",
            "uninstall_code",
            "provisional_create_canister",
            "dfx",
        ] {
            assert!(
                !authority_source.contains(forbidden),
                "authority CLI path must stay dry-run; found forbidden token {forbidden}"
            );
        }
    }

    #[test]
    fn deploy_promote_path_has_no_mutation_primitives() {
        let source = include_str!("mod.rs");
        let promote_source = source_between(source, "fn run_promote<I>", "fn run_authority<I>");
        for forbidden in [
            "update_settings",
            "install_code",
            "create_canister",
            "delete_canister",
            "stop_canister",
            "uninstall_code",
            "provisional_create_canister",
            "dfx",
        ] {
            assert!(
                !promote_source.contains(forbidden),
                "promote CLI path must stay passive; found forbidden token {forbidden}"
            );
        }
    }

    #[test]
    fn deploy_promote_path_has_no_live_deployment_truth_dependencies() {
        let source = include_str!("mod.rs");
        let promote_source = source_between(source, "fn run_promote<I>", "fn run_authority<I>");
        for forbidden in [
            "load_deployment_check",
            "check_install_deployment_truth",
            "resolve_current_canic_icp_root",
            "latest_deployment_truth_receipt_path_from_root",
        ] {
            assert!(
                !promote_source.contains(forbidden),
                "promote CLI path must stay request-file based; found forbidden token {forbidden}"
            );
        }
    }

    #[test]
    fn deploy_authority_command_dispatches_check() {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("authority"),
                OsString::from("check"),
                OsString::from("demo"),
            ],
        )
        .expect("parse deploy authority")
        .expect("authority command");

        assert_eq!(parsed.0, "authority");
        assert_eq!(
            parsed.1,
            vec![OsString::from("check"), OsString::from("demo")]
        );

        let nested = parse_subcommand(deploy_authority_command(), parsed.1)
            .expect("parse nested authority")
            .expect("authority check command");
        assert_eq!(nested.0, "check");
        assert_eq!(nested.1, vec![OsString::from("demo")]);
    }

    #[test]
    fn deploy_authority_command_dispatches_evidence() {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("authority"),
                OsString::from("evidence"),
                OsString::from("demo"),
            ],
        )
        .expect("parse deploy authority")
        .expect("authority command");

        assert_eq!(parsed.0, "authority");
        assert_eq!(
            parsed.1,
            vec![OsString::from("evidence"), OsString::from("demo")]
        );

        let nested = parse_subcommand(deploy_authority_command(), parsed.1)
            .expect("parse nested authority")
            .expect("authority evidence command");
        assert_eq!(nested.0, "evidence");
        assert_eq!(nested.1, vec![OsString::from("demo")]);
    }

    #[test]
    fn deploy_authority_command_dispatches_report() {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("authority"),
                OsString::from("report"),
                OsString::from("demo"),
            ],
        )
        .expect("parse deploy authority")
        .expect("authority command");

        assert_eq!(parsed.0, "authority");
        assert_eq!(
            parsed.1,
            vec![OsString::from("report"), OsString::from("demo")]
        );

        let nested = parse_subcommand(deploy_authority_command(), parsed.1)
            .expect("parse nested authority")
            .expect("authority report command");
        assert_eq!(nested.0, "report");
        assert_eq!(nested.1, vec![OsString::from("demo")]);
    }

    #[test]
    fn deploy_authority_command_dispatches_receipt() {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("authority"),
                OsString::from("receipt"),
                OsString::from("demo"),
            ],
        )
        .expect("parse deploy authority")
        .expect("authority command");

        assert_eq!(parsed.0, "authority");
        assert_eq!(
            parsed.1,
            vec![OsString::from("receipt"), OsString::from("demo")]
        );

        let nested = parse_subcommand(deploy_authority_command(), parsed.1)
            .expect("parse nested authority")
            .expect("authority receipt command");
        assert_eq!(nested.0, "receipt");
        assert_eq!(nested.1, vec![OsString::from("demo")]);
    }

    #[test]
    fn deploy_promote_command_dispatches_plan_as_public_surface() {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("promote"),
                OsString::from("plan"),
                OsString::from("--request"),
                OsString::from("promotion-plan.json"),
            ],
        )
        .expect("parse deploy promote")
        .expect("promote command");

        assert_eq!(parsed.0, "promote");

        let nested = parse_subcommand(deploy_promote_command(), parsed.1)
            .expect("parse nested promote")
            .expect("promote plan command");
        assert_eq!(nested.0, "plan");
        assert_eq!(
            nested.1,
            vec![
                OsString::from("--request"),
                OsString::from("promotion-plan.json")
            ]
        );
    }

    #[test]
    fn deploy_promote_command_dispatches_check_and_diff_as_public_surface() {
        for (command, request) in [
            ("check", "promotion-check.json"),
            ("diff", "promotion-diff.json"),
        ] {
            let parsed = parse_subcommand(
                deploy_command(),
                [
                    OsString::from("promote"),
                    OsString::from(command),
                    OsString::from("--request"),
                    OsString::from(request),
                ],
            )
            .expect("parse deploy promote")
            .expect("promote command");

            assert_eq!(parsed.0, "promote");

            let nested = parse_subcommand(deploy_promote_command(), parsed.1)
                .expect("parse nested promote")
                .expect("promote public command");
            assert_eq!(nested.0, command);
            assert_eq!(
                nested.1,
                vec![OsString::from("--request"), OsString::from(request)]
            );
        }
    }

    #[test]
    fn deploy_promote_command_dispatches_inspect_namespace() {
        let parsed = parse_subcommand(
            deploy_command(),
            [OsString::from("promote"), OsString::from("inspect")],
        )
        .expect("parse deploy promote")
        .expect("promote command");

        assert_eq!(parsed.0, "promote");

        let nested = parse_subcommand(deploy_promote_command(), parsed.1)
            .expect("parse nested promote")
            .expect("promote inspect command");
        assert_eq!(nested.0, "inspect");
        assert!(nested.1.is_empty());
    }

    #[test]
    fn deploy_promote_inspect_dispatches_leaf_commands() {
        for (_, _, command, request) in promote_inspect_leaf_commands() {
            let parsed = parse_subcommand(
                deploy_command(),
                [
                    OsString::from("promote"),
                    OsString::from("inspect"),
                    OsString::from(command),
                    OsString::from("--request"),
                    OsString::from(request),
                ],
            )
            .expect("parse deploy promote inspect")
            .expect("promote command");

            assert_eq!(parsed.0, "promote");

            let promote = parse_subcommand(deploy_promote_command(), parsed.1)
                .expect("parse nested promote")
                .expect("promote inspect command");
            assert_eq!(promote.0, "inspect");

            let inspect = parse_subcommand(deploy_promote_inspect_command(), promote.1)
                .expect("parse nested inspect")
                .expect("promote inspect leaf command");
            assert_eq!(inspect.0, command);
            assert_eq!(
                inspect.1,
                vec![OsString::from("--request"), OsString::from(request)]
            );
        }
    }

    #[test]
    fn authority_evidence_builder_delegates_to_host_local_ids() {
        let check = sample_authority_check();

        let evidence =
            build_authority_dry_run_evidence(&check).expect("build authority dry-run evidence");

        assert_eq!(evidence.evidence_id, "local:local:demo:authority-evidence");
        assert_eq!(evidence.check_id, "check-1");
        assert_eq!(
            evidence.authority_report.check_id.as_deref(),
            Some("check-1")
        );
        assert_eq!(
            evidence.authority_report.report_id,
            "local:local:demo:authority-report"
        );
        assert_eq!(evidence.authority_report.inventory_id, "inventory-1");
        assert_eq!(
            evidence.authority_report.authority_profile_hash.as_deref(),
            Some("authority")
        );
        assert_eq!(
            evidence.authority_receipt.check_id.as_deref(),
            Some("check-1")
        );
        assert_eq!(
            evidence.authority_receipt.operation_id,
            "local:local:demo:authority-dry-run-receipt"
        );
        assert_eq!(evidence.authority_receipt.inventory_id, "inventory-1");
        assert_eq!(
            evidence.authority_receipt.authority_profile_hash.as_deref(),
            Some("authority")
        );
    }

    #[test]
    fn authority_receipt_builder_delegates_to_host_local_ids() {
        let check = sample_authority_check();

        let receipt =
            build_authority_dry_run_receipt(&check).expect("build authority dry-run receipt");

        assert_eq!(
            receipt.operation_id,
            "local:local:demo:authority-dry-run-receipt"
        );
        assert_eq!(receipt.check_id.as_deref(), Some("check-1"));
        assert_eq!(receipt.reconciliation_plan_id, "plan-1");
        assert_eq!(
            receipt.authority_report_id,
            "local:local:demo:authority-report"
        );
        assert_eq!(receipt.inventory_id, "inventory-1");
        assert_eq!(receipt.authority_profile_hash.as_deref(), Some("authority"));
        assert!(receipt.attempted_actions.is_empty());
    }

    #[test]
    fn authority_check_rejects_unknown_format() {
        let result = DeployAuthorityOptions::parse(
            [
                OsString::from("--format"),
                OsString::from("csv"),
                OsString::from("demo"),
            ],
            deploy_authority_check_command,
            authority_check_usage,
        );

        assert!(matches!(
            result,
            Err(DeployCommandError::Usage(message))
                if message.contains("invalid authority output format: csv")
        ));
    }

    #[test]
    fn authority_evidence_rejects_unknown_format() {
        let result = DeployAuthorityOptions::parse(
            [
                OsString::from("--format"),
                OsString::from("xml"),
                OsString::from("demo"),
            ],
            deploy_authority_evidence_command,
            authority_evidence_usage,
        );

        assert!(matches!(
            result,
            Err(DeployCommandError::Usage(message))
                if message.contains("invalid authority output format: xml")
        ));
    }

    #[test]
    fn authority_report_rejects_unknown_format() {
        let result = DeployAuthorityOptions::parse(
            [
                OsString::from("--format"),
                OsString::from("yaml"),
                OsString::from("demo"),
            ],
            deploy_authority_report_command,
            authority_report_usage,
        );

        assert!(matches!(
            result,
            Err(DeployCommandError::Usage(message))
                if message.contains("invalid authority output format: yaml")
        ));
    }

    #[test]
    fn authority_receipt_rejects_unknown_format() {
        let result = DeployAuthorityOptions::parse(
            [
                OsString::from("--format"),
                OsString::from("toml"),
                OsString::from("demo"),
            ],
            deploy_authority_receipt_command,
            authority_receipt_usage,
        );

        assert!(matches!(
            result,
            Err(DeployCommandError::Usage(message))
                if message.contains("invalid authority output format: toml")
        ));
    }

    #[test]
    fn promote_policy_check_rejects_unknown_format() {
        let result = DeployPromoteReportOptions::parse(
            [
                OsString::from("--request"),
                OsString::from("promotion-policy.json"),
                OsString::from("--format"),
                OsString::from("csv"),
            ],
            deploy_promote_policy_check_command,
            promote_policy_check_usage,
        );

        assert!(matches!(
            result,
            Err(DeployCommandError::Usage(message))
                if message.contains("invalid promotion output format: csv")
        ));
    }

    type PromoteCommandFactory = fn() -> ClapCommand;
    type PromoteUsageFactory = fn() -> String;

    fn promote_leaf_commands() -> [(PromoteCommandFactory, PromoteUsageFactory, &'static str); 14] {
        [
            (
                deploy_promote_readiness_command,
                promote_readiness_usage,
                "promotion-readiness.json",
            ),
            (
                deploy_promote_check_command,
                promote_check_usage,
                "promotion-check.json",
            ),
            (
                deploy_promote_artifact_identity_command,
                promote_artifact_identity_usage,
                "promotion-artifacts.json",
            ),
            (
                deploy_promote_transform_command,
                promote_transform_usage,
                "promotion-transform.json",
            ),
            (
                deploy_promote_diff_command,
                promote_diff_usage,
                "promotion-diff.json",
            ),
            (
                deploy_promote_transform_evidence_command,
                promote_transform_evidence_usage,
                "transform-evidence.json",
            ),
            (
                deploy_promote_target_lineage_command,
                promote_target_lineage_usage,
                "target-lineage.json",
            ),
            (
                deploy_promote_plan_command,
                promote_plan_usage,
                "promotion-plan.json",
            ),
            (
                deploy_promote_provenance_command,
                promote_provenance_usage,
                "promotion-provenance.json",
            ),
            (
                deploy_promote_wasm_store_identity_command,
                promote_wasm_store_identity_usage,
                "wasm-store-identity.json",
            ),
            (
                deploy_promote_catalog_verification_command,
                promote_catalog_verification_usage,
                "catalog-verification.json",
            ),
            (
                deploy_promote_execution_receipt_command,
                promote_execution_receipt_usage,
                "promotion-execution-receipt.json",
            ),
            (
                deploy_promote_policy_check_command,
                promote_policy_check_usage,
                "promotion-policy.json",
            ),
            (
                deploy_promote_materialization_identity_command,
                promote_materialization_identity_usage,
                "materialization.json",
            ),
        ]
    }

    fn promote_inspect_leaf_commands() -> [(
        PromoteCommandFactory,
        PromoteUsageFactory,
        &'static str,
        &'static str,
    ); 11] {
        [
            (
                deploy_promote_readiness_command,
                promote_readiness_usage,
                "readiness",
                "promotion-readiness.json",
            ),
            (
                deploy_promote_artifact_identity_command,
                promote_artifact_identity_usage,
                "artifact-identity",
                "promotion-artifacts.json",
            ),
            (
                deploy_promote_transform_command,
                promote_transform_usage,
                "transform",
                "promotion-transform.json",
            ),
            (
                deploy_promote_transform_evidence_command,
                promote_transform_evidence_usage,
                "transform-evidence",
                "transform-evidence.json",
            ),
            (
                deploy_promote_target_lineage_command,
                promote_target_lineage_usage,
                "target-lineage",
                "target-lineage.json",
            ),
            (
                deploy_promote_provenance_command,
                promote_provenance_usage,
                "provenance",
                "promotion-provenance.json",
            ),
            (
                deploy_promote_wasm_store_identity_command,
                promote_wasm_store_identity_usage,
                "wasm-store-identity",
                "wasm-store-identity.json",
            ),
            (
                deploy_promote_catalog_verification_command,
                promote_catalog_verification_usage,
                "catalog-verification",
                "catalog-verification.json",
            ),
            (
                deploy_promote_execution_receipt_command,
                promote_execution_receipt_usage,
                "execution-receipt",
                "promotion-execution-receipt.json",
            ),
            (
                deploy_promote_policy_check_command,
                promote_policy_check_usage,
                "policy",
                "promotion-policy.json",
            ),
            (
                deploy_promote_materialization_identity_command,
                promote_materialization_identity_usage,
                "materialization-identity",
                "materialization.json",
            ),
        ]
    }

    fn source_between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
        let start_index = source.find(start).expect("source start marker");
        let rest = &source[start_index..];
        let end_index = rest.find(end).expect("source end marker");
        &rest[..end_index]
    }

    #[test]
    fn deploy_resume_report_allows_latest_local_receipt_lookup() {
        let resume_report = DeployResumeReportOptions::parse([OsString::from("demo")])
            .expect("parse deploy resume-report");

        assert_eq!(resume_report.truth.fleet, "demo");
        assert_eq!(resume_report.receipt, None);
    }

    #[test]
    fn deploy_check_builds_current_install_options() {
        let options = DeployTruthOptions {
            fleet: "demo".to_string(),
            network: "local".to_string(),
            profile: Some(CanisterBuildProfile::Fast),
        }
        .into_install_root_options_with_icp_root(Some(std::path::PathBuf::from("/tmp/icp")));

        assert_eq!(options.root_canister, "root");
        assert_eq!(options.root_build_target, "root");
        assert_eq!(options.network, "local");
        assert_eq!(options.build_profile, Some(CanisterBuildProfile::Fast));
        assert_eq!(
            options.config_path.as_deref(),
            Some("fleets/demo/canic.toml")
        );
        assert_eq!(options.expected_fleet.as_deref(), Some("demo"));
    }

    fn sample_authority_check() -> DeploymentCheckV1 {
        let identity = sample_deployment_identity();
        let plan = sample_deployment_plan(identity.clone());
        let inventory = sample_deployment_inventory(identity);
        let diff = sample_deployment_diff(&plan, &inventory);
        let report = sample_safety_report();

        DeploymentCheckV1 {
            schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            check_id: "check-1".to_string(),
            plan,
            inventory,
            diff,
            report,
        }
    }

    fn sample_deployment_identity() -> DeploymentIdentityV1 {
        DeploymentIdentityV1 {
            deployment_name: "demo".to_string(),
            network: "local".to_string(),
            root_principal: Some("aaaaa-aa".to_string()),
            authority_profile_hash: Some("authority".to_string()),
            role_topology_hash: None,
            deployment_manifest_digest: None,
            canonical_runtime_config_digest: None,
            role_embedded_config_set_digest: None,
            artifact_set_digest: None,
            pool_identity_set_digest: None,
            canic_version: None,
            ic_memory_version: None,
        }
    }

    fn sample_deployment_plan(identity: DeploymentIdentityV1) -> DeploymentPlanV1 {
        DeploymentPlanV1 {
            schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            plan_id: "plan-1".to_string(),
            deployment_identity: identity,
            trust_domain: TrustDomainV1 {
                root_trust_anchor: Some("aaaaa-aa".to_string()),
                migration_from: None,
            },
            fleet_template: "demo".to_string(),
            runtime_variant: "local".to_string(),
            authority_profile: AuthorityProfileV1 {
                profile_id: "authority-profile-1".to_string(),
                expected_controllers: vec!["aaaaa-aa".to_string()],
                staging_controllers: Vec::new(),
                emergency_controllers: Vec::new(),
            },
            role_artifacts: Vec::new(),
            expected_canisters: vec![ExpectedCanisterV1 {
                role: "root".to_string(),
                canister_id: Some("aaaaa-aa".to_string()),
                control_class: CanisterControlClassV1::DeploymentControlled,
            }],
            expected_pool: Vec::new(),
            expected_verifier_readiness: VerifierReadinessExpectationV1 {
                required: false,
                expected_role_epochs: Vec::new(),
            },
            unresolved_assumptions: Vec::new(),
        }
    }

    fn sample_deployment_inventory(identity: DeploymentIdentityV1) -> DeploymentInventoryV1 {
        DeploymentInventoryV1 {
            schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            inventory_id: "inventory-1".to_string(),
            observed_at: "2026-05-23T00:00:00Z".to_string(),
            observed_identity: Some(identity),
            local_config: LocalDeploymentConfigV1 {
                config_path: None,
                raw_config_sha256: None,
                canonical_embedded_config_sha256: None,
            },
            observed_canisters: vec![ObservedCanisterV1 {
                canister_id: "aaaaa-aa".to_string(),
                role: Some("root".to_string()),
                control_class: CanisterControlClassV1::DeploymentControlled,
                controllers: vec!["aaaaa-aa".to_string()],
                module_hash: None,
                status: Some("running".to_string()),
                root_trust_anchor: Some("aaaaa-aa".to_string()),
                canonical_embedded_config_digest: None,
                role_assignment_source: Some("test".to_string()),
            }],
            observed_pool: Vec::new(),
            observed_artifacts: Vec::new(),
            observed_verifier_readiness: VerifierReadinessObservationV1 {
                status: ObservationStatusV1::NotObserved,
                role_epochs: Vec::new(),
            },
            unresolved_observations: Vec::new(),
        }
    }

    fn sample_deployment_diff(
        plan: &DeploymentPlanV1,
        inventory: &DeploymentInventoryV1,
    ) -> DeploymentDiffV1 {
        DeploymentDiffV1 {
            schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            plan_identity: plan.deployment_identity.clone(),
            observed_identity: inventory.observed_identity.clone(),
            artifact_diff: Vec::new(),
            controller_diff: Vec::new(),
            pool_diff: Vec::new(),
            embedded_config_diff: Vec::new(),
            module_hash_diff: Vec::new(),
            verifier_readiness_diff: Vec::new(),
            resume_safety: ResumeSafetyV1 {
                status: SafetyStatusV1::Safe,
                reasons: vec!["safe".to_string()],
            },
            hard_failures: Vec::new(),
            warnings: Vec::new(),
            resumable_phases: Vec::new(),
        }
    }

    fn sample_safety_report() -> SafetyReportV1 {
        SafetyReportV1 {
            schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            report_id: "safety-report-1".to_string(),
            diff_id: None,
            status: SafetyStatusV1::Safe,
            summary: "safe".to_string(),
            hard_failures: Vec::new(),
            warnings: Vec::new(),
            next_actions: Vec::new(),
        }
    }
}
