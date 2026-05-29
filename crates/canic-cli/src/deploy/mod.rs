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
        AuthorityDryRunEvidenceV1, BuildMaterializationEvidenceV1, CriticalExternalFixReportV1,
        DeploymentCheckV1, DeploymentComparisonReportV1, DeploymentExecutionPreflightV1,
        DeploymentPlanV1, DeploymentReceiptV1, DeploymentRootVerificationReportV1,
        DeploymentRootVerificationRequestV1, ExternalLifecycleCheckV1, ExternalLifecycleHandoffV1,
        ExternalLifecyclePendingReportV1, ExternalLifecyclePlanV1,
        ExternalUpgradeCompletionReportRequest, ExternalUpgradeCompletionReportV1,
        ExternalUpgradeConsentEvidenceRequest, ExternalUpgradeConsentEvidenceV1,
        ExternalUpgradeProposalReportV1, ExternalUpgradeVerificationCheckRequest,
        ExternalUpgradeVerificationCheckV1, ExternalUpgradeVerificationPolicyRequest,
        ExternalUpgradeVerificationPolicyV1, ExternalUpgradeVerificationReportRequest,
        ExternalUpgradeVerificationReportV1, PromotionArtifactIdentityReportRequest,
        PromotionArtifactIdentityReportV1, PromotionMaterializationIdentityReportRequest,
        PromotionMaterializationIdentityReportV1, PromotionPlanTransformEvidenceRequest,
        PromotionPlanTransformEvidenceV1, PromotionPlanTransformRequest, PromotionPlanTransformV1,
        PromotionPlanTransformWithMaterializationRequest, PromotionPolicyCheckRequest,
        PromotionPolicyCheckV1, PromotionReadinessRequest, PromotionReadinessStatusV1,
        PromotionReadinessV1, PromotionTargetExecutionLineageRequest,
        PromotionTargetExecutionLineageV1, PromotionWasmStoreCatalogEntryV1,
        PromotionWasmStoreCatalogVerificationRequest, PromotionWasmStoreCatalogVerificationV1,
        PromotionWasmStoreIdentityReportRequest, PromotionWasmStoreIdentityReportV1,
        RolePromotionInputV1, RolePromotionPolicyV1, SafetyReportV1, SafetyStatusV1,
        StagingReceiptV1, artifact_promotion_execution_receipt,
        artifact_promotion_execution_receipt_text, artifact_promotion_plan,
        artifact_promotion_plan_text, artifact_promotion_provenance_report,
        artifact_promotion_provenance_report_text,
        authority_dry_run_evidence_from_check_with_local_ids,
        authority_dry_run_receipt_from_check_with_local_id, authority_evidence_text,
        authority_plan_text, authority_receipt_text, authority_report_from_check_with_local_id,
        authority_report_text, build_authority_reconciliation_plan, check_promotion_policy,
        check_promotion_readiness, compare_plan_inventory_and_receipt,
        critical_external_fix_report_from_pending, critical_external_fix_report_text,
        deployment_comparison_report_from_checks, deployment_comparison_report_text,
        deployment_root_verification_receipt_text, deployment_root_verification_report_from_check,
        deployment_root_verification_report_text, external_lifecycle_check_from_reports,
        external_lifecycle_check_text, external_lifecycle_handoff_from_reports,
        external_lifecycle_handoff_text, external_lifecycle_pending_report_from_plan,
        external_lifecycle_pending_report_text, external_lifecycle_plan_from_check,
        external_lifecycle_plan_text, external_upgrade_completion_report_from_evidence,
        external_upgrade_completion_report_text, external_upgrade_consent_evidence_from_receipt,
        external_upgrade_consent_evidence_text,
        external_upgrade_proposal_report_from_lifecycle_plan,
        external_upgrade_proposal_report_text, external_upgrade_verification_check_from_policy,
        external_upgrade_verification_check_text,
        external_upgrade_verification_observation_from_check,
        external_upgrade_verification_policy_from_proposal,
        external_upgrade_verification_policy_text,
        external_upgrade_verification_report_from_receipt,
        external_upgrade_verification_report_text, promoted_deployment_plan_transform_from_inputs,
        promoted_deployment_plan_transform_from_inputs_with_materialization,
        promotion_artifact_identity_report_from_inputs, promotion_artifact_identity_report_text,
        promotion_materialization_identity_report_from_evidence,
        promotion_materialization_identity_report_text, promotion_plan_transform_evidence,
        promotion_plan_transform_evidence_text, promotion_plan_transform_text,
        promotion_policy_check_text, promotion_readiness_text, promotion_target_execution_lineage,
        promotion_target_execution_lineage_text, promotion_wasm_store_catalog_verification,
        promotion_wasm_store_catalog_verification_text,
        promotion_wasm_store_identity_report_from_staging,
        promotion_wasm_store_identity_report_text, validate_artifact_promotion_plan,
        validate_deployment_comparison_report, validate_deployment_root_verification_report,
        validate_external_upgrade_verification_check_for_deployment_check,
        validate_external_upgrade_verification_check_for_policy,
    },
    icp_config::resolve_current_canic_icp_root,
    install_root::{
        InstallRootOptions, RegisterDeploymentStateOptions, VerifyDeploymentRootOptions,
        check_install_deployment_truth, install_root,
        latest_deployment_truth_receipt_path_from_root, register_deployment_state,
        verify_registered_deployment_root,
    },
};
use clap::{ArgAction, Command as ClapCommand};
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

Deployment truth commands are read-only checks. Plan-mediated mutation flows
through `canic deploy install <deployment> --plan <file>` or the legacy `canic install`
entrypoint. Authority commands are dry-run reconciliation reports and do not
mutate controller state.";
const DEPLOY_ROOT_HELP_AFTER: &str = "\
Examples:
  canic deploy root inspect --request root-verification.json
  canic deploy root verify demo-local --from-check deployment-check.json
  canic deploy root inspect --request root-verification.json --format text

0.47 root commands are deployment-root scoped. Inspect builds passive
root-verification reports without writing state. Verify records verified root
state only when a registered deployment target and DeploymentCheckV1 source
evidence match.";
const DEPLOY_ROOT_INSPECT_HELP_AFTER: &str = "\
Examples:
  canic deploy root inspect --request root-verification.json
  canic deploy root inspect --request root-verification.json --format text

Reads a DeploymentRootVerificationRequestV1-shaped JSON file and prints a
DeploymentRootVerificationReportV1 JSON artifact by default, or host-owned
passive text with --format text. EvidenceSatisfied means the supplied
deployment-truth evidence is sufficient for a later explicit state transition;
this command does not persist verified root state.";
const DEPLOY_ROOT_VERIFY_HELP_AFTER: &str = "\
Examples:
  canic deploy root verify demo-local --from-check deployment-check.json
  canic deploy root verify demo-local --from-check deployment-check.json --format text

Verifies a registered deployment root from a deployment-truth check artifact
and records verified root state only when deployment target identity and source
evidence match. This is not full deployment verification and does not install
code or mutate canisters.";
const DEPLOY_COMPARE_HELP_AFTER: &str = "\
Examples:
  canic deploy compare --left staging-check.json --right prod-check.json
  canic deploy compare --left staging-check.json --right prod-check.json --format text

Compares two existing DeploymentCheckV1 JSON artifacts. It does not query live
state, install code, or mutate deployments. Each input check's embedded
diff/report is revalidated against its plan and inventory before comparison
status is rendered.";
const DEPLOY_REGISTER_HELP_AFTER: &str = "\
Examples:
  canic deploy register demo --fleet-template demo --root aaaaa-aa --allow-unverified
  canic --network local deploy register demo --fleet-template demo --root uxrrr-q7777-77774-qaaaq-cai --allow-unverified

Registers minimal deployment-target local state for an existing root canister.
This is an explicit 0.46 hard-cut recovery path. It does not migrate legacy
fleet-template state, query live inventory, copy receipts, record
artifact/controller truth, install code, or mutate canisters. Registered roots are marked
not_verified until a later verification path records live evidence. The
--allow-unverified flag is required so unverified registration remains an
explicit operator acknowledgement.";
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
const DEPLOY_INSTALL_HELP_AFTER: &str = "\
Examples:
  canic deploy install demo-local --plan promoted-plan.json
  canic --network local deploy install demo-local --plan promoted-plan.json --profile fast

Installs through the current install runner using a supplied DeploymentPlanV1
or ArtifactPromotionPlanV1. The deployment-truth/preflight gate runs before
mutation, and activation phases still execute through the current-install
operation runner.";
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
const DEPLOY_RESUME_REPORT_HELP_AFTER: &str = "\
Examples:
  canic deploy resume-report demo
  canic deploy resume-report --receipt receipt.json demo
  canic --network local deploy resume-report --receipt receipt.json --profile fast demo

Prints the passive ResumeSafetyV1 JSON for the current deployment truth check
and a prior DeploymentReceiptV1. When --receipt is omitted, Canic uses the
latest local receipt under .canic/<network>/deployment-receipts/<deployment>. It
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
    deployment: String,
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

///
/// DeployInstallPlanOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeployInstallPlanOptions {
    deployment: String,
    plan: PathBuf,
    network: String,
    profile: Option<CanisterBuildProfile>,
}

///
/// DeployRegisterOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeployRegisterOptions {
    deployment: String,
    fleet_template: String,
    root: String,
    network: String,
    allow_unverified: bool,
}

///
/// DeployInstallPlanInput
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeployInstallPlanInput {
    deployment_plan: DeploymentPlanV1,
    artifact_promotion_plan: Option<ArtifactPromotionPlanV1>,
}

///
/// DeployCompareOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeployCompareOptions {
    left: PathBuf,
    right: PathBuf,
    left_label: Option<String>,
    right_label: Option<String>,
    format: CompareOutputFormat,
}

/// DeployAuthorityOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeployAuthorityOptions {
    truth: DeployTruthOptions,
    format: AuthorityOutputFormat,
}

///
/// DeployExternalOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeployExternalOptions {
    truth: DeployTruthOptions,
    format: ExternalOutputFormat,
}

///
/// DeployExternalCriticalFixOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeployExternalCriticalFixOptions {
    truth: DeployTruthOptions,
    format: ExternalOutputFormat,
    fix_id: String,
    severity: String,
}

///
/// DeployExternalVerifyOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeployExternalVerifyOptions {
    request: PathBuf,
    format: ExternalOutputFormat,
}

///
/// DeployExternalInspectOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeployExternalInspectOptions {
    request: PathBuf,
    format: ExternalOutputFormat,
}

///
/// DeployRootInspectOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeployRootInspectOptions {
    request: PathBuf,
    format: RootOutputFormat,
}

///
/// DeployRootVerifyOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeployRootVerifyOptions {
    deployment: String,
    from_check: PathBuf,
    network: String,
    format: RootOutputFormat,
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
/// ExternalOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExternalOutputFormat {
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

///
/// CompareOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompareOutputFormat {
    Json,
    Text,
}

///
/// RootOutputFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RootOutputFormat {
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
            "external" => run_external(args),
            "promote" => run_promote(args),
            "root" => run_root(args),
            "install" => run_install(args),
            "register" => run_register(args),
            "compare" => run_compare(args),
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

fn run_root<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, root_usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(deploy_root_command(), args)
        .map_err(|_| DeployCommandError::Usage(root_usage()))?
    {
        Some((command, args)) if command == "inspect" => run_root_inspect(args),
        Some((command, args)) if command == "verify" => run_root_verify(args),
        _ => {
            println!("{}", root_usage());
            Ok(())
        }
    }
}

fn run_root_inspect<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, root_inspect_usage, version_text()) {
        return Ok(());
    }

    let options = DeployRootInspectOptions::parse(args)?;
    let request = read_json_file::<DeploymentRootVerificationRequestV1>(&options.request)?;
    let report = build_root_verification_report(request)?;
    match options.format {
        RootOutputFormat::Json => print_json(&report)?,
        RootOutputFormat::Text => println!("{}", deployment_root_verification_report_text(&report)),
    }
    Ok(())
}

fn run_root_verify<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, root_verify_usage, version_text()) {
        return Ok(());
    }

    let options = DeployRootVerifyOptions::parse(args)?;
    let check = read_json_file::<DeploymentCheckV1>(&options.from_check)?;
    let receipt = verify_registered_deployment_root(VerifyDeploymentRootOptions {
        deployment_name: options.deployment,
        network: options.network,
        deployment_check: check,
        verified_at_unix_secs: None,
        icp_root: resolve_current_canic_icp_root().ok(),
    })
    .map_err(DeployCommandError::from)?;
    match options.format {
        RootOutputFormat::Json => print_json(&receipt)?,
        RootOutputFormat::Text => {
            println!("{}", deployment_root_verification_receipt_text(&receipt));
        }
    }
    Ok(())
}

fn run_install<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, deploy_install_usage, version_text()) {
        return Ok(());
    }

    let options = DeployInstallPlanOptions::parse(args)?;
    let plan = read_install_deployment_plan(&options.plan)?;
    let icp_root = resolve_current_canic_icp_root().ok();
    install_root(options.into_install_root_options(plan, icp_root))
        .map_err(DeployCommandError::from)
}

