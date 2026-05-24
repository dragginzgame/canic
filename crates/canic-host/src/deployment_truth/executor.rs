use super::{
    AuthorityReconciliationPlanV1, AuthorityReconciliationStateV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION,
    DeploymentCheckV1, DeploymentExecutionContextV1, DeploymentExecutionPreflightStatusV1,
    DeploymentExecutionPreflightV1, DeploymentExecutorBackendV1, DeploymentExecutorCapabilityV1,
    DeploymentPlanV1, SafetyFindingV1, SafetyReportV1, SafetySeverityV1, SafetyStatusV1,
    build_authority_reconciliation_plan,
};
use std::collections::BTreeSet;

///
/// DeploymentExecutor
///
pub trait DeploymentExecutor {
    fn execution_context(&self) -> DeploymentExecutionContextV1;
}

///
/// CurrentCliDeploymentExecutor
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentCliDeploymentExecutor {
    context: DeploymentExecutionContextV1,
}

impl CurrentCliDeploymentExecutor {
    #[must_use]
    pub fn new(
        workspace_root: Option<String>,
        icp_root: Option<String>,
        artifact_roots: Vec<String>,
    ) -> Self {
        Self {
            context: current_cli_execution_context(workspace_root, icp_root, artifact_roots),
        }
    }
}

impl DeploymentExecutor for CurrentCliDeploymentExecutor {
    fn execution_context(&self) -> DeploymentExecutionContextV1 {
        self.context.clone()
    }
}

pub const CURRENT_CLI_EXECUTOR_CAPABILITIES: &[DeploymentExecutorCapabilityV1] = &[
    DeploymentExecutorCapabilityV1::CreateCanister,
    DeploymentExecutorCapabilityV1::CanisterStatus,
    DeploymentExecutorCapabilityV1::UpdateSettings,
    DeploymentExecutorCapabilityV1::InstallCode,
    DeploymentExecutorCapabilityV1::Call,
    DeploymentExecutorCapabilityV1::Query,
    DeploymentExecutorCapabilityV1::StageArtifact,
];

pub const CURRENT_INSTALL_EXECUTION_PHASES: &[&str] = &[
    "create_root",
    "build_artifacts",
    "materialize_artifacts",
    "install_root",
    "stage_release_set",
    "resume_bootstrap",
    "wait_ready",
    "post_validate",
];

#[must_use]
pub fn current_cli_execution_context(
    workspace_root: Option<String>,
    icp_root: Option<String>,
    artifact_roots: Vec<String>,
) -> DeploymentExecutionContextV1 {
    DeploymentExecutionContextV1 {
        workspace_root,
        icp_root,
        artifact_roots,
        backend: DeploymentExecutorBackendV1::CurrentCli,
        backend_capabilities: CURRENT_CLI_EXECUTOR_CAPABILITIES.to_vec(),
    }
}

#[must_use]
pub fn missing_executor_capabilities(
    available: &[DeploymentExecutorCapabilityV1],
    required: &[DeploymentExecutorCapabilityV1],
) -> Vec<DeploymentExecutorCapabilityV1> {
    let available = available.iter().copied().collect::<BTreeSet<_>>();
    required
        .iter()
        .copied()
        .filter(|capability| !available.contains(capability))
        .collect()
}

#[must_use]
pub fn has_executor_capabilities(
    available: &[DeploymentExecutorCapabilityV1],
    required: &[DeploymentExecutorCapabilityV1],
) -> bool {
    missing_executor_capabilities(available, required).is_empty()
}

#[must_use]
pub fn deployment_execution_preflight_from_check(
    check: &DeploymentCheckV1,
    executor: &impl DeploymentExecutor,
    required_capabilities: &[DeploymentExecutorCapabilityV1],
) -> DeploymentExecutionPreflightV1 {
    let authority_plan = build_authority_reconciliation_plan(check);
    deployment_execution_preflight(
        &check.plan,
        &check.report,
        &authority_plan,
        executor,
        required_capabilities,
    )
}

