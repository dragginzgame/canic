use super::super::*;
use crate::deployment_truth::authority::AUTHORITY_UNSAFE_BLOCKED_CODE;
use crate::deployment_truth::executor::{
    DEPLOYMENT_SAFETY_BLOCKED_CODE, EXECUTOR_CAPABILITY_MISSING_CODE,
};

const DEPLOYMENT_ARTIFACT_MISSING_CODE: &str = "deployment_artifact_missing";

#[test]
fn deployment_execution_preflight_accepts_safe_plan_and_capable_executor() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let authority = build_authority_reconciliation_plan(&check);
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );

    let preflight = deployment_execution_preflight(
        &check.plan,
        &check.report,
        &authority,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );

    assert_eq!(preflight.plan_id, check.plan.plan_id);
    assert_eq!(preflight.safety_report_id, check.report.report_id);
    assert_eq!(preflight.authority_plan_id, authority.plan_id);
    assert_eq!(
        preflight.status,
        DeploymentExecutionPreflightStatusV1::Ready
    );
    assert!(preflight.blockers.is_empty());
    assert!(preflight.missing_capabilities.is_empty());
    assert_eq!(
        preflight.planned_phases,
        vec![
            "resolve_root_canister",
            "build_artifacts",
            "materialize_artifacts",
            "execution_preflight",
            "emit_manifest",
            "install_root",
            "fund_root_pre_bootstrap",
            "stage_release_set",
            "resume_bootstrap",
            "wait_ready",
            "fund_root_post_ready",
            "write_install_state",
        ]
    );
    assert_json_round_trip(&preflight);
}

#[test]
fn deployment_execution_preflight_from_check_derives_authority_plan() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );

    let from_check = deployment_execution_preflight_from_check(
        &check,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );
    let authority = build_authority_reconciliation_plan(&check);
    let explicit = deployment_execution_preflight(
        &check.plan,
        &check.report,
        &authority,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );

    assert_eq!(from_check, explicit);
}

#[test]
fn deployment_execution_preflight_validation_accepts_check_derived_artifact() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );
    let preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );

    validate_deployment_execution_preflight(&preflight).expect("preflight should validate");
    validate_deployment_execution_preflight_for_check(&check, &preflight)
        .expect("preflight should match source check");
}

#[test]
fn deployment_execution_preflight_validation_rejects_mutated_status() {
    let check = sample_unknown_unsafe_check();
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );
    let mut preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );
    preflight.status = DeploymentExecutionPreflightStatusV1::Ready;

    let err = validate_deployment_execution_preflight(&preflight)
        .expect_err("ready status with blockers should fail");

    std::assert_matches!(
        err,
        DeploymentExecutionPreflightError::StatusBlockerMismatch { .. }
    );
}

#[test]
fn deployment_execution_preflight_validation_rejects_source_check_mismatch() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );
    let mut preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );
    preflight.plan_id = "other-plan".to_string();

    let err = validate_deployment_execution_preflight_for_check(&check, &preflight)
        .expect_err("preflight from another plan should fail");

    std::assert_matches!(
        err,
        DeploymentExecutionPreflightError::SourceCheckMismatch {
            field: "plan_id",
            ..
        }
    );
}

#[test]
fn deployment_execution_preflight_validation_rejects_capability_inconsistency() {
    let check = sample_unknown_unsafe_check();
    let executor = LimitedExecutor {
        context: DeploymentExecutionContextV1 {
            workspace_root: Some("/workspace/canic".to_string()),
            icp_root: Some("/workspace/canic/.icp".to_string()),
            artifact_roots: Vec::new(),
            backend: DeploymentExecutorBackendV1::Other {
                name: "limited-test-backend".to_string(),
            },
            backend_capabilities: vec![DeploymentExecutorCapabilityV1::CanisterStatus],
        },
    };
    let mut preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        &[DeploymentExecutorCapabilityV1::CanisterStatus],
    );
    preflight
        .missing_capabilities
        .push(DeploymentExecutorCapabilityV1::InstallCode);

    let err = validate_deployment_execution_preflight(&preflight)
        .expect_err("missing non-required capability should fail");

    std::assert_matches!(
        err,
        DeploymentExecutionPreflightError::MissingCapabilityNotRequired {
            capability: DeploymentExecutorCapabilityV1::InstallCode
        }
    );
}