fn run_register<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, deploy_register_usage, version_text()) {
        return Ok(());
    }

    let options = DeployRegisterOptions::parse(args)?;
    let state_path = register_deployment_state(options.into_register_options(None))
        .map_err(DeployCommandError::from)?;
    println!("Registered deployment state: {}", state_path.display());
    println!("root_verification: not_verified");
    Ok(())
}

fn run_compare<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, compare_usage, version_text()) {
        return Ok(());
    }

    let options = DeployCompareOptions::parse(args)?;
    let format = options.format;
    let report = build_deployment_comparison_report(options)?;
    match format {
        CompareOutputFormat::Json => print_json(&report)?,
        CompareOutputFormat::Text => println!("{}", deployment_comparison_report_text(&report)),
    }
    Ok(())
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

fn build_root_verification_report(
    request: DeploymentRootVerificationRequestV1,
) -> Result<DeploymentRootVerificationReportV1, DeployCommandError> {
    let report = deployment_root_verification_report_from_check(request);
    validate_deployment_root_verification_report(&report)
        .map_err(|err| DeployCommandError::Check(Box::new(err)))?;
    Ok(report)
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

fn run_external<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, external_usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(deploy_external_command(), args)
        .map_err(|_| DeployCommandError::Usage(external_usage()))?
    {
        Some((command, args)) if command == "plan" => run_external_plan(args),
        Some((command, args)) if command == "check" => run_external_check(args),
        Some((command, args)) if command == "handoff" => run_external_handoff(args),
        Some((command, args)) if command == "proposals" => run_external_proposals(args),
        Some((command, args)) if command == "pending" => run_external_pending(args),
        Some((command, args)) if command == "critical-fix" => run_external_critical_fix(args),
        Some((command, args)) if command == "inspect" => run_external_inspect(args),
        Some((command, args)) if command == "verify" => run_external_verify(args),
        _ => {
            println!("{}", external_usage());
            Ok(())
        }
    }
}

fn run_external_inspect<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, external_inspect_usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(deploy_external_inspect_command(), args)
        .map_err(|_| DeployCommandError::Usage(external_inspect_usage()))?
    {
        Some((command, args)) if command == "consent" => run_external_inspect_consent(args),
        Some((command, args)) if command == "verification-policy" => {
            run_external_inspect_verification_policy(args)
        }
        Some((command, args)) if command == "verification-check" => {
            run_external_inspect_verification_check(args)
        }
        Some((command, args)) if command == "completion" => run_external_inspect_completion(args),
        _ => {
            println!("{}", external_inspect_usage());
            Ok(())
        }
    }
}

fn run_external_plan<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_external_output(
        args,
        deploy_external_plan_command,
        external_plan_usage,
        build_external_lifecycle_plan,
        external_lifecycle_plan_text,
    )
}

fn run_external_check<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_external_output(
        args,
        deploy_external_check_command,
        external_check_usage,
        build_external_lifecycle_check,
        external_lifecycle_check_text,
    )
}

fn run_external_handoff<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_external_output(
        args,
        deploy_external_handoff_command,
        external_handoff_usage,
        build_external_lifecycle_handoff,
        external_lifecycle_handoff_text,
    )
}

fn run_external_proposals<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_external_output(
        args,
        deploy_external_proposals_command,
        external_proposals_usage,
        build_external_upgrade_proposal_report,
        external_upgrade_proposal_report_text,
    )
}

fn run_external_pending<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    run_external_output(
        args,
        deploy_external_pending_command,
        external_pending_usage,
        build_external_lifecycle_pending_report,
        external_lifecycle_pending_report_text,
    )
}

fn run_external_critical_fix<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, external_critical_fix_usage, version_text()) {
        return Ok(());
    }

    let options = DeployExternalCriticalFixOptions::parse(
        args,
        deploy_external_critical_fix_command,
        external_critical_fix_usage,
    )?;
    let check = load_deployment_check(options.truth)?;
    let report = build_critical_external_fix_report(
        &check,
        options.fix_id.as_str(),
        options.severity.as_str(),
    );
    match options.format {
        ExternalOutputFormat::Json => print_json(&report)?,
        ExternalOutputFormat::Text => println!("{}", critical_external_fix_report_text(&report)),
    }
    Ok(())
}

fn run_external_inspect_consent<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, external_consent_usage, version_text()) {
        return Ok(());
    }

    let options = DeployExternalInspectOptions::parse(
        args,
        deploy_external_inspect_consent_command,
        external_consent_usage,
    )?;
    let request = read_json_file::<ExternalUpgradeConsentEvidenceRequest>(&options.request)?;
    let evidence = build_external_upgrade_consent_evidence(request)?;
    match options.format {
        ExternalOutputFormat::Json => print_json(&evidence)?,
        ExternalOutputFormat::Text => {
            println!("{}", external_upgrade_consent_evidence_text(&evidence));
        }
    }
    Ok(())
}

fn run_external_inspect_verification_policy<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, external_verification_policy_usage, version_text()) {
        return Ok(());
    }

    let options = DeployExternalInspectOptions::parse(
        args,
        deploy_external_inspect_verification_policy_command,
        external_verification_policy_usage,
    )?;
    let request = read_json_file::<ExternalUpgradeVerificationPolicyRequest>(&options.request)?;
    let policy = build_external_upgrade_verification_policy(request);
    match options.format {
        ExternalOutputFormat::Json => print_json(&policy)?,
        ExternalOutputFormat::Text => {
            println!("{}", external_upgrade_verification_policy_text(&policy));
        }
    }
    Ok(())
}

fn run_external_inspect_verification_check<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, external_verification_check_usage, version_text()) {
        return Ok(());
    }

    let options = DeployExternalInspectOptions::parse(
        args,
        deploy_external_inspect_verification_check_command,
        external_verification_check_usage,
    )?;
    let request = read_json_file::<ExternalUpgradeVerificationCheckRequest>(&options.request)?;
    let check = build_external_upgrade_verification_check(request)?;
    match options.format {
        ExternalOutputFormat::Json => print_json(&check)?,
        ExternalOutputFormat::Text => {
            println!("{}", external_upgrade_verification_check_text(&check));
        }
    }
    Ok(())
}

fn run_external_inspect_completion<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, external_completion_usage, version_text()) {
        return Ok(());
    }

    let options = DeployExternalInspectOptions::parse(
        args,
        deploy_external_inspect_completion_command,
        external_completion_usage,
    )?;
    let request = read_json_file::<ExternalUpgradeCompletionReportRequest>(&options.request)?;
    let report = build_external_upgrade_completion_report(request)?;
    match options.format {
        ExternalOutputFormat::Json => print_json(&report)?,
        ExternalOutputFormat::Text => {
            println!("{}", external_upgrade_completion_report_text(&report));
        }
    }
    Ok(())
}

fn run_external_verify<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, external_verify_usage, version_text()) {
        return Ok(());
    }

    let options = DeployExternalVerifyOptions::parse(
        args,
        deploy_external_verify_command,
        external_verify_usage,
    )?;
    let request = read_json_file::<ExternalUpgradeVerificationReportRequest>(&options.request)?;
    let report = build_external_upgrade_verification_report(request)?;
    match options.format {
        ExternalOutputFormat::Json => print_json(&report)?,
        ExternalOutputFormat::Text => {
            println!("{}", external_upgrade_verification_report_text(&report));
        }
    }
    Ok(())
}

fn run_external_output<I, T>(
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

fn build_external_lifecycle_plan(check: &DeploymentCheckV1) -> ExternalLifecyclePlanV1 {
    external_lifecycle_plan_from_check(
        local_external_lifecycle_plan_id(check),
        local_lifecycle_authority_report_id(check),
        check,
    )
}

fn build_external_upgrade_proposal_report(
    check: &DeploymentCheckV1,
) -> ExternalUpgradeProposalReportV1 {
    let lifecycle_plan = build_external_lifecycle_plan(check);
    external_upgrade_proposal_report_from_lifecycle_plan(
        local_external_proposal_report_id(check),
        &lifecycle_plan,
        check,
    )
}

fn build_external_lifecycle_pending_report(
    check: &DeploymentCheckV1,
) -> ExternalLifecyclePendingReportV1 {
    let lifecycle_plan = build_external_lifecycle_plan(check);
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        local_external_proposal_report_id(check),
        &lifecycle_plan,
        check,
    );
    external_lifecycle_pending_report_from_plan(
        local_external_pending_report_id(check),
        &lifecycle_plan,
        &proposal_report,
    )
}

fn build_external_lifecycle_check(check: &DeploymentCheckV1) -> ExternalLifecycleCheckV1 {
    let lifecycle_plan = build_external_lifecycle_plan(check);
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        local_external_proposal_report_id(check),
        &lifecycle_plan,
        check,
    );
    let pending_report = external_lifecycle_pending_report_from_plan(
        local_external_pending_report_id(check),
        &lifecycle_plan,
        &proposal_report,
    );
    external_lifecycle_check_from_reports(
        local_external_check_id(check),
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    )
}

fn build_external_lifecycle_handoff(check: &DeploymentCheckV1) -> ExternalLifecycleHandoffV1 {
    let lifecycle_plan = build_external_lifecycle_plan(check);
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        local_external_proposal_report_id(check),
        &lifecycle_plan,
        check,
    );
    let pending_report = external_lifecycle_pending_report_from_plan(
        local_external_pending_report_id(check),
        &lifecycle_plan,
        &proposal_report,
    );
    let lifecycle_check = external_lifecycle_check_from_reports(
        local_external_check_id(check),
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    );
    external_lifecycle_handoff_from_reports(
        local_external_handoff_id(check),
        &lifecycle_check,
        &proposal_report,
        &pending_report,
    )
}

fn build_critical_external_fix_report(
    check: &DeploymentCheckV1,
    fix_id: &str,
    severity: &str,
) -> CriticalExternalFixReportV1 {
    let lifecycle_plan = build_external_lifecycle_plan(check);
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        local_external_proposal_report_id(check),
        &lifecycle_plan,
        check,
    );
    let pending_report = external_lifecycle_pending_report_from_plan(
        local_external_pending_report_id(check),
        &lifecycle_plan,
        &proposal_report,
    );
    critical_external_fix_report_from_pending(
        local_external_critical_fix_report_id(check),
        fix_id,
        severity,
        &lifecycle_plan,
        &pending_report,
    )
}