#[must_use]
pub fn deployment_execution_preflight(
    plan: &DeploymentPlanV1,
    safety_report: &SafetyReportV1,
    authority_plan: &AuthorityReconciliationPlanV1,
    executor: &impl DeploymentExecutor,
    required_capabilities: &[DeploymentExecutorCapabilityV1],
) -> DeploymentExecutionPreflightV1 {
    let execution_context = executor.execution_context();
    let missing_capabilities = missing_executor_capabilities(
        &execution_context.backend_capabilities,
        required_capabilities,
    );
    let blockers =
        deployment_execution_blockers(safety_report, authority_plan, &missing_capabilities);
    let status = if blockers.is_empty() {
        DeploymentExecutionPreflightStatusV1::Ready
    } else {
        DeploymentExecutionPreflightStatusV1::Blocked
    };

    DeploymentExecutionPreflightV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        plan_id: plan.plan_id.clone(),
        safety_report_id: safety_report.report_id.clone(),
        authority_plan_id: authority_plan.plan_id.clone(),
        backend: execution_context.backend,
        status,
        planned_phases: CURRENT_INSTALL_EXECUTION_PHASES
            .iter()
            .map(|phase| (*phase).to_string())
            .collect(),
        required_capabilities: required_capabilities.to_vec(),
        missing_capabilities,
        blockers,
    }
}

fn deployment_execution_blockers(
    safety_report: &SafetyReportV1,
    authority_plan: &AuthorityReconciliationPlanV1,
    missing_capabilities: &[DeploymentExecutorCapabilityV1],
) -> Vec<SafetyFindingV1> {
    let mut blockers = Vec::new();

    if matches!(safety_report.status, SafetyStatusV1::Blocked) {
        blockers.push(SafetyFindingV1 {
            code: "deployment_safety_blocked".to_string(),
            message: safety_report.summary.clone(),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(safety_report.report_id.clone()),
        });
    }
    blockers.extend(safety_report.hard_failures.clone());
    blockers.extend(
        authority_plan
            .hard_failures
            .iter()
            .filter(|failure| failure.code != "authority_unsafe_blocked")
            .cloned(),
    );

    for action in &authority_plan.canister_actions {
        match action.state {
            AuthorityReconciliationStateV1::AlreadyCorrect => {}
            AuthorityReconciliationStateV1::CanApplyAutomatically => {
                blockers.push(SafetyFindingV1 {
                    code: "authority_controller_change_pending".to_string(),
                    message: action.reason.clone(),
                    severity: SafetySeverityV1::HardFailure,
                    subject: action
                        .canister_id
                        .clone()
                        .or_else(|| action.role.clone())
                        .or_else(|| Some("authority".to_string())),
                });
            }
            AuthorityReconciliationStateV1::RequiresExternalAction => {
                blockers.push(SafetyFindingV1 {
                    code: "authority_external_action_required".to_string(),
                    message: action.reason.clone(),
                    severity: SafetySeverityV1::HardFailure,
                    subject: action
                        .canister_id
                        .clone()
                        .or_else(|| action.role.clone())
                        .or_else(|| Some("authority".to_string())),
                });
            }
            AuthorityReconciliationStateV1::UnsafeBlocked => {
                blockers.push(SafetyFindingV1 {
                    code: "authority_unsafe_blocked".to_string(),
                    message: action.reason.clone(),
                    severity: SafetySeverityV1::HardFailure,
                    subject: action
                        .canister_id
                        .clone()
                        .or_else(|| action.role.clone())
                        .or_else(|| Some("authority".to_string())),
                });
            }
            AuthorityReconciliationStateV1::Unknown => {
                blockers.push(SafetyFindingV1 {
                    code: "authority_observation_missing".to_string(),
                    message: action.reason.clone(),
                    severity: SafetySeverityV1::HardFailure,
                    subject: action
                        .canister_id
                        .clone()
                        .or_else(|| action.role.clone())
                        .or_else(|| Some("authority".to_string())),
                });
            }
        }
    }

    for capability in missing_capabilities {
        blockers.push(SafetyFindingV1 {
            code: "executor_capability_missing".to_string(),
            message: format!("executor backend is missing required capability: {capability:?}"),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(format!("{capability:?}")),
        });
    }

    blockers
}