#[test]
fn deployment_execution_preflight_v1_json_schema_shape_is_stable() {
    let check = sample_unknown_unsafe_check();
    let executor = LimitedExecutor {
        context: DeploymentExecutionContextV1 {
            workspace_root: Some("/workspace/canic".to_string()),
            icp_root: Some("/workspace/canic/.icp".to_string()),
            artifact_roots: Vec::new(),
            backend: DeploymentExecutorBackendV1::Other {
                name: "limited-test-backend".to_string(),
            },
            backend_capabilities: vec![DeploymentExecutorCapabilityV1::CanisterStatus],
        },
    };
    let preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        &[
            DeploymentExecutorCapabilityV1::CanisterStatus,
            DeploymentExecutorCapabilityV1::StageArtifact,
        ],
    );
    let value = serde_json::to_value(&preflight).expect("encode execution preflight");

    assert_object_keys(
        &value,
        &[
            "schema_version",
            "plan_id",
            "safety_report_id",
            "authority_plan_id",
            "backend",
            "status",
            "planned_phases",
            "required_capabilities",
            "missing_capabilities",
            "blockers",
        ],
    );
    assert_eq!(value["schema_version"], DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(value["plan_id"], "plan-local-root");
    assert_eq!(value["safety_report_id"], "report-1");
    assert_eq!(value["authority_plan_id"], "plan-local-root");
    assert_eq!(value["backend"]["Other"]["name"], "limited-test-backend");
    assert_eq!(value["status"], "Blocked");
    assert_eq!(value["required_capabilities"][0], "CanisterStatus");
    assert_eq!(value["required_capabilities"][1], "StageArtifact");
    assert_eq!(value["missing_capabilities"][0], "StageArtifact");
    assert_eq!(
        value["blockers"]
            .as_array()
            .expect("blockers should be array")
            .iter()
            .filter(|finding| finding["code"] == EXECUTOR_CAPABILITY_MISSING_CODE)
            .count(),
        1
    );
}

#[test]
fn deployment_execution_preflight_text_reports_passive_readiness() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let executor = CurrentCliDeploymentExecutor::new(
        Some("/workspace/canic".to_string()),
        Some("/workspace/canic/.icp".to_string()),
        vec!["/workspace/canic/.icp/local/canisters".to_string()],
    );
    let preflight = deployment_execution_preflight_from_check(
        &check,
        &executor,
        CURRENT_CLI_EXECUTOR_CAPABILITIES,
    );

    let text = deployment_execution_preflight_text(&preflight);

    assert!(text.contains("Deployment execution preflight"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("status: ready"));
    assert!(text.contains("plan_id: plan-local-root"));
    assert!(text.contains("backend: CurrentCli"));
    assert!(text.contains("planned_phases:"));
    assert!(text.contains("  - install_root"));
    assert!(text.contains("required_capabilities:"));
    assert!(text.contains("  - StageArtifact"));
}

#[test]
fn deployment_execution_preflight_blocks_safety_authority_and_capability_gaps() {
    let mut check = sample_unknown_unsafe_check();
    check.report.status = SafetyStatusV1::Blocked;
    check.report.hard_failures.push(SafetyFindingV1 {
        code: DEPLOYMENT_ARTIFACT_MISSING_CODE.to_string(),
        message: "planned artifact was not observed".to_string(),
        severity: SafetySeverityV1::HardFailure,
        subject: Some("root".to_string()),
    });
    let authority = build_authority_reconciliation_plan(&check);
    let executor = LimitedExecutor {
        context: DeploymentExecutionContextV1 {
            workspace_root: Some("/workspace/canic".to_string()),
            icp_root: Some("/workspace/canic/.icp".to_string()),
            artifact_roots: Vec::new(),
            backend: DeploymentExecutorBackendV1::Other {
                name: "limited-test-backend".to_string(),
            },
            backend_capabilities: vec![DeploymentExecutorCapabilityV1::CanisterStatus],
        },
    };

    let preflight = deployment_execution_preflight(
        &check.plan,
        &check.report,
        &authority,
        &executor,
        &[
            DeploymentExecutorCapabilityV1::CanisterStatus,
            DeploymentExecutorCapabilityV1::StageArtifact,
        ],
    );

    assert_eq!(
        preflight.status,
        DeploymentExecutionPreflightStatusV1::Blocked
    );
    assert_eq!(
        preflight.missing_capabilities,
        vec![DeploymentExecutorCapabilityV1::StageArtifact]
    );
    assert!(preflight.blockers.iter().any(|finding| {
        finding.code == DEPLOYMENT_SAFETY_BLOCKED_CODE
            && finding.subject.as_deref() == Some("report-1")
    }));
    assert!(
        preflight
            .blockers
            .iter()
            .any(|finding| finding.code == DEPLOYMENT_ARTIFACT_MISSING_CODE)
    );
    assert!(
        preflight
            .blockers
            .iter()
            .any(|finding| finding.code == AUTHORITY_UNSAFE_BLOCKED_CODE)
    );
    assert!(preflight.blockers.iter().any(|finding| {
        finding.code == EXECUTOR_CAPABILITY_MISSING_CODE
            && finding.subject.as_deref() == Some("StageArtifact")
    }));
}