fn build_external_upgrade_consent_evidence(
    request: ExternalUpgradeConsentEvidenceRequest,
) -> Result<ExternalUpgradeConsentEvidenceV1, DeployCommandError> {
    external_upgrade_consent_evidence_from_receipt(
        request.evidence_id,
        &request.proposal,
        &request.receipt,
    )
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

fn build_external_upgrade_verification_policy(
    request: ExternalUpgradeVerificationPolicyRequest,
) -> ExternalUpgradeVerificationPolicyV1 {
    external_upgrade_verification_policy_from_proposal(request.policy_id, &request.proposal)
}

fn build_external_upgrade_verification_check(
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

fn build_external_upgrade_completion_report(
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

fn build_external_upgrade_verification_report(
    request: ExternalUpgradeVerificationReportRequest,
) -> Result<ExternalUpgradeVerificationReportV1, DeployCommandError> {
    external_upgrade_verification_report_from_receipt(
        request.report_id,
        &request.proposal,
        &request.receipt,
    )
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

fn local_external_lifecycle_plan_id(check: &DeploymentCheckV1) -> String {
    local_external_artifact_id(check, "external-lifecycle-plan")
}

fn local_external_check_id(check: &DeploymentCheckV1) -> String {
    local_external_artifact_id(check, "external-lifecycle-check")
}

fn local_external_handoff_id(check: &DeploymentCheckV1) -> String {
    local_external_artifact_id(check, "external-lifecycle-handoff")
}

fn local_lifecycle_authority_report_id(check: &DeploymentCheckV1) -> String {
    local_external_artifact_id(check, "lifecycle-authority-report")
}

fn local_external_proposal_report_id(check: &DeploymentCheckV1) -> String {
    local_external_artifact_id(check, "external-upgrade-proposals")
}

fn local_external_pending_report_id(check: &DeploymentCheckV1) -> String {
    local_external_artifact_id(check, "external-lifecycle-pending")
}

fn local_external_critical_fix_report_id(check: &DeploymentCheckV1) -> String {
    local_external_artifact_id(check, "critical-external-fix")
}

fn local_external_artifact_id(check: &DeploymentCheckV1, suffix: &str) -> String {
    format!(
        "local:{}:{}:{suffix}",
        check.plan.runtime_variant, check.plan.deployment_identity.deployment_name
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

fn build_deployment_comparison_report(
    options: DeployCompareOptions,
) -> Result<DeploymentComparisonReportV1, DeployCommandError> {
    let left = read_json_file::<DeploymentCheckV1>(&options.left)?;
    let right = read_json_file::<DeploymentCheckV1>(&options.right)?;
    build_deployment_comparison_report_from_checks(
        &left,
        &right,
        options.left_label.as_deref(),
        options.right_label.as_deref(),
    )
}

fn build_deployment_comparison_report_from_checks(
    left: &DeploymentCheckV1,
    right: &DeploymentCheckV1,
    left_label: Option<&str>,
    right_label: Option<&str>,
) -> Result<DeploymentComparisonReportV1, DeployCommandError> {
    let left_label = left_label.unwrap_or(left.plan.deployment_identity.deployment_name.as_str());
    let right_label =
        right_label.unwrap_or(right.plan.deployment_identity.deployment_name.as_str());
    let report = deployment_comparison_report_from_checks(
        local_deployment_comparison_report_id(left_label, right_label),
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

fn local_deployment_comparison_report_id(left_label: &str, right_label: &str) -> String {
    format!("local:{left_label}:{right_label}:deployment-comparison")
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

fn read_install_deployment_plan(
    path: &PathBuf,
) -> Result<DeployInstallPlanInput, DeployCommandError> {
    let bytes = fs::read(path).map_err(Box::<dyn std::error::Error>::from)?;
    if let Ok(plan) = serde_json::from_slice::<ArtifactPromotionPlanV1>(&bytes) {
        validate_artifact_promotion_plan(&plan).map_err(Box::<dyn std::error::Error>::from)?;
        if plan.status != PromotionReadinessStatusV1::Ready {
            return Err(DeployCommandError::Blocked(format!(
                "artifact promotion plan {} is not ready",
                plan.plan_id
            )));
        }
        return Ok(DeployInstallPlanInput {
            deployment_plan: plan.transform.promoted_plan.clone(),
            artifact_promotion_plan: Some(plan),
        });
    }

    serde_json::from_slice::<DeploymentPlanV1>(&bytes)
        .map(|deployment_plan| DeployInstallPlanInput {
            deployment_plan,
            artifact_promotion_plan: None,
        })
        .map_err(|err| {
            DeployCommandError::Check(
                format!(
                    "failed to decode {} as ArtifactPromotionPlanV1 or DeploymentPlanV1: {err}",
                    path.display()
                )
                .into(),
            )
        })
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

impl DeployInstallPlanOptions {
    fn parse<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(deploy_install_command(), args)
            .map_err(|_| DeployCommandError::Usage(deploy_install_usage()))?;
        Ok(Self {
            deployment: string_option(&matches, "deployment").expect("clap requires deployment"),
            plan: path_option(&matches, "plan").expect("clap requires plan"),
            network: string_option(&matches, "network").unwrap_or_else(local_network),
            profile: string_option(&matches, "profile")
                .as_deref()
                .map(|profile| parse_profile(profile, deploy_install_usage))
                .transpose()?,
        })
    }

    fn into_install_root_options(
        self,
        plan: DeployInstallPlanInput,
        icp_root: Option<std::path::PathBuf>,
    ) -> InstallRootOptions {
        let fleet_template = plan.deployment_plan.fleet_template.clone();
        InstallRootOptions {
            root_canister: root_canister_for_plan(&plan.deployment_plan),
            root_build_target: DEFAULT_ROOT_TARGET.to_string(),
            network: self.network,
            deployment_name: Some(self.deployment),
            icp_root,
            build_profile: self.profile,
            ready_timeout_seconds: DEFAULT_READY_TIMEOUT_SECONDS,
            config_path: Some(default_fleet_config_path(&fleet_template)),
            expected_fleet: Some(fleet_template),
            interactive_config_selection: false,
            deployment_plan_override: Some(plan.deployment_plan),
            artifact_promotion_plan_override: plan.artifact_promotion_plan,
        }
    }
}

impl DeployRegisterOptions {
    fn parse<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(deploy_register_command(), args)
            .map_err(|_| DeployCommandError::Usage(deploy_register_usage()))?;
        Ok(Self {
            deployment: string_option(&matches, "deployment").expect("clap requires deployment"),
            fleet_template: string_option(&matches, "fleet-template")
                .expect("clap requires fleet-template"),
            root: string_option(&matches, "root").expect("clap requires root"),
            network: string_option(&matches, "network").unwrap_or_else(local_network),
            allow_unverified: matches.get_flag("allow-unverified"),
        })
    }

    fn into_register_options(
        self,
        icp_root: Option<std::path::PathBuf>,
    ) -> RegisterDeploymentStateOptions {
        RegisterDeploymentStateOptions {
            deployment_name: self.deployment,
            fleet_template: self.fleet_template,
            root_canister_id: self.root,
            network: self.network,
            allow_unverified: self.allow_unverified,
            icp_root,
            workspace_root: None,
        }
    }
}

impl DeployCompareOptions {
    fn parse<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(deploy_compare_command(), args)
            .map_err(|_| DeployCommandError::Usage(compare_usage()))?;
        Ok(Self {
            left: path_option(&matches, "left").expect("clap requires left"),
            right: path_option(&matches, "right").expect("clap requires right"),
            left_label: string_option(&matches, "left-label"),
            right_label: string_option(&matches, "right-label"),
            format: parse_compare_output_format(
                string_option(&matches, "format").as_deref(),
                compare_usage,
            )?,
        })
    }
}

fn root_canister_for_plan(plan: &DeploymentPlanV1) -> String {
    plan.trust_domain
        .root_trust_anchor
        .clone()
        .or_else(|| plan.deployment_identity.root_principal.clone())
        .or_else(|| {
            plan.expected_canisters
                .iter()
                .find(|canister| canister.role == DEFAULT_ROOT_TARGET)
                .and_then(|canister| canister.canister_id.clone())
        })
        .unwrap_or_else(|| DEFAULT_ROOT_TARGET.to_string())
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

impl DeployExternalOptions {
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
            format: parse_external_output_format(
                string_option(&matches, "format").as_deref(),
                usage,
            )?,
        })
    }
}

impl DeployExternalCriticalFixOptions {
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
            format: parse_external_output_format(
                string_option(&matches, "format").as_deref(),
                usage,
            )?,
        })
    }
}

impl DeployExternalInspectOptions {
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
            format: parse_external_output_format(
                string_option(&matches, "format").as_deref(),
                usage,
            )?,
        })
    }
}

impl DeployRootInspectOptions {
    fn parse<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(deploy_root_inspect_command(), args)
            .map_err(|_| DeployCommandError::Usage(root_inspect_usage()))?;
        Ok(Self {
            request: path_option(&matches, "request").expect("clap requires request"),
            format: parse_root_output_format(
                string_option(&matches, "format").as_deref(),
                root_inspect_usage,
            )?,
        })
    }
}

