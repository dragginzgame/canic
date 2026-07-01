use super::authority::AUTHORITY_UNSAFE_BLOCKED_CODE;
use super::{
    AuthorityReconciliationPlanV1, AuthorityReconciliationStateV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION,
    DeploymentCheckV1, DeploymentExecutionContextV1, DeploymentExecutionPreflightStatusV1,
    DeploymentExecutionPreflightV1, DeploymentExecutorBackendV1, DeploymentExecutorCapabilityV1,
    DeploymentPlanV1, SafetyFindingV1, SafetyReportV1, SafetySeverityV1, SafetyStatusV1,
    build_authority_reconciliation_plan,
};
use std::collections::BTreeSet;
use thiserror::Error as ThisError;

pub(in crate::deployment_truth) const DEPLOYMENT_SAFETY_BLOCKED_CODE: &str =
    "deployment_safety_blocked";
pub(in crate::deployment_truth) const AUTHORITY_CONTROLLER_CHANGE_PENDING_CODE: &str =
    "authority_controller_change_pending";
pub(in crate::deployment_truth) const AUTHORITY_EXTERNAL_ACTION_REQUIRED_CODE: &str =
    "authority_external_action_required";
pub(in crate::deployment_truth) const AUTHORITY_OBSERVATION_MISSING_CODE: &str =
    "authority_observation_missing";
pub(in crate::deployment_truth) const EXECUTOR_CAPABILITY_MISSING_CODE: &str =
    "executor_capability_missing";

///
/// DeploymentExecutionPreflightError
///
#[derive(Debug, ThisError)]
pub enum DeploymentExecutionPreflightError {
    #[error("deployment execution preflight schema mismatch: expected {expected}, found {found}")]
    SchemaVersionMismatch { expected: u32, found: u32 },
    #[error("deployment execution preflight is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error(
        "deployment execution preflight status {status:?} does not match blocker count {blocker_count}"
    )]
    StatusBlockerMismatch {
        status: DeploymentExecutionPreflightStatusV1,
        blocker_count: usize,
    },
    #[error(
        "deployment execution preflight contains duplicate capability in {field}: {capability:?}"
    )]
    DuplicateCapability {
        field: &'static str,
        capability: DeploymentExecutorCapabilityV1,
    },
    #[error(
        "deployment execution preflight reports missing capability that was not required: {capability:?}"
    )]
    MissingCapabilityNotRequired {
        capability: DeploymentExecutorCapabilityV1,
    },
    #[error("deployment execution preflight missing capability has no blocker: {capability:?}")]
    MissingCapabilityWithoutBlocker {
        capability: DeploymentExecutorCapabilityV1,
    },
    #[error(
        "deployment execution preflight {field} does not match source check: preflight={preflight_value}, check={check_value}"
    )]
    SourceCheckMismatch {
        field: &'static str,
        preflight_value: String,
        check_value: String,
    },
}

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