impl DeployRootVerifyOptions {
    fn parse<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(deploy_root_verify_command(), args)
            .map_err(|_| DeployCommandError::Usage(root_verify_usage()))?;
        Ok(Self {
            deployment: string_option(&matches, "deployment").expect("clap requires deployment"),
            from_check: path_option(&matches, "from-check").expect("clap requires from-check"),
            network: string_option(&matches, "network").unwrap_or_else(local_network),
            format: parse_root_output_format(
                string_option(&matches, "format").as_deref(),
                root_verify_usage,
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

fn deploy_root_command() -> ClapCommand {
    ClapCommand::new("root")
        .bin_name("canic deploy root")
        .about("Inspect or verify deployment-root evidence")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("inspect")
                .about("Inspect deployment-root verification evidence")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("verify")
                .about("Verify a registered deployment root from check evidence")
                .disable_help_flag(true),
        ))
        .after_help(DEPLOY_ROOT_HELP_AFTER)
}

fn deploy_root_inspect_command() -> ClapCommand {
    ClapCommand::new("inspect")
        .bin_name("canic deploy root inspect")
        .about("Inspect deployment-root verification evidence")
        .disable_help_flag(true)
        .override_usage("canic deploy root inspect --request <file>")
        .arg(
            value_arg("request")
                .long("request")
                .value_name("file")
                .required(true)
                .help("DeploymentRootVerificationRequestV1 JSON file to inspect"),
        )
        .arg(
            value_arg("format")
                .long("format")
                .value_name("json|text")
                .num_args(1)
                .help("Output format; defaults to json"),
        )
        .after_help(DEPLOY_ROOT_INSPECT_HELP_AFTER)
}

fn deploy_root_verify_command() -> ClapCommand {
    ClapCommand::new("verify")
        .bin_name("canic deploy root verify")
        .about("Verify a registered deployment root from check evidence")
        .disable_help_flag(true)
        .override_usage("canic deploy root verify <deployment> --from-check <file>")
        .arg(
            value_arg("deployment")
                .value_name("deployment")
                .required(true)
                .help("Registered deployment target whose root should be verified"),
        )
        .arg(
            value_arg("from-check")
                .long("from-check")
                .value_name("file")
                .required(true)
                .help("DeploymentCheckV1 JSON artifact carrying explicit root evidence"),
        )
        .arg(root_format_arg())
        .arg(internal_network_arg())
        .after_help(DEPLOY_ROOT_VERIFY_HELP_AFTER)
}

fn deploy_external_command() -> ClapCommand {
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

fn deploy_external_inspect_command() -> ClapCommand {
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

fn deploy_install_command() -> ClapCommand {
    ClapCommand::new("install")
        .bin_name("canic deploy install")
        .about("Install through the current runner using a supplied deployment plan")
        .disable_help_flag(true)
        .override_usage("canic deploy install <deployment> --plan <file>")
        .arg(
            value_arg("deployment")
                .required(true)
                .help("Deployment target name that must match the supplied plan"),
        )
        .arg(
            value_arg("plan")
                .long("plan")
                .value_name("file")
                .required(true)
                .help("DeploymentPlanV1 or ArtifactPromotionPlanV1 JSON file to install"),
        )
        .arg(
            value_arg("profile")
                .long("profile")
                .value_name("debug|fast|release")
                .num_args(1)
                .help("Canister wasm build profile; defaults to CANIC_WASM_PROFILE or release"),
        )
        .arg(internal_network_arg())
        .after_help(DEPLOY_INSTALL_HELP_AFTER)
}

fn deploy_register_command() -> ClapCommand {
    ClapCommand::new("register")
        .bin_name("canic deploy register")
        .about("Register minimal deployment-target state")
        .disable_help_flag(true)
        .override_usage(
            "canic deploy register <deployment> --fleet-template <fleet> --root <principal> --allow-unverified",
        )
        .arg(
            value_arg("deployment")
                .required(true)
                .help("Deployment target name to register"),
        )
        .arg(
            value_arg("fleet-template")
                .long("fleet-template")
                .value_name("fleet")
                .required(true)
                .help("Reusable fleet template this deployment target uses"),
        )
        .arg(
            value_arg("root")
                .long("root")
                .value_name("principal")
                .required(true)
                .help("Existing root canister principal for this deployment"),
        )
        .arg(
            clap::Arg::new("allow-unverified")
                .long("allow-unverified")
                .action(ArgAction::SetTrue)
                .required(true)
                .help("Acknowledge that the registered root is not live-verified"),
        )
        .arg(internal_network_arg())
        .after_help(DEPLOY_REGISTER_HELP_AFTER)
}

fn deploy_compare_command() -> ClapCommand {
    ClapCommand::new("compare")
        .bin_name("canic deploy compare")
        .about("Compare two deployment truth check artifacts")
        .disable_help_flag(true)
        .override_usage("canic deploy compare --left <file> --right <file>")
        .arg(
            value_arg("left")
                .long("left")
                .value_name("file")
                .required(true)
                .help("Left DeploymentCheckV1 JSON artifact"),
        )
        .arg(
            value_arg("right")
                .long("right")
                .value_name("file")
                .required(true)
                .help("Right DeploymentCheckV1 JSON artifact"),
        )
        .arg(
            value_arg("left-label")
                .long("left-label")
                .value_name("label")
                .help("Optional display label for the left artifact"),
        )
        .arg(
            value_arg("right-label")
                .long("right-label")
                .value_name("label")
                .help("Optional display label for the right artifact"),
        )
        .arg(compare_format_arg())
        .after_help(DEPLOY_COMPARE_HELP_AFTER)
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

fn deploy_external_plan_command() -> ClapCommand {
    deploy_truth_leaf_command("plan", "Print the local external lifecycle plan")
        .arg(external_format_arg())
        .bin_name("canic deploy external plan")
        .after_help(DEPLOY_EXTERNAL_PLAN_HELP_AFTER)
}

fn deploy_external_check_command() -> ClapCommand {
    deploy_truth_leaf_command("check", "Print the local external lifecycle check")
        .arg(external_format_arg())
        .bin_name("canic deploy external check")
        .after_help(DEPLOY_EXTERNAL_CHECK_HELP_AFTER)
}

fn deploy_external_handoff_command() -> ClapCommand {
    deploy_truth_leaf_command("handoff", "Print the local external lifecycle handoff")
        .arg(external_format_arg())
        .bin_name("canic deploy external handoff")
        .after_help(DEPLOY_EXTERNAL_HANDOFF_HELP_AFTER)
}

fn deploy_external_proposals_command() -> ClapCommand {
    deploy_truth_leaf_command("proposals", "Print local external upgrade proposals")
        .arg(external_format_arg())
        .bin_name("canic deploy external proposals")
        .after_help(DEPLOY_EXTERNAL_PROPOSALS_HELP_AFTER)
}

fn deploy_external_pending_command() -> ClapCommand {
    deploy_truth_leaf_command(
        "pending",
        "Print the local external lifecycle pending report",
    )
    .arg(external_format_arg())
    .bin_name("canic deploy external pending")
    .after_help(DEPLOY_EXTERNAL_PENDING_HELP_AFTER)
}

fn deploy_external_critical_fix_command() -> ClapCommand {
    deploy_truth_leaf_command(
        "critical-fix",
        "Print the local critical external fix report",
    )
    .arg(external_format_arg())
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

fn deploy_external_verify_command() -> ClapCommand {
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
        .arg(external_format_arg())
        .after_help(DEPLOY_EXTERNAL_VERIFY_HELP_AFTER)
}

fn deploy_external_inspect_consent_command() -> ClapCommand {
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
        .arg(external_format_arg())
        .after_help(DEPLOY_EXTERNAL_CONSENT_HELP_AFTER)
}

fn deploy_external_inspect_verification_policy_command() -> ClapCommand {
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
        .arg(external_format_arg())
        .after_help(DEPLOY_EXTERNAL_VERIFICATION_POLICY_HELP_AFTER)
}

fn deploy_external_inspect_verification_check_command() -> ClapCommand {
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
        .arg(external_format_arg())
        .after_help(DEPLOY_EXTERNAL_VERIFICATION_CHECK_HELP_AFTER)
}

fn deploy_external_inspect_completion_command() -> ClapCommand {
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
        .arg(external_format_arg())
        .after_help(DEPLOY_EXTERNAL_COMPLETION_HELP_AFTER)
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

fn authority_format_arg() -> clap::Arg {
    value_arg("format")
        .long("format")
        .value_name("json|text")
        .num_args(1)
        .help("Output format; defaults to json")
}

fn external_format_arg() -> clap::Arg {
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

fn compare_format_arg() -> clap::Arg {
    value_arg("format")
        .long("format")
        .value_name("json|text")
        .num_args(1)
        .help("Output format; defaults to json")
}

fn root_format_arg() -> clap::Arg {
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

fn compare_usage() -> String {
    let mut command = deploy_compare_command();
    command.render_help().to_string()
}

fn promote_usage() -> String {
    let mut command = deploy_promote_command();
    command.render_help().to_string()
}

fn external_usage() -> String {
    let mut command = deploy_external_command();
    command.render_help().to_string()
}

fn external_plan_usage() -> String {
    let mut command = deploy_external_plan_command();
    command.render_help().to_string()
}

fn external_check_usage() -> String {
    let mut command = deploy_external_check_command();
    command.render_help().to_string()
}

fn external_handoff_usage() -> String {
    let mut command = deploy_external_handoff_command();
    command.render_help().to_string()
}

fn external_proposals_usage() -> String {
    let mut command = deploy_external_proposals_command();
    command.render_help().to_string()
}

fn external_pending_usage() -> String {
    let mut command = deploy_external_pending_command();
    command.render_help().to_string()
}

fn external_critical_fix_usage() -> String {
    let mut command = deploy_external_critical_fix_command();
    command.render_help().to_string()
}

fn external_inspect_usage() -> String {
    let mut command = deploy_external_inspect_command();
    command.render_help().to_string()
}

fn external_consent_usage() -> String {
    let mut command = deploy_external_inspect_consent_command();
    command.render_help().to_string()
}

fn external_verification_policy_usage() -> String {
    let mut command = deploy_external_inspect_verification_policy_command();
    command.render_help().to_string()
}

fn external_verification_check_usage() -> String {
    let mut command = deploy_external_inspect_verification_check_command();
    command.render_help().to_string()
}

fn external_completion_usage() -> String {
    let mut command = deploy_external_inspect_completion_command();
    command.render_help().to_string()
}

fn external_verify_usage() -> String {
    let mut command = deploy_external_verify_command();
    command.render_help().to_string()
}

fn root_usage() -> String {
    let mut command = deploy_root_command();
    command.render_help().to_string()
}

fn root_inspect_usage() -> String {
    let mut command = deploy_root_inspect_command();
    command.render_help().to_string()
}

fn root_verify_usage() -> String {
    let mut command = deploy_root_verify_command();
    command.render_help().to_string()
}

fn deploy_install_usage() -> String {
    let mut command = deploy_install_command();
    command.render_help().to_string()
}

fn deploy_register_usage() -> String {
    let mut command = deploy_register_command();
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

fn parse_external_output_format(
    value: Option<&str>,
    usage: fn() -> String,
) -> Result<ExternalOutputFormat, DeployCommandError> {
    match value.unwrap_or("json") {
        "json" => Ok(ExternalOutputFormat::Json),
        "text" => Ok(ExternalOutputFormat::Text),
        other => Err(DeployCommandError::Usage(format!(
            "invalid external lifecycle output format: {other}\n\n{}",
            usage()
        ))),
    }
}

fn parse_compare_output_format(
    value: Option<&str>,
    usage: fn() -> String,
) -> Result<CompareOutputFormat, DeployCommandError> {
    match value.unwrap_or("json") {
        "json" => Ok(CompareOutputFormat::Json),
        "text" => Ok(CompareOutputFormat::Text),
        other => Err(DeployCommandError::Usage(format!(
            "invalid deployment comparison output format: {other}\n\n{}",
            usage()
        ))),
    }
}

fn parse_root_output_format(
    value: Option<&str>,
    usage: fn() -> String,
) -> Result<RootOutputFormat, DeployCommandError> {
    match value.unwrap_or("json") {
        "json" => Ok(RootOutputFormat::Json),
        "text" => Ok(RootOutputFormat::Text),
        other => Err(DeployCommandError::Usage(format!(
            "invalid deployment root output format: {other}\n\n{}",
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
        ArtifactDigestSourceV1, ArtifactSourceV1, AuthorityProfileV1, CanisterControlClassV1,
        ConsentChannelKindV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentDiffV1,
        DeploymentIdentityV1, DeploymentInventoryV1, DeploymentPlanV1,
        DeploymentRootObservationSourceV1, DeploymentRootObservationV1,
        DeploymentRootVerificationEvidenceStatusV1, DeploymentRootVerificationRequestV1,
        DeploymentRootVerificationSourceV1, DeploymentRootVerificationStateTransitionV1,
        DeploymentRootVerificationStateV1, ExpectedCanisterV1, ExternalUpgradeCompletionStatusV1,
        ExternalUpgradeConsentStateV1, ExternalUpgradeVerificationObservationV1,
        ExternalUpgradeVerificationRequirementStatusV1, ExternalUpgradeVerificationResultV1,
        ExternalVerificationObservationSourceV1, LifecycleVerificationRequirementV1,
        LocalDeploymentConfigV1, ObservationStatusV1, ObservedArtifactV1, ObservedCanisterV1,
        PreviousArtifactReceiptKindV1, PromotionArtifactLevelV1, ResumeSafetyV1,
        RoleArtifactSourceKindV1, RoleArtifactSourceV1, RoleArtifactV1, RolePromotionInputV1,
        TrustDomainV1, VerifierReadinessExpectationV1, VerifierReadinessObservationV1,
        compare_plan_to_inventory, external_upgrade_receipt_from_observation,
        promotion_readiness_from_inputs, safety_report_from_diff,
    };

    #[test]
    fn deploy_check_parses_required_deployment() {
        let options =
            DeployTruthOptions::parse([OsString::from("demo")], deploy_check_command, check_usage)
                .expect("parse deploy check");

        assert_eq!(options.deployment, "demo");
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
        std::assert_matches!(
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
        );
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

        std::assert_matches!(
            enforce_deployment_check_status(&report),
            Err(DeployCommandError::Blocked(message))
                if message == "deployment inventory has 1 blocking issue(s) and 0 warning(s)"
        );
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

        assert_eq!(plan.deployment, "demo");
        assert_eq!(inventory.deployment, "demo");
        assert_eq!(diff.deployment, "demo");
        assert_eq!(report.deployment, "demo");
        assert_eq!(resume_report.truth.deployment, "demo");
        assert_eq!(resume_report.receipt, Some(PathBuf::from("receipt.json")));
    }

    #[test]
    fn deploy_compare_parses_artifact_paths_and_text_format() {
        let options = DeployCompareOptions::parse([
            OsString::from("--left"),
            OsString::from("staging-check.json"),
            OsString::from("--right"),
            OsString::from("prod-check.json"),
            OsString::from("--left-label"),
            OsString::from("staging"),
            OsString::from("--right-label"),
            OsString::from("prod"),
            OsString::from("--format"),
            OsString::from("text"),
        ])
        .expect("parse deploy compare");

        assert_eq!(options.left, PathBuf::from("staging-check.json"));
        assert_eq!(options.right, PathBuf::from("prod-check.json"));
        assert_eq!(options.left_label.as_deref(), Some("staging"));
        assert_eq!(options.right_label.as_deref(), Some("prod"));
        assert_eq!(options.format, CompareOutputFormat::Text);
    }

    #[test]
    fn deploy_compare_builder_uses_existing_check_artifacts() {
        let left = sample_authority_check();
        let mut right = sample_authority_check();
        right.plan.deployment_identity.deployment_name = "prod".to_string();

        let report =
            build_deployment_comparison_report_from_checks(&left, &right, Some("stage"), None)
                .expect("comparison report should build");

        assert_eq!(report.report_id, "local:stage:prod:deployment-comparison");
        assert_eq!(report.left.label, "stage");
        assert_eq!(report.right.label, "prod");
        assert!(!report.identity_diff.is_empty());
        assert_eq!(report.report_digest.len(), 64);
    }

    #[test]
    fn deploy_compare_rejects_unknown_format() {
        let result = DeployCompareOptions::parse([
            OsString::from("--left"),
            OsString::from("staging-check.json"),
            OsString::from("--right"),
            OsString::from("prod-check.json"),
            OsString::from("--format"),
            OsString::from("yaml"),
        ]);

        std::assert_matches!(
            result,
            Err(DeployCommandError::Usage(message))
                if message.contains("invalid deployment comparison output format: yaml")
        );
    }

    #[test]
    fn deploy_root_inspect_parses_request_and_text_format() {
        let options = DeployRootInspectOptions::parse([
            OsString::from("--request"),
            OsString::from("root-verification.json"),
            OsString::from("--format"),
            OsString::from("text"),
        ])
        .expect("parse deploy root inspect");

        assert_eq!(options.request, PathBuf::from("root-verification.json"));
        assert_eq!(options.format, RootOutputFormat::Text);
    }

    #[test]
    fn deploy_root_inspect_defaults_to_json() {
        let options = DeployRootInspectOptions::parse([
            OsString::from("--request"),
            OsString::from("root-verification.json"),
        ])
        .expect("parse deploy root inspect");

        assert_eq!(options.request, PathBuf::from("root-verification.json"));
        assert_eq!(options.format, RootOutputFormat::Json);
    }

    #[test]
    fn deploy_root_inspect_rejects_unknown_format() {
        let result = DeployRootInspectOptions::parse([
            OsString::from("--request"),
            OsString::from("root-verification.json"),
            OsString::from("--format"),
            OsString::from("yaml"),
        ]);

        std::assert_matches!(
            result,
            Err(DeployCommandError::Usage(message))
                if message.contains("invalid deployment root output format: yaml")
        );
    }

    #[test]
    fn deploy_root_verify_parses_deployment_check_and_text_format() {
        let options = DeployRootVerifyOptions::parse([
            OsString::from("demo-local"),
            OsString::from("--from-check"),
            OsString::from("deployment-check.json"),
            OsString::from("--format"),
            OsString::from("text"),
            OsString::from("--__canic-network"),
            OsString::from("ic"),
        ])
        .expect("parse deploy root verify");

        assert_eq!(options.deployment, "demo-local");
        assert_eq!(options.from_check, PathBuf::from("deployment-check.json"));
        assert_eq!(options.network, "ic");
        assert_eq!(options.format, RootOutputFormat::Text);
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
            assert_eq!(options.truth.deployment, "demo");
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

        assert_eq!(authority_check.truth.deployment, "demo");
        assert_eq!(authority_check.format, AuthorityOutputFormat::Text);
        assert_eq!(authority_evidence.truth.deployment, "demo");
        assert_eq!(authority_evidence.format, AuthorityOutputFormat::Text);
        assert_eq!(authority_report.truth.deployment, "demo");
        assert_eq!(authority_report.format, AuthorityOutputFormat::Text);
        assert_eq!(authority_receipt.truth.deployment, "demo");
        assert_eq!(authority_receipt.format, AuthorityOutputFormat::Text);
    }

    #[test]
    fn deploy_external_leaf_commands_default_to_json() {
        let external_plan = DeployExternalOptions::parse(
            [OsString::from("demo")],
            deploy_external_plan_command,
            external_plan_usage,
        )
        .expect("parse deploy external plan");
        let external_check = DeployExternalOptions::parse(
            [OsString::from("demo")],
            deploy_external_check_command,
            external_check_usage,
        )
        .expect("parse deploy external check");
        let external_handoff = DeployExternalOptions::parse(
            [OsString::from("demo")],
            deploy_external_handoff_command,
            external_handoff_usage,
        )
        .expect("parse deploy external handoff");
        let external_proposals = DeployExternalOptions::parse(
            [OsString::from("demo")],
            deploy_external_proposals_command,
            external_proposals_usage,
        )
        .expect("parse deploy external proposals");
        let external_pending = DeployExternalOptions::parse(
            [OsString::from("demo")],
            deploy_external_pending_command,
            external_pending_usage,
        )
        .expect("parse deploy external pending");

        for options in [
            external_plan,
            external_check,
            external_handoff,
            external_proposals,
            external_pending,
        ] {
            assert_eq!(options.truth.deployment, "demo");
            assert_eq!(options.format, ExternalOutputFormat::Json);
        }
        let critical_fix = DeployExternalCriticalFixOptions::parse(
            [
                OsString::from("--fix-id"),
                OsString::from("fix-2026-05"),
                OsString::from("--severity"),
                OsString::from("critical"),
                OsString::from("demo"),
            ],
            deploy_external_critical_fix_command,
            external_critical_fix_usage,
        )
        .expect("parse deploy external critical-fix");
        assert_eq!(critical_fix.truth.deployment, "demo");
        assert_eq!(critical_fix.format, ExternalOutputFormat::Json);
        assert_eq!(critical_fix.fix_id, "fix-2026-05");
        assert_eq!(critical_fix.severity, "critical");
        let verify = DeployExternalVerifyOptions::parse(
            [
                OsString::from("--request"),
                OsString::from("external-verification.json"),
            ],
            deploy_external_verify_command,
            external_verify_usage,
        )
        .expect("parse deploy external verify");
        assert_eq!(verify.request, PathBuf::from("external-verification.json"));
        assert_eq!(verify.format, ExternalOutputFormat::Json);
        let consent = DeployExternalInspectOptions::parse(
            [
                OsString::from("--request"),
                OsString::from("external-consent.json"),
            ],
            deploy_external_inspect_consent_command,
            external_consent_usage,
        )
        .expect("parse deploy external inspect consent");
        assert_eq!(consent.request, PathBuf::from("external-consent.json"));
        assert_eq!(consent.format, ExternalOutputFormat::Json);
    }

    #[test]
    fn deploy_external_leaf_commands_parse_text_format() {
        let external_plan = DeployExternalOptions::parse(
            [
                OsString::from("--format"),
                OsString::from("text"),
                OsString::from("demo"),
            ],
            deploy_external_plan_command,
            external_plan_usage,
        )
        .expect("parse deploy external plan text");
        let external_check = DeployExternalOptions::parse(
            [
                OsString::from("--format"),
                OsString::from("text"),
                OsString::from("demo"),
            ],
            deploy_external_check_command,
            external_check_usage,
        )
        .expect("parse deploy external check text");
        let external_handoff = DeployExternalOptions::parse(
            [
                OsString::from("--format"),
                OsString::from("text"),
                OsString::from("demo"),
            ],
            deploy_external_handoff_command,
            external_handoff_usage,
        )
        .expect("parse deploy external handoff text");
        let external_proposals = DeployExternalOptions::parse(
            [
                OsString::from("--format"),
                OsString::from("text"),
                OsString::from("demo"),
            ],
            deploy_external_proposals_command,
            external_proposals_usage,
        )
        .expect("parse deploy external proposals text");
        let external_pending = DeployExternalOptions::parse(
            [
                OsString::from("--format"),
                OsString::from("text"),
                OsString::from("demo"),
            ],
            deploy_external_pending_command,
            external_pending_usage,
        )
        .expect("parse deploy external pending text");

        assert_eq!(external_plan.truth.deployment, "demo");
        assert_eq!(external_plan.format, ExternalOutputFormat::Text);
        assert_eq!(external_check.truth.deployment, "demo");
        assert_eq!(external_check.format, ExternalOutputFormat::Text);
        assert_eq!(external_handoff.truth.deployment, "demo");
        assert_eq!(external_handoff.format, ExternalOutputFormat::Text);
        assert_eq!(external_proposals.truth.deployment, "demo");
        assert_eq!(external_proposals.format, ExternalOutputFormat::Text);
        assert_eq!(external_pending.truth.deployment, "demo");
        assert_eq!(external_pending.format, ExternalOutputFormat::Text);
    }

    #[test]
    fn deploy_external_request_commands_parse_text_format() {
        let critical_fix = DeployExternalCriticalFixOptions::parse(
            [
                OsString::from("--fix-id"),
                OsString::from("fix-2026-05"),
                OsString::from("--severity"),
                OsString::from("critical"),
                OsString::from("--format"),
                OsString::from("text"),
                OsString::from("demo"),
            ],
            deploy_external_critical_fix_command,
            external_critical_fix_usage,
        )
        .expect("parse deploy external critical-fix text");
        assert_eq!(critical_fix.truth.deployment, "demo");
        assert_eq!(critical_fix.format, ExternalOutputFormat::Text);
        assert_eq!(critical_fix.fix_id, "fix-2026-05");
        assert_eq!(critical_fix.severity, "critical");
        let verify = DeployExternalVerifyOptions::parse(
            [
                OsString::from("--request"),
                OsString::from("external-verification.json"),
                OsString::from("--format"),
                OsString::from("text"),
            ],
            deploy_external_verify_command,
            external_verify_usage,
        )
        .expect("parse deploy external verify text");
        assert_eq!(verify.request, PathBuf::from("external-verification.json"));
        assert_eq!(verify.format, ExternalOutputFormat::Text);
        let consent = DeployExternalInspectOptions::parse(
            [
                OsString::from("--request"),
                OsString::from("external-consent.json"),
                OsString::from("--format"),
                OsString::from("text"),
            ],
            deploy_external_inspect_consent_command,
            external_consent_usage,
        )
        .expect("parse deploy external inspect consent text");
        assert_eq!(consent.request, PathBuf::from("external-consent.json"));
        assert_eq!(consent.format, ExternalOutputFormat::Text);
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
    fn deploy_root_help_documents_passive_boundary() {
        let help = root_usage();
        let inspect_help = root_inspect_usage();
        let verify_help = root_verify_usage();

        assert!(help.contains("Inspect or verify deployment-root evidence"));
        assert!(help.contains("deployment-root scoped"));
        assert!(help.contains("Verify records verified root"));
        assert!(inspect_help.contains("DeploymentRootVerificationRequestV1-shaped JSON"));
        assert!(inspect_help.contains("does not persist verified root state"));
        assert!(inspect_help.contains("EvidenceSatisfied means"));
        assert!(verify_help.contains("Verifies a registered deployment root"));
        assert!(verify_help.contains("not full deployment verification"));
        assert!(verify_help.contains("does not install"));
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
    fn deploy_promote_help_keeps_advanced_reports_under_inspect() {
        let help = promote_usage();

        for advanced_command in [
            "target-lineage",
            "wasm-store-identity",
            "catalog-verification",
            "materialization-identity",
            "execution-receipt",
            "transform-evidence",
            "policy",
        ] {
            assert!(
                !help.contains(&format!("canic deploy promote {advanced_command}")),
                "advanced promotion report {advanced_command} must stay below inspect"
            );
        }
        assert!(help.contains("canic deploy promote inspect readiness"));
        assert!(help.contains("canic deploy promote inspect artifact-identity"));
        assert!(help.contains("canic deploy promote inspect provenance"));
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
    fn deploy_external_help_documents_passive_scope() {
        let help = external_usage();
        let plan_help = external_plan_usage();
        let check_help = external_check_usage();
        let handoff_help = external_handoff_usage();
        let proposals_help = external_proposals_usage();
        let pending_help = external_pending_usage();
        let critical_fix_help = external_critical_fix_usage();
        let inspect_help = external_inspect_usage();
        let consent_help = external_consent_usage();
        let verification_policy_help = external_verification_policy_usage();
        let verification_check_help = external_verification_check_usage();
        let completion_help = external_completion_usage();
        let verify_help = external_verify_usage();

        assert!(help.contains("Build passive external lifecycle reports"));
        assert!(help.contains("do not request"));
        assert!(help.contains("mutate deployment state"));
        assert!(help.contains("Build a passive external lifecycle check"));
        assert!(help.contains("Build a passive external lifecycle handoff packet"));
        assert!(help.contains("Build a passive external lifecycle pending report"));
        assert!(help.contains("Build a passive critical external fix report"));
        assert!(help.contains("Inspect passive external lifecycle internals"));
        assert!(help.contains("Build a passive external upgrade verification report"));
        assert!(plan_help.contains("ExternalLifecyclePlanV1 JSON"));
        assert!(plan_help.contains("No consent delivery"));
        assert!(check_help.contains("ExternalLifecycleCheckV1 JSON"));
        assert!(check_help.contains("summarize direct, pending"));
        assert!(handoff_help.contains("ExternalLifecycleHandoffV1 JSON"));
        assert!(handoff_help.contains("operator coordination instructions"));
        assert!(proposals_help.contains("ExternalUpgradeProposalReportV1 JSON"));
        assert!(proposals_help.contains("do not grant consent"));
        assert!(pending_help.contains("ExternalLifecyclePendingReportV1 JSON"));
        assert!(pending_help.contains("residual exposure"));
        assert!(critical_fix_help.contains("CriticalExternalFixReportV1 JSON"));
        assert!(critical_fix_help.contains("without claiming deployment completion"));
        assert!(inspect_help.contains("canic deploy external inspect consent"));
        assert!(inspect_help.contains("verification-policy"));
        assert!(inspect_help.contains("verification-check"));
        assert!(inspect_help.contains("completion"));
        assert!(inspect_help.contains("do not request consent"));
        assert!(consent_help.contains("ExternalUpgradeConsentEvidenceRequest-shaped JSON"));
        assert!(consent_help.contains("does not verify live completion"));
        assert!(
            verification_policy_help
                .contains("ExternalUpgradeVerificationPolicyRequest-shaped JSON")
        );
        assert!(verification_policy_help.contains("live-inventory"));
        assert!(verification_policy_help.contains("postconditions"));
        assert!(
            verification_check_help.contains("ExternalUpgradeVerificationCheckRequest-shaped JSON")
        );
        assert!(verification_check_help.contains("supplied observation facts"));
        assert!(verification_check_help.contains("DeploymentCheckV1 inventory artifact"));
        assert!(completion_help.contains("ExternalUpgradeCompletionReportRequest-shaped JSON"));
        assert!(completion_help.contains("proposal, consent evidence"));
        assert!(completion_help.contains("only deployment-truth inventory verification"));
        assert!(verify_help.contains("ExternalUpgradeVerificationReportRequest-shaped JSON"));
        assert!(verify_help.contains("live inventory remains the source of truth"));
    }

    #[test]
    fn deploy_compare_help_documents_passive_artifact_scope() {
        let help = compare_usage();

        assert!(help.contains("Compare two deployment truth check artifacts"));
        assert!(help.contains("DeploymentCheckV1 JSON artifacts"));
        assert!(help.contains("does not query live"));
        assert!(help.contains("install code"));
        assert!(help.contains("mutate deployments"));
        assert!(help.contains("embedded"));
        assert!(help.contains("revalidated"));
    }

    #[test]
    fn deploy_compare_path_has_no_live_lookup_or_mutation_primitives() {
        let source = include_str!("mod.rs");
        let compare_source = source_between(source, "fn run_compare<I>", "fn run_promote<I>");
        let compare_builder_source = source_between(
            source,
            "fn build_deployment_comparison_report",
            "fn load_deployment_check",
        );

        for forbidden in [
            "update_settings",
            "install_code",
            "create_canister",
            "delete_canister",
            "stop_canister",
            "uninstall_code",
            "provisional_create_canister",
            "dfx",
            "load_deployment_check",
            "check_install_deployment_truth",
            "resolve_current_canic_icp_root",
        ] {
            assert!(
                !compare_source.contains(forbidden),
                "deploy compare run path must stay passive; found forbidden token {forbidden}"
            );
            assert!(
                !compare_builder_source.contains(forbidden),
                "deploy compare builder must stay artifact-only; found forbidden token {forbidden}"
            );
        }
    }

    #[test]
    fn deploy_check_path_has_no_local_state_write_primitives() {
        let source = include_str!("mod.rs");
        let check_source = source_between(source, "fn run_check<I>", "fn run_resume_report<I>");
        let loader_source = source_between(source, "fn load_deployment_check", "fn print_json<T>");

        for forbidden in [
            "register_deployment_state",
            "write_install_state",
            "install_root(",
            "run_install(",
        ] {
            assert!(
                !check_source.contains(forbidden),
                "deploy check path must stay read-only; found forbidden token {forbidden}"
            );
            assert!(
                !loader_source.contains(forbidden),
                "deployment check loader must stay read-only; found forbidden token {forbidden}"
            );
        }
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
    fn deploy_external_path_has_no_mutation_primitives() {
        let source = include_str!("mod.rs");
        let external_source =
            source_between(source, "fn run_external<I>", "fn run_authority_output<I");
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
                !external_source.contains(forbidden),
                "external lifecycle CLI path must stay passive; found forbidden token {forbidden}"
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
    fn deploy_root_path_has_no_mutation_primitives() {
        let source = include_str!("mod.rs");
        let root_source = source_between(source, "fn run_root_inspect<I>", "fn run_root_verify<I>");
        for forbidden in [
            "update_settings",
            "install_code",
            "create_canister",
            "delete_canister",
            "stop_canister",
            "uninstall_code",
            "provisional_create_canister",
            "dfx",
            "register_deployment_state",
            "install_root",
        ] {
            assert!(
                !root_source.contains(forbidden),
                "root inspect CLI path must stay passive; found forbidden token {forbidden}"
            );
        }
    }

    #[test]
    fn deploy_root_verify_path_has_no_controller_mutation_primitives() {
        let source = include_str!("mod.rs");
        let root_source = source_between(source, "fn run_root_verify<I>", "fn run_install<I>");
        for forbidden in [
            "update_settings",
            "install_code",
            "create_canister",
            "delete_canister",
            "stop_canister",
            "uninstall_code",
            "provisional_create_canister",
            "dfx",
            "install_root",
            "register_deployment_state",
        ] {
            assert!(
                !root_source.contains(forbidden),
                "root verify CLI path must not mutate IC/controller state; found forbidden token {forbidden}"
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
    fn deploy_external_command_dispatches_passive_leaf_commands() {
        for command in [
            "plan",
            "check",
            "handoff",
            "proposals",
            "pending",
            "critical-fix",
        ] {
            let parsed = parse_subcommand(
                deploy_command(),
                [
                    OsString::from("external"),
                    OsString::from(command),
                    OsString::from("demo"),
                ],
            )
            .expect("parse deploy external")
            .expect("external command");

            assert_eq!(parsed.0, "external");

            let nested = parse_subcommand(deploy_external_command(), parsed.1)
                .expect("parse nested external")
                .expect("external leaf command");
            assert_eq!(nested.0, command);
            assert_eq!(nested.1, vec![OsString::from("demo")]);
        }

        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("external"),
                OsString::from("verify"),
                OsString::from("--request"),
                OsString::from("external-verification.json"),
            ],
        )
        .expect("parse deploy external verify")
        .expect("external command");

        assert_eq!(parsed.0, "external");

        let nested = parse_subcommand(deploy_external_command(), parsed.1)
            .expect("parse nested external verify")
            .expect("external verify command");
        assert_eq!(nested.0, "verify");
        assert_eq!(
            nested.1,
            vec![
                OsString::from("--request"),
                OsString::from("external-verification.json")
            ]
        );
    }

    #[test]
    fn deploy_external_inspect_dispatches_passive_leaf_commands() {
        for (command, request) in [
            ("consent", "external-consent.json"),
            ("verification-policy", "external-verification-policy.json"),
            ("verification-check", "external-verification-check.json"),
            ("completion", "external-completion.json"),
        ] {
            assert_external_inspect_dispatches(command, request);
        }
    }

    fn assert_external_inspect_dispatches(command: &str, request: &str) {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("external"),
                OsString::from("inspect"),
                OsString::from(command),
                OsString::from("--request"),
                OsString::from(request),
            ],
        )
        .expect("parse deploy external inspect")
        .expect("external command");

        assert_eq!(parsed.0, "external");

        let external = parse_subcommand(deploy_external_command(), parsed.1)
            .expect("parse nested external inspect")
            .expect("external inspect command");
        assert_eq!(external.0, "inspect");

        let inspect = parse_subcommand(deploy_external_inspect_command(), external.1)
            .expect("parse nested inspect command")
            .expect("external inspect leaf command");
        assert_eq!(inspect.0, command);
        assert_eq!(
            inspect.1,
            vec![OsString::from("--request"), OsString::from(request)]
        );
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
    fn deploy_install_command_dispatches_plan_install() {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("install"),
                OsString::from("demo-local"),
                OsString::from("--plan"),
                OsString::from("promoted-plan.json"),
            ],
        )
        .expect("parse deploy install")
        .expect("install command");

        assert_eq!(parsed.0, "install");
        assert_eq!(
            parsed.1,
            vec![
                OsString::from("demo-local"),
                OsString::from("--plan"),
                OsString::from("promoted-plan.json")
            ]
        );

        let options = DeployInstallPlanOptions::parse(parsed.1).expect("parse install plan");
        assert_eq!(options.deployment, "demo-local");
        assert_eq!(options.plan, PathBuf::from("promoted-plan.json"));
    }

    #[test]
    fn deploy_register_command_dispatches_register() {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("register"),
                OsString::from("demo-local"),
                OsString::from("--fleet-template"),
                OsString::from("demo"),
                OsString::from("--root"),
                OsString::from("uxrrr-q7777-77774-qaaaq-cai"),
                OsString::from("--allow-unverified"),
            ],
        )
        .expect("parse deploy register")
        .expect("register command");

        assert_eq!(parsed.0, "register");

        let options = DeployRegisterOptions::parse(parsed.1).expect("parse register options");
        assert_eq!(options.deployment, "demo-local");
        assert_eq!(options.fleet_template, "demo");
        assert_eq!(options.root, "uxrrr-q7777-77774-qaaaq-cai");
        assert!(options.allow_unverified);
    }

    #[test]
    fn deploy_root_command_dispatches_inspect() {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("root"),
                OsString::from("inspect"),
                OsString::from("--request"),
                OsString::from("root-verification.json"),
            ],
        )
        .expect("parse deploy root")
        .expect("root command");

        assert_eq!(parsed.0, "root");

        let root = parse_subcommand(deploy_root_command(), parsed.1)
            .expect("parse nested root")
            .expect("root inspect command");
        assert_eq!(root.0, "inspect");
        assert_eq!(
            root.1,
            vec![
                OsString::from("--request"),
                OsString::from("root-verification.json")
            ]
        );
    }

    #[test]
    fn deploy_root_command_dispatches_verify() {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("root"),
                OsString::from("verify"),
                OsString::from("demo-local"),
                OsString::from("--from-check"),
                OsString::from("deployment-check.json"),
            ],
        )
        .expect("parse deploy root")
        .expect("root command");

        assert_eq!(parsed.0, "root");

        let root = parse_subcommand(deploy_root_command(), parsed.1)
            .expect("parse nested root")
            .expect("root verify command");
        assert_eq!(root.0, "verify");
        assert_eq!(
            root.1,
            vec![
                OsString::from("demo-local"),
                OsString::from("--from-check"),
                OsString::from("deployment-check.json")
            ]
        );
    }

    #[test]
    fn deploy_register_builds_minimal_registration_options() {
        let options = DeployRegisterOptions {
            deployment: "demo-local".to_string(),
            fleet_template: "demo".to_string(),
            root: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
            network: "local".to_string(),
            allow_unverified: true,
        }
        .into_register_options(Some(PathBuf::from("/tmp/icp")));

        assert_eq!(options.deployment_name, "demo-local");
        assert_eq!(options.fleet_template, "demo");
        assert_eq!(options.root_canister_id, "uxrrr-q7777-77774-qaaaq-cai");
        assert_eq!(options.network, "local");
        assert!(options.allow_unverified);
        assert_eq!(options.icp_root, Some(PathBuf::from("/tmp/icp")));
        assert_eq!(options.workspace_root, None);
    }

    #[test]
    fn deploy_register_requires_unverified_acknowledgement_flag() {
        let err = DeployRegisterOptions::parse([
            OsString::from("demo-local"),
            OsString::from("--fleet-template"),
            OsString::from("demo"),
            OsString::from("--root"),
            OsString::from("uxrrr-q7777-77774-qaaaq-cai"),
        ])
        .expect_err("register without acknowledgement should fail usage");

        std::assert_matches!(err, DeployCommandError::Usage(_));
    }

    #[test]
    fn deploy_compare_command_dispatches_compare() {
        let parsed = parse_subcommand(
            deploy_command(),
            [
                OsString::from("compare"),
                OsString::from("--left"),
                OsString::from("staging-check.json"),
                OsString::from("--right"),
                OsString::from("prod-check.json"),
            ],
        )
        .expect("parse deploy compare")
        .expect("compare command");

        assert_eq!(parsed.0, "compare");
        assert_eq!(
            parsed.1,
            vec![
                OsString::from("--left"),
                OsString::from("staging-check.json"),
                OsString::from("--right"),
                OsString::from("prod-check.json")
            ]
        );

        let options = DeployCompareOptions::parse(parsed.1).expect("parse compare options");
        assert_eq!(options.left, PathBuf::from("staging-check.json"));
        assert_eq!(options.right, PathBuf::from("prod-check.json"));
    }

    #[test]
    fn deploy_install_path_uses_current_install_with_plan_override() {
        let source = include_str!("mod.rs");
        let install_source = source_between(source, "fn run_install<I>", "fn run_promote<I>");

        assert!(install_source.contains("read_install_deployment_plan"));
        assert!(install_source.contains("into_install_root_options"));
        assert!(install_source.contains("install_root"));
        for forbidden in [
            "artifact_promotion_execution_receipt",
            "artifact_promotion_provenance_report",
            "build_artifact_promotion_plan",
            "run_promote",
        ] {
            assert!(
                !install_source.contains(forbidden),
                "deploy install path must stay current-install mediated; found forbidden token {forbidden}"
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
    fn root_verification_report_builder_delegates_to_host_report() {
        let report = build_root_verification_report(sample_root_verification_request())
            .expect("build root verification report");

        assert_eq!(
            report.evidence_status,
            DeploymentRootVerificationEvidenceStatusV1::EvidenceSatisfied
        );
        assert_eq!(
            report.state_transition,
            DeploymentRootVerificationStateTransitionV1::WouldPromoteNotVerifiedToVerified
        );
        assert_eq!(report.deployment_name, "demo");
        assert_eq!(report.source_check_id, "check-1");
        assert_eq!(report.source_inventory_id, "inventory-1");
        assert_eq!(report.report_digest.len(), 64);
    }

    #[test]
    fn external_lifecycle_plan_builder_uses_stable_local_ids() {
        let mut check = sample_authority_check();
        check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].control_class =
            CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

        let plan = build_external_lifecycle_plan(&check);

        assert_eq!(
            plan.lifecycle_plan_id,
            "local:local:demo:external-lifecycle-plan"
        );
        assert_eq!(
            plan.lifecycle_authority_report_id,
            "local:local:demo:lifecycle-authority-report"
        );
        assert_eq!(plan.deployment_plan_id, "plan-1");
        assert_eq!(plan.inventory_id, "inventory-1");
        assert_eq!(plan.proposed_external_role_upgrades.len(), 1);
        assert!(plan.directly_executable_role_upgrades.is_empty());
        assert_eq!(plan.lifecycle_plan_digest.len(), 64);
    }

    #[test]
    fn external_lifecycle_check_builder_links_pending_report() {
        let mut check = sample_authority_check();
        check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].control_class =
            CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

        let pending_report = build_external_lifecycle_pending_report(&check);
        let lifecycle_check = build_external_lifecycle_check(&check);

        assert_eq!(
            lifecycle_check.check_id,
            "local:local:demo:external-lifecycle-check"
        );
        assert_eq!(lifecycle_check.pending_report_id, pending_report.report_id);
        assert_eq!(
            lifecycle_check.pending_report_digest,
            pending_report.report_digest
        );
        assert_eq!(lifecycle_check.pending_external_count, 1);
        assert_eq!(lifecycle_check.direct_upgrade_count, 0);
        assert_eq!(lifecycle_check.blocked_count, 0);
        assert_eq!(lifecycle_check.check_digest.len(), 64);
    }

    #[test]
    fn external_lifecycle_handoff_builder_packages_pending_actions() {
        let mut check = sample_authority_check();
        check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].control_class =
            CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

        let lifecycle_check = build_external_lifecycle_check(&check);
        let handoff = build_external_lifecycle_handoff(&check);

        assert_eq!(
            handoff.handoff_id,
            "local:local:demo:external-lifecycle-handoff"
        );
        assert_eq!(handoff.lifecycle_check_id, lifecycle_check.check_id);
        assert_eq!(handoff.lifecycle_check_digest, lifecycle_check.check_digest);
        assert_eq!(handoff.handoff_actions.len(), 1);
        assert_eq!(
            handoff.handoff_actions[0].consent_channel_kind,
            ConsentChannelKindV1::GeneratedCommand
        );
        assert_eq!(handoff.handoff_digest.len(), 64);
    }

    #[test]
    fn external_proposal_report_builder_delegates_to_lifecycle_plan() {
        let mut check = sample_authority_check();
        check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].control_class =
            CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

        let lifecycle_plan = build_external_lifecycle_plan(&check);
        let report = build_external_upgrade_proposal_report(&check);

        assert_eq!(
            report.report_id,
            "local:local:demo:external-upgrade-proposals"
        );
        assert_eq!(report.report_digest.len(), 64);
        assert_eq!(report.lifecycle_plan_id, lifecycle_plan.lifecycle_plan_id);
        assert_eq!(
            report.lifecycle_plan_digest,
            lifecycle_plan.lifecycle_plan_digest
        );
        assert_eq!(report.deployment_plan_id, "plan-1");
        assert_eq!(report.inventory_id, "inventory-1");
        assert_eq!(report.proposals.len(), 1);
        assert_eq!(
            report.proposals[0].lifecycle_plan_digest,
            lifecycle_plan.lifecycle_plan_digest
        );
        assert_eq!(
            report.proposals[0].required_external_action,
            "external_controller_execution"
        );
    }

    #[test]
    fn external_pending_report_builder_links_plan_and_proposals() {
        let mut check = sample_authority_check();
        check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].control_class =
            CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

        let lifecycle_plan = build_external_lifecycle_plan(&check);
        let proposal_report = build_external_upgrade_proposal_report(&check);
        let pending_report = build_external_lifecycle_pending_report(&check);

        assert_eq!(
            pending_report.report_id,
            "local:local:demo:external-lifecycle-pending"
        );
        assert_eq!(
            pending_report.lifecycle_plan_id,
            lifecycle_plan.lifecycle_plan_id
        );
        assert_eq!(
            pending_report.lifecycle_plan_digest,
            lifecycle_plan.lifecycle_plan_digest
        );
        assert_eq!(pending_report.proposal_report_id, proposal_report.report_id);
        assert_eq!(
            pending_report.proposal_report_digest,
            proposal_report.report_digest
        );
        assert_eq!(pending_report.pending_external_count, 1);
        assert_eq!(pending_report.direct_upgrade_count, 0);
        assert_eq!(pending_report.blocked_count, 0);
        assert_eq!(
            pending_report.pending_external_actions[0].proposal_id,
            proposal_report.proposals[0].proposal_id
        );
        assert_eq!(pending_report.report_digest.len(), 64);
    }

    #[test]
    fn external_critical_fix_report_builder_links_pending_report() {
        let mut check = sample_authority_check();
        check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].control_class =
            CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

        let pending_report = build_external_lifecycle_pending_report(&check);
        let critical_fix = build_critical_external_fix_report(&check, "fix-2026-05", "critical");

        assert_eq!(
            critical_fix.report_id,
            "local:local:demo:critical-external-fix"
        );
        assert_eq!(critical_fix.fix_id, "fix-2026-05");
        assert_eq!(critical_fix.severity, "critical");
        assert_eq!(critical_fix.pending_report_id, pending_report.report_id);
        assert_eq!(
            critical_fix.pending_report_digest,
            pending_report.report_digest
        );
        assert_eq!(critical_fix.externally_blocked_roles, vec!["root"]);
        assert_eq!(critical_fix.required_external_actions.len(), 1);
        assert!(!critical_fix.residual_exposure.is_empty());
        assert_eq!(critical_fix.report_digest.len(), 64);
    }

    #[test]
    fn external_consent_evidence_builder_delegates_to_receipt_validation() {
        let mut check = sample_authority_check();
        check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].control_class =
            CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

        let proposal_report = build_external_upgrade_proposal_report(&check);
        let proposal = proposal_report.proposals[0].clone();
        let receipt = external_upgrade_receipt_from_observation(
            "external-upgrade-receipt-1",
            &proposal,
            ExternalUpgradeConsentStateV1::Pending,
            None,
            None,
        );
        let evidence =
            build_external_upgrade_consent_evidence(ExternalUpgradeConsentEvidenceRequest {
                evidence_id: "external-upgrade-consent-1".to_string(),
                proposal,
                receipt,
            })
            .expect("consent evidence should build");

        assert_eq!(evidence.evidence_id, "external-upgrade-consent-1");
        assert_eq!(evidence.receipt_id, "external-upgrade-receipt-1");
        assert!(!evidence.status_summary.is_empty());
        assert_eq!(evidence.evidence_digest.len(), 64);
    }

    #[test]
    fn external_verification_policy_builder_uses_proposal_requirements() {
        let mut check = sample_authority_check();
        check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].control_class =
            CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

        let proposal_report = build_external_upgrade_proposal_report(&check);
        let proposal = proposal_report.proposals[0].clone();
        let policy =
            build_external_upgrade_verification_policy(ExternalUpgradeVerificationPolicyRequest {
                policy_id: "external-upgrade-verification-policy-1".to_string(),
                proposal,
            });

        assert_eq!(policy.policy_id, "external-upgrade-verification-policy-1");
        assert_eq!(policy.verification_requirements.len(), 5);
        assert!(policy.verification_requirements.iter().any(|row| {
            row.requirement == LifecycleVerificationRequirementV1::LiveInventory
                && row.status == ExternalUpgradeVerificationRequirementStatusV1::Required
        }));
        assert!(!policy.status_summary.is_empty());
        assert_eq!(policy.policy_digest.len(), 64);
    }

    #[test]
    fn external_verification_check_builder_evaluates_supplied_observation() {
        let mut check = sample_authority_check();
        check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].control_class =
            CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

        let proposal_report = build_external_upgrade_proposal_report(&check);
        let proposal = proposal_report.proposals[0].clone();
        let policy =
            build_external_upgrade_verification_policy(ExternalUpgradeVerificationPolicyRequest {
                policy_id: "external-upgrade-verification-policy-1".to_string(),
                proposal: proposal.clone(),
            });
        let verification_check =
            build_external_upgrade_verification_check(ExternalUpgradeVerificationCheckRequest {
                check_id: "external-upgrade-verification-check-1".to_string(),
                policy,
                observation: Some(ExternalUpgradeVerificationObservationV1 {
                    source: ExternalVerificationObservationSourceV1::SuppliedObservation,
                    deployment_check_id: None,
                    deployment_check_digest: None,
                    inventory_id: Some("inventory-verified".to_string()),
                    observed_at: Some("2026-05-26T00:00:00Z".to_string()),
                    live_inventory_observed: true,
                    controller_observation_present: true,
                    observed_control_class: Some(proposal.control_class),
                    observed_module_hash: proposal.target_installed_module_hash,
                    observed_canonical_embedded_config_sha256: proposal
                        .target_canonical_embedded_config_sha256,
                    protected_call_ready: Some(true),
                }),
                deployment_check: None,
            })
            .expect("verification check should build");

        assert_eq!(
            verification_check.check_id,
            "external-upgrade-verification-check-1"
        );
        assert_eq!(
            verification_check.verification_result,
            ExternalUpgradeVerificationResultV1::Pending
        );
        assert_eq!(verification_check.check_digest.len(), 64);
    }

    #[test]
    fn external_verification_check_builder_verifies_deployment_truth_inventory() {
        let mut check = sample_authority_check();
        check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].control_class =
            CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
        check.inventory.observed_canisters[0].module_hash = Some("module".to_string());
        check.inventory.observed_canisters[0].canonical_embedded_config_digest =
            Some(sample_sha256("c"));
        check.inventory.observed_verifier_readiness.status = ObservationStatusV1::Observed;

        let proposal_report = build_external_upgrade_proposal_report(&check);
        let proposal = proposal_report.proposals[0].clone();
        let policy =
            build_external_upgrade_verification_policy(ExternalUpgradeVerificationPolicyRequest {
                policy_id: "external-upgrade-verification-policy-1".to_string(),
                proposal,
            });
        let verification_check =
            build_external_upgrade_verification_check(ExternalUpgradeVerificationCheckRequest {
                check_id: "external-upgrade-verification-check-1".to_string(),
                policy,
                observation: None,
                deployment_check: Some(check.clone()),
            })
            .expect("inventory-backed verification check should build");

        assert_eq!(
            verification_check.observation.source,
            ExternalVerificationObservationSourceV1::DeploymentTruthInventory
        );
        assert_eq!(
            verification_check
                .observation
                .deployment_check_id
                .as_deref(),
            Some(check.check_id.as_str())
        );
        assert!(
            verification_check.requirement_results.iter().all(|row| {
                row.status == ExternalUpgradeVerificationRequirementStatusV1::NotRequired
                    || row.satisfied == Some(true)
            }),
            "{:?}",
            verification_check.requirement_results
        );
        assert_eq!(
            verification_check.verification_result,
            ExternalUpgradeVerificationResultV1::Verified
        );
    }

    #[test]
    fn external_verification_check_builder_rejects_ambiguous_observation_sources() {
        let mut check = sample_authority_check();
        check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].control_class =
            CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

        let proposal_report = build_external_upgrade_proposal_report(&check);
        let proposal = proposal_report.proposals[0].clone();
        let policy =
            build_external_upgrade_verification_policy(ExternalUpgradeVerificationPolicyRequest {
                policy_id: "external-upgrade-verification-policy-1".to_string(),
                proposal: proposal.clone(),
            });
        let observation = ExternalUpgradeVerificationObservationV1 {
            source: ExternalVerificationObservationSourceV1::SuppliedObservation,
            deployment_check_id: None,
            deployment_check_digest: None,
            inventory_id: Some("inventory-verified".to_string()),
            observed_at: Some("2026-05-26T00:00:00Z".to_string()),
            live_inventory_observed: true,
            controller_observation_present: true,
            observed_control_class: Some(proposal.control_class),
            observed_module_hash: proposal.target_installed_module_hash,
            observed_canonical_embedded_config_sha256: proposal
                .target_canonical_embedded_config_sha256,
            protected_call_ready: Some(true),
        };

        let both_err =
            build_external_upgrade_verification_check(ExternalUpgradeVerificationCheckRequest {
                check_id: "external-upgrade-verification-check-1".to_string(),
                policy: policy.clone(),
                observation: Some(observation),
                deployment_check: Some(check.clone()),
            })
            .expect_err("both observation sources should be rejected");
        std::assert_matches!(both_err, DeployCommandError::Blocked(_));

        let neither_err =
            build_external_upgrade_verification_check(ExternalUpgradeVerificationCheckRequest {
                check_id: "external-upgrade-verification-check-1".to_string(),
                policy,
                observation: None,
                deployment_check: None,
            })
            .expect_err("missing observation source should be rejected");
        std::assert_matches!(neither_err, DeployCommandError::Blocked(_));
    }

    #[test]
    fn external_completion_report_builder_links_consent_and_verification() {
        let mut check = sample_authority_check();
        check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].control_class =
            CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

        let proposal_report = build_external_upgrade_proposal_report(&check);
        let proposal = proposal_report.proposals[0].clone();
        let receipt = external_upgrade_receipt_from_observation(
            "external-upgrade-receipt-1",
            &proposal,
            ExternalUpgradeConsentStateV1::Pending,
            None,
            None,
        );
        let consent_evidence =
            build_external_upgrade_consent_evidence(ExternalUpgradeConsentEvidenceRequest {
                evidence_id: "external-upgrade-consent-1".to_string(),
                proposal: proposal.clone(),
                receipt,
            })
            .expect("consent evidence should build");
        let policy =
            build_external_upgrade_verification_policy(ExternalUpgradeVerificationPolicyRequest {
                policy_id: "external-upgrade-verification-policy-1".to_string(),
                proposal: proposal.clone(),
            });
        let verification_check =
            build_external_upgrade_verification_check(ExternalUpgradeVerificationCheckRequest {
                check_id: "external-upgrade-verification-check-1".to_string(),
                policy,
                observation: Some(ExternalUpgradeVerificationObservationV1 {
                    source: ExternalVerificationObservationSourceV1::SuppliedObservation,
                    deployment_check_id: None,
                    deployment_check_digest: None,
                    inventory_id: Some("inventory-verified".to_string()),
                    observed_at: Some("2026-05-26T00:00:00Z".to_string()),
                    live_inventory_observed: true,
                    controller_observation_present: true,
                    observed_control_class: Some(proposal.control_class),
                    observed_module_hash: proposal.target_installed_module_hash.clone(),
                    observed_canonical_embedded_config_sha256: proposal
                        .target_canonical_embedded_config_sha256
                        .clone(),
                    protected_call_ready: Some(true),
                }),
                deployment_check: None,
            })
            .expect("verification check should build");

        let completion =
            build_external_upgrade_completion_report(ExternalUpgradeCompletionReportRequest {
                report_id: "external-upgrade-completion-1".to_string(),
                proposal,
                consent_evidence,
                verification_check,
            })
            .expect("completion report should build");

        assert_eq!(completion.report_id, "external-upgrade-completion-1");
        assert_eq!(
            completion.completion_status,
            ExternalUpgradeCompletionStatusV1::AwaitingConsent
        );
        assert_eq!(completion.report_digest.len(), 64);
    }

    #[test]
    fn external_verification_report_builder_delegates_to_receipt_validation() {
        let mut check = sample_authority_check();
        check.plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].control_class =
            CanisterControlClassV1::UserControlled;
        check.inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];

        let proposal_report = build_external_upgrade_proposal_report(&check);
        let proposal = proposal_report.proposals[0].clone();
        let receipt = external_upgrade_receipt_from_observation(
            "external-upgrade-receipt-1",
            &proposal,
            ExternalUpgradeConsentStateV1::Pending,
            None,
            None,
        );
        let report =
            build_external_upgrade_verification_report(ExternalUpgradeVerificationReportRequest {
                report_id: "external-upgrade-verification-1".to_string(),
                proposal,
                receipt,
            })
            .expect("verification report should build");

        assert_eq!(report.report_id, "external-upgrade-verification-1");
        assert_eq!(report.receipt_id, "external-upgrade-receipt-1");
        assert!(!report.status_summary.is_empty());
        assert_eq!(report.report_digest.len(), 64);
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

        std::assert_matches!(
            result,
            Err(DeployCommandError::Usage(message))
                if message.contains("invalid authority output format: csv")
        );
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

        std::assert_matches!(
            result,
            Err(DeployCommandError::Usage(message))
                if message.contains("invalid authority output format: xml")
        );
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

        std::assert_matches!(
            result,
            Err(DeployCommandError::Usage(message))
                if message.contains("invalid authority output format: yaml")
        );
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

        std::assert_matches!(
            result,
            Err(DeployCommandError::Usage(message))
                if message.contains("invalid authority output format: toml")
        );
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