///
/// TestkitPreflightContext
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestkitPreflightContext {
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

impl TestkitPreflightContext {
    #[must_use]
    pub fn new(artifact_roots: Vec<String>) -> Self {
        Self {
            context: testkit_execution_context(artifact_roots),
        }
    }
}

impl DeploymentExecutor for CurrentCliDeploymentExecutor {
    fn execution_context(&self) -> DeploymentExecutionContextV1 {
        self.context.clone()
    }
}

impl DeploymentExecutor for TestkitPreflightContext {
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

pub const TESTKIT_PREFLIGHT_CAPABILITIES: &[DeploymentExecutorCapabilityV1] =
    CURRENT_CLI_EXECUTOR_CAPABILITIES;

pub const CURRENT_INSTALL_EXECUTION_PHASES: &[&str] = &[
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
pub fn testkit_execution_context(artifact_roots: Vec<String>) -> DeploymentExecutionContextV1 {
    DeploymentExecutionContextV1 {
        workspace_root: None,
        icp_root: None,
        artifact_roots,
        backend: DeploymentExecutorBackendV1::PocketIc,
        backend_capabilities: TESTKIT_PREFLIGHT_CAPABILITIES.to_vec(),
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
    deployment_execution_preflight_with_unknown_authority_policy(
        &check.plan,
        &check.report,
        &authority_plan,
        executor,
        required_capabilities,
        allow_initial_install_unknown_authority(check),
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
    deployment_execution_preflight_with_unknown_authority_policy(
        plan,
        safety_report,
        authority_plan,
        executor,
        required_capabilities,
        false,
    )
}

fn deployment_execution_preflight_with_unknown_authority_policy(
    plan: &DeploymentPlanV1,
    safety_report: &SafetyReportV1,
    authority_plan: &AuthorityReconciliationPlanV1,
    executor: &impl DeploymentExecutor,
    required_capabilities: &[DeploymentExecutorCapabilityV1],
    allow_unknown_authority: bool,
) -> DeploymentExecutionPreflightV1 {
    let execution_context = executor.execution_context();
    let missing_capabilities = missing_executor_capabilities(
        &execution_context.backend_capabilities,
        required_capabilities,
    );
    let blockers = deployment_execution_blockers(
        safety_report,
        authority_plan,
        &missing_capabilities,
        allow_unknown_authority,
    );
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

pub fn validate_deployment_execution_preflight(
    preflight: &DeploymentExecutionPreflightV1,
) -> Result<(), DeploymentExecutionPreflightError> {
    if preflight.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(DeploymentExecutionPreflightError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found: preflight.schema_version,
        });
    }

    ensure_preflight_field("plan_id", &preflight.plan_id)?;
    ensure_preflight_field("safety_report_id", &preflight.safety_report_id)?;
    ensure_preflight_field("authority_plan_id", &preflight.authority_plan_id)?;
    ensure_preflight_status_matches_blockers(preflight)?;
    ensure_unique_capabilities("required_capabilities", &preflight.required_capabilities)?;
    ensure_unique_capabilities("missing_capabilities", &preflight.missing_capabilities)?;
    ensure_missing_capabilities_are_required(preflight)?;
    ensure_missing_capabilities_have_blockers(preflight)?;

    Ok(())
}

pub fn validate_deployment_execution_preflight_for_check(
    check: &DeploymentCheckV1,
    preflight: &DeploymentExecutionPreflightV1,
) -> Result<(), DeploymentExecutionPreflightError> {
    validate_deployment_execution_preflight(preflight)?;
    ensure_preflight_check_match("plan_id", &preflight.plan_id, &check.plan.plan_id)?;
    ensure_preflight_check_match(
        "safety_report_id",
        &preflight.safety_report_id,
        &check.report.report_id,
    )?;

    let authority_plan = build_authority_reconciliation_plan(check);
    ensure_preflight_check_match(
        "authority_plan_id",
        &preflight.authority_plan_id,
        &authority_plan.plan_id,
    )?;

    Ok(())
}

fn deployment_execution_blockers(
    safety_report: &SafetyReportV1,
    authority_plan: &AuthorityReconciliationPlanV1,
    missing_capabilities: &[DeploymentExecutorCapabilityV1],
    allow_unknown_authority: bool,
) -> Vec<SafetyFindingV1> {
    let mut blockers = Vec::new();

    if matches!(safety_report.status, SafetyStatusV1::Blocked) {
        blockers.push(SafetyFindingV1 {
            code: DEPLOYMENT_SAFETY_BLOCKED_CODE.to_string(),
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
            .filter(|failure| failure.code != AUTHORITY_UNSAFE_BLOCKED_CODE)
            .cloned(),
    );

    for action in &authority_plan.canister_actions {
        match action.state {
            AuthorityReconciliationStateV1::AlreadyCorrect => {}
            AuthorityReconciliationStateV1::CanApplyAutomatically => {
                blockers.push(SafetyFindingV1 {
                    code: AUTHORITY_CONTROLLER_CHANGE_PENDING_CODE.to_string(),
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
                    code: AUTHORITY_EXTERNAL_ACTION_REQUIRED_CODE.to_string(),
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
                    code: AUTHORITY_UNSAFE_BLOCKED_CODE.to_string(),
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
                if allow_unknown_authority {
                    continue;
                }
                blockers.push(SafetyFindingV1 {
                    code: AUTHORITY_OBSERVATION_MISSING_CODE.to_string(),
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
            code: EXECUTOR_CAPABILITY_MISSING_CODE.to_string(),
            message: format!("executor backend is missing required capability: {capability:?}"),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(format!("{capability:?}")),
        });
    }

    blockers
}

fn allow_initial_install_unknown_authority(check: &DeploymentCheckV1) -> bool {
    check.plan.unresolved_assumptions.iter().any(|assumption| {
        assumption.key == "local_state.root_canister_id"
            && assumption
                .description
                .contains("no local deployment state exists")
    })
}

fn ensure_preflight_field(
    field: &'static str,
    value: &str,
) -> Result<(), DeploymentExecutionPreflightError> {
    if value.trim().is_empty() {
        return Err(DeploymentExecutionPreflightError::MissingRequiredField { field });
    }
    Ok(())
}

const fn ensure_preflight_status_matches_blockers(
    preflight: &DeploymentExecutionPreflightV1,
) -> Result<(), DeploymentExecutionPreflightError> {
    let blocker_count = preflight.blockers.len();
    let matches_blockers = match preflight.status {
        DeploymentExecutionPreflightStatusV1::Ready => blocker_count == 0,
        DeploymentExecutionPreflightStatusV1::Blocked => blocker_count > 0,
    };
    if !matches_blockers {
        return Err(DeploymentExecutionPreflightError::StatusBlockerMismatch {
            status: preflight.status,
            blocker_count,
        });
    }
    Ok(())
}

fn ensure_unique_capabilities(
    field: &'static str,
    capabilities: &[DeploymentExecutorCapabilityV1],
) -> Result<(), DeploymentExecutionPreflightError> {
    let mut seen = BTreeSet::new();
    for capability in capabilities {
        if !seen.insert(*capability) {
            return Err(DeploymentExecutionPreflightError::DuplicateCapability {
                field,
                capability: *capability,
            });
        }
    }
    Ok(())
}

fn ensure_missing_capabilities_are_required(
    preflight: &DeploymentExecutionPreflightV1,
) -> Result<(), DeploymentExecutionPreflightError> {
    let required = preflight
        .required_capabilities
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    for capability in &preflight.missing_capabilities {
        if !required.contains(capability) {
            return Err(
                DeploymentExecutionPreflightError::MissingCapabilityNotRequired {
                    capability: *capability,
                },
            );
        }
    }
    Ok(())
}

fn ensure_missing_capabilities_have_blockers(
    preflight: &DeploymentExecutionPreflightV1,
) -> Result<(), DeploymentExecutionPreflightError> {
    for capability in &preflight.missing_capabilities {
        let subject = format!("{capability:?}");
        if !preflight.blockers.iter().any(|finding| {
            finding.code == EXECUTOR_CAPABILITY_MISSING_CODE
                && finding.subject.as_deref() == Some(subject.as_str())
        }) {
            return Err(
                DeploymentExecutionPreflightError::MissingCapabilityWithoutBlocker {
                    capability: *capability,
                },
            );
        }
    }
    Ok(())
}

fn ensure_preflight_check_match(
    field: &'static str,
    preflight_value: &str,
    check_value: &str,
) -> Result<(), DeploymentExecutionPreflightError> {
    if preflight_value != check_value {
        return Err(DeploymentExecutionPreflightError::SourceCheckMismatch {
            field,
            preflight_value: preflight_value.to_string(),
            check_value: check_value.to_string(),
        });
    }
    Ok(())
}