        std::assert_matches!(
            result,
            Err(DeployCommandError::Usage(message))
                if message.contains("invalid promotion output format: csv")
        );
    }

    #[test]
    fn external_plan_rejects_unknown_format() {
        let result = DeployExternalOptions::parse(
            [
                OsString::from("--format"),
                OsString::from("yaml"),
                OsString::from("demo"),
            ],
            deploy_external_plan_command,
            external_plan_usage,
        );

        std::assert_matches!(
            result,
            Err(DeployCommandError::Usage(message))
                if message.contains("invalid external lifecycle output format: yaml")
        );
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

        assert_eq!(resume_report.truth.deployment, "demo");
        assert_eq!(resume_report.receipt, None);
    }

    #[test]
    fn deploy_check_builds_current_install_options() {
        let options = DeployTruthOptions {
            deployment: "demo".to_string(),
            network: "local".to_string(),
            profile: Some(CanisterBuildProfile::Fast),
        }
        .into_install_root_options_with_icp_root(Some(std::path::PathBuf::from("/tmp/icp")));

        assert_eq!(options.root_canister, "root");
        assert_eq!(options.root_build_target, "root");
        assert_eq!(options.network, "local");
        assert_eq!(options.build_profile, Some(CanisterBuildProfile::Fast));
        assert_eq!(options.deployment_name.as_deref(), Some("demo"));
        assert_eq!(options.config_path, None);
        assert_eq!(options.expected_fleet, None);
    }

    #[test]
    fn deploy_install_plan_builds_current_install_options_with_plan_override() {
        let mut identity = sample_deployment_identity();
        identity.deployment_name = "demo-local".to_string();
        let plan = sample_deployment_plan(identity);
        let input = DeployInstallPlanInput {
            deployment_plan: plan,
            artifact_promotion_plan: None,
        };
        let options = DeployInstallPlanOptions {
            deployment: "demo-local".to_string(),
            plan: PathBuf::from("promoted-plan.json"),
            network: "local".to_string(),
            profile: Some(CanisterBuildProfile::Fast),
        }
        .into_install_root_options(input, Some(std::path::PathBuf::from("/tmp/icp")));

        assert_eq!(options.root_canister, "aaaaa-aa");
        assert_eq!(options.root_build_target, "root");
        assert_eq!(options.network, "local");
        assert_eq!(options.deployment_name.as_deref(), Some("demo-local"));
        assert_eq!(options.build_profile, Some(CanisterBuildProfile::Fast));
        assert_eq!(
            options.config_path.as_deref(),
            Some("fleets/demo/canic.toml")
        );
        assert_eq!(options.expected_fleet.as_deref(), Some("demo"));
        assert!(options.deployment_plan_override.is_some());
    }

    #[test]
    fn deploy_install_plan_reader_accepts_raw_deployment_plan() {
        let path = temp_json_path("deploy-install-raw-plan.json");
        let plan = sample_deployment_plan(sample_deployment_identity());
        fs::write(&path, serde_json::to_vec(&plan).expect("encode plan")).expect("write plan");

        let decoded = read_install_deployment_plan(&path).expect("decode deployment plan");

        assert_eq!(decoded.deployment_plan.plan_id, "plan-1");
        assert_eq!(decoded.artifact_promotion_plan, None);
        fs::remove_file(path).expect("clean temp plan");
    }

    #[test]
    fn deploy_install_plan_reader_accepts_ready_promotion_envelope() {
        let path = temp_json_path("deploy-install-ready-promotion-plan.json");
        let plan = sample_artifact_promotion_plan();
        fs::write(&path, serde_json::to_vec(&plan).expect("encode plan")).expect("write plan");

        let decoded = read_install_deployment_plan(&path).expect("decode promotion plan");

        assert_eq!(decoded.deployment_plan.plan_id, "promoted-plan-1");
        assert_eq!(
            decoded
                .artifact_promotion_plan
                .as_ref()
                .map(|plan| plan.plan_id.as_str()),
            Some("artifact-promotion-plan-1")
        );
        fs::remove_file(path).expect("clean temp plan");
    }

    #[test]
    fn deploy_install_plan_reader_rejects_blocked_promotion_envelope() {
        let path = temp_json_path("deploy-install-blocked-promotion-plan.json");
        let plan = sample_blocked_artifact_promotion_plan();
        fs::write(&path, serde_json::to_vec(&plan).expect("encode plan")).expect("write plan");

        let result = read_install_deployment_plan(&path);

        std::assert_matches!(
            result,
            Err(DeployCommandError::Blocked(message))
                if message.contains("artifact promotion plan artifact-promotion-plan-1 is not ready")
        );
        fs::remove_file(path).expect("clean temp plan");
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

    fn sample_root_verification_request() -> DeploymentRootVerificationRequestV1 {
        let mut check = sample_authority_check();
        check.inventory.observed_root = Some(DeploymentRootObservationV1 {
            deployment_name: "demo".to_string(),
            network: "local".to_string(),
            fleet_template: "demo".to_string(),
            root_principal: "aaaaa-aa".to_string(),
            observed_canister_id: "aaaaa-aa".to_string(),
            observation_source: DeploymentRootObservationSourceV1::IcpCanisterStatus,
            control_class: CanisterControlClassV1::DeploymentControlled,
            controllers: vec!["aaaaa-aa".to_string()],
            module_hash: None,
            status: Some("running".to_string()),
            role_assignment_source: Some("icp_canister_status".to_string()),
        });
        check.inventory.observed_artifacts = vec![ObservedArtifactV1 {
            role: "root".to_string(),
            artifact_path: "artifacts/root.wasm.gz".to_string(),
            file_sha256: Some(sample_sha256("a")),
            file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            payload_sha256: None,
            payload_size_bytes: Some(123),
            source: ArtifactSourceV1::LocalBuild,
        }];
        if let Some(root) = check.inventory.observed_canisters.first_mut() {
            root.module_hash = Some("module".to_string());
            root.canonical_embedded_config_digest = Some(sample_sha256("c"));
        }
        check.diff = compare_plan_to_inventory(&check.plan, &check.inventory);
        check.report = safety_report_from_diff(
            &check.report.report_id,
            check.report.diff_id.clone(),
            &check.diff,
        );
        DeploymentRootVerificationRequestV1 {
            report_id: "root-verification-report-1".to_string(),
            requested_at: "2026-05-27T00:00:00Z".to_string(),
            deployment_name: "demo".to_string(),
            network: "local".to_string(),
            expected_fleet_template: "demo".to_string(),
            expected_root_principal: "aaaaa-aa".to_string(),
            current_root_verification: DeploymentRootVerificationStateV1::NotVerified,
            source: DeploymentRootVerificationSourceV1::DeploymentTruthCheck,
            deployment_check: check,
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
            role_artifacts: vec![sample_role_artifact()],
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

    fn sample_artifact_promotion_plan() -> ArtifactPromotionPlanV1 {
        sample_artifact_promotion_plan_for_input(sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        ))
    }

    fn sample_blocked_artifact_promotion_plan() -> ArtifactPromotionPlanV1 {
        let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
        input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("e"));
        sample_artifact_promotion_plan_for_inputs(
            input,
            sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm),
        )
    }

    fn sample_artifact_promotion_plan_for_input(
        input: RolePromotionInputV1,
    ) -> ArtifactPromotionPlanV1 {
        sample_artifact_promotion_plan_for_inputs(input.clone(), input)
    }

    fn sample_artifact_promotion_plan_for_inputs(
        report_input: RolePromotionInputV1,
        transform_input: RolePromotionInputV1,
    ) -> ArtifactPromotionPlanV1 {
        let target_plan = sample_deployment_plan(sample_deployment_identity());
        let readiness = promotion_readiness_from_inputs(
            "promotion-readiness-1",
            &target_plan,
            std::slice::from_ref(&report_input),
        );
        let artifact_identity_report = promotion_artifact_identity_report_from_inputs(
            PromotionArtifactIdentityReportRequest {
                report_id: "promotion-artifact-identity-1".to_string(),
                inputs: vec![report_input],
            },
        )
        .expect("sample artifact identity report");
        let transform =
            promoted_deployment_plan_transform_from_inputs(&PromotionPlanTransformRequest {
                promoted_plan_id: "promoted-plan-1".to_string(),
                target_plan,
                inputs: vec![transform_input],
            })
            .expect("sample transform");

        artifact_promotion_plan(ArtifactPromotionPlanRequest {
            plan_id: "artifact-promotion-plan-1".to_string(),
            generated_at: "2026-05-26T00:00:00Z".to_string(),
            readiness,
            artifact_identity_report,
            transform,
            target_execution_lineage: None,
        })
        .expect("sample artifact promotion plan")
    }

    fn sample_role_promotion_input(
        promotion_level: PromotionArtifactLevelV1,
    ) -> RolePromotionInputV1 {
        RolePromotionInputV1 {
            role: "root".to_string(),
            promotion_level,
            source: sample_role_artifact_source(RoleArtifactSourceKindV1::LocalWasmGz),
            require_byte_identical_wasm: promotion_level == PromotionArtifactLevelV1::SealedWasm,
            require_target_embedded_config: true,
            target_store_has_artifact: Some(true),
        }
    }

    fn sample_role_artifact_source(kind: RoleArtifactSourceKindV1) -> RoleArtifactSourceV1 {
        RoleArtifactSourceV1 {
            role: "root".to_string(),
            kind,
            locator: Some("artifacts/root.wasm.gz".to_string()),
            previous_receipt_kind: (kind == RoleArtifactSourceKindV1::PreviousReceiptArtifact)
                .then_some(PreviousArtifactReceiptKindV1::DeploymentReceipt),
            previous_receipt_lineage_digest: (kind
                == RoleArtifactSourceKindV1::PreviousReceiptArtifact)
                .then(|| sample_sha256("9")),
            expected_wasm_sha256: Some(sample_sha256("d")),
            expected_wasm_gz_sha256: Some(sample_sha256("a")),
            expected_candid_sha256: Some(sample_sha256("b")),
            expected_canonical_embedded_config_sha256: Some(sample_sha256("c")),
        }
    }

    fn sample_role_artifact() -> RoleArtifactV1 {
        RoleArtifactV1 {
            role: "root".to_string(),
            source: ArtifactSourceV1::LocalBuild,
            build_profile: "fast".to_string(),
            wasm_path: Some("artifacts/root.wasm".to_string()),
            wasm_gz_path: Some("artifacts/root.wasm.gz".to_string()),
            wasm_gz_size_bytes: Some(123),
            wasm_sha256: Some(sample_sha256("d")),
            wasm_gz_sha256: Some(sample_sha256("a")),
            wasm_gz_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            observed_wasm_gz_file_sha256: Some(sample_sha256("a")),
            observed_wasm_gz_file_sha256_source: Some(ArtifactDigestSourceV1::ObservedFileDigest),
            installed_module_hash: Some("module".to_string()),
            candid_path: Some("root.did".to_string()),
            candid_sha256: Some(sample_sha256("b")),
            raw_config_sha256: Some("raw".to_string()),
            canonical_embedded_config_sha256: Some(sample_sha256("c")),
            embedded_topology_sha256: Some("topology".to_string()),
            builder_version: Some("0.44.0".to_string()),
            rust_toolchain: Some("stable".to_string()),
            package_version: Some("0.44.0".to_string()),
        }
    }

    fn sample_sha256(seed: &str) -> String {
        seed.repeat(64)
    }

    fn temp_json_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "canic-cli-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ))
    }

    fn sample_deployment_inventory(identity: DeploymentIdentityV1) -> DeploymentInventoryV1 {
        DeploymentInventoryV1 {
            schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            inventory_id: "inventory-1".to_string(),
            observed_at: "2026-05-23T00:00:00Z".to_string(),
            observed_identity: Some(identity),
            observed_root: None,
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
