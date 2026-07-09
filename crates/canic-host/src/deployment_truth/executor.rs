use super::authority::AUTHORITY_UNSAFE_BLOCKED_CODE;
use super::{
    AuthorityReconciliationPlanV1, AuthorityReconciliationStateV1, CanisterAuthorityActionV1,
    DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentCheckV1, DeploymentExecutionContextV1,
    DeploymentExecutionPreflightStatusV1, DeploymentExecutionPreflightV1,
    DeploymentExecutorBackendV1, DeploymentExecutorCapabilityV1, DeploymentPlanV1, SafetyFindingV1,
    SafetyReportV1, SafetySeverityV1, SafetyStatusV1, build_authority_reconciliation_plan,
};
use std::collections::BTreeSet;
use thiserror::Error as ThisError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DeploymentExecutionPreflightBlockerCode(&'static str);

impl DeploymentExecutionPreflightBlockerCode {
    const AUTHORITY_CONTROLLER_CHANGE_PENDING: Self = Self("authority_controller_change_pending");
    const AUTHORITY_EXTERNAL_ACTION_REQUIRED: Self = Self("authority_external_action_required");
    const AUTHORITY_OBSERVATION_MISSING: Self = Self("authority_observation_missing");
    const DEPLOYMENT_SAFETY_BLOCKED: Self = Self("deployment_safety_blocked");
    const EXECUTOR_CAPABILITY_MISSING: Self = Self("executor_capability_missing");

    #[must_use]
    const fn as_str(self) -> &'static str {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DeploymentExecutionPreflightSubjectLabel(&'static str);

impl DeploymentExecutionPreflightSubjectLabel {
    const AUTHORITY: Self = Self("authority");

    #[must_use]
    const fn as_str(self) -> &'static str {
        self.0
    }
}

pub(in crate::deployment_truth) const DEPLOYMENT_SAFETY_BLOCKED_CODE: &str =
    DeploymentExecutionPreflightBlockerCode::DEPLOYMENT_SAFETY_BLOCKED.as_str();
pub(in crate::deployment_truth) const EXECUTOR_CAPABILITY_MISSING_CODE: &str =
    DeploymentExecutionPreflightBlockerCode::EXECUTOR_CAPABILITY_MISSING.as_str();

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CurrentInstallExecutionPhaseLabel(&'static str);

impl CurrentInstallExecutionPhaseLabel {
    const BUILD_ARTIFACTS: Self = Self("build_artifacts");
    const EMIT_MANIFEST: Self = Self("emit_manifest");
    const EXECUTION_PREFLIGHT: Self = Self("execution_preflight");
    const FUND_ROOT_POST_READY: Self = Self("fund_root_post_ready");
    const FUND_ROOT_PRE_BOOTSTRAP: Self = Self("fund_root_pre_bootstrap");
    const INSTALL_ROOT: Self = Self("install_root");
    const MATERIALIZE_ARTIFACTS: Self = Self("materialize_artifacts");
    const RESOLVE_ROOT_CANISTER: Self = Self("resolve_root_canister");
    const RESUME_BOOTSTRAP: Self = Self("resume_bootstrap");
    const STAGE_RELEASE_SET: Self = Self("stage_release_set");
    const WAIT_READY: Self = Self("wait_ready");
    const WRITE_INSTALL_STATE: Self = Self("write_install_state");

    #[must_use]
    const fn as_str(self) -> &'static str {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DeploymentExecutionPreflightFieldLabel(&'static str);

impl DeploymentExecutionPreflightFieldLabel {
    const AUTHORITY_PLAN_ID: Self = Self("authority_plan_id");
    const MISSING_CAPABILITIES: Self = Self("missing_capabilities");
    const PLAN_ID: Self = Self("plan_id");
    const REQUIRED_CAPABILITIES: Self = Self("required_capabilities");
    const SAFETY_REPORT_ID: Self = Self("safety_report_id");

    #[must_use]
    const fn as_str(self) -> &'static str {
        self.0
    }
}

const CURRENT_INSTALL_EXECUTION_PHASES: &[CurrentInstallExecutionPhaseLabel] = &[
    CurrentInstallExecutionPhaseLabel::RESOLVE_ROOT_CANISTER,
    CurrentInstallExecutionPhaseLabel::BUILD_ARTIFACTS,
    CurrentInstallExecutionPhaseLabel::MATERIALIZE_ARTIFACTS,
    CurrentInstallExecutionPhaseLabel::EXECUTION_PREFLIGHT,
    CurrentInstallExecutionPhaseLabel::EMIT_MANIFEST,
    CurrentInstallExecutionPhaseLabel::INSTALL_ROOT,
    CurrentInstallExecutionPhaseLabel::FUND_ROOT_PRE_BOOTSTRAP,
    CurrentInstallExecutionPhaseLabel::STAGE_RELEASE_SET,
    CurrentInstallExecutionPhaseLabel::RESUME_BOOTSTRAP,
    CurrentInstallExecutionPhaseLabel::WAIT_READY,
    CurrentInstallExecutionPhaseLabel::FUND_ROOT_POST_READY,
    CurrentInstallExecutionPhaseLabel::WRITE_INSTALL_STATE,
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
            .map(|phase| phase.as_str().to_string())
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

    ensure_preflight_field(
        DeploymentExecutionPreflightFieldLabel::PLAN_ID,
        &preflight.plan_id,
    )?;
    ensure_preflight_field(
        DeploymentExecutionPreflightFieldLabel::SAFETY_REPORT_ID,
        &preflight.safety_report_id,
    )?;
    ensure_preflight_field(
        DeploymentExecutionPreflightFieldLabel::AUTHORITY_PLAN_ID,
        &preflight.authority_plan_id,
    )?;
    ensure_preflight_status_matches_blockers(preflight)?;
    ensure_unique_capabilities(
        DeploymentExecutionPreflightFieldLabel::REQUIRED_CAPABILITIES,
        &preflight.required_capabilities,
    )?;
    ensure_unique_capabilities(
        DeploymentExecutionPreflightFieldLabel::MISSING_CAPABILITIES,
        &preflight.missing_capabilities,
    )?;
    ensure_missing_capabilities_are_required(preflight)?;
    ensure_missing_capabilities_have_blockers(preflight)?;

    Ok(())
}

pub fn validate_deployment_execution_preflight_for_check(
    check: &DeploymentCheckV1,
    preflight: &DeploymentExecutionPreflightV1,
) -> Result<(), DeploymentExecutionPreflightError> {
    validate_deployment_execution_preflight(preflight)?;
    ensure_preflight_check_match(
        DeploymentExecutionPreflightFieldLabel::PLAN_ID,
        &preflight.plan_id,
        &check.plan.plan_id,
    )?;
    ensure_preflight_check_match(
        DeploymentExecutionPreflightFieldLabel::SAFETY_REPORT_ID,
        &preflight.safety_report_id,
        &check.report.report_id,
    )?;

    let authority_plan = build_authority_reconciliation_plan(check);
    ensure_preflight_check_match(
        DeploymentExecutionPreflightFieldLabel::AUTHORITY_PLAN_ID,
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
                    code:
                        DeploymentExecutionPreflightBlockerCode::AUTHORITY_CONTROLLER_CHANGE_PENDING
                            .as_str()
                            .to_string(),
                    message: action.reason.clone(),
                    severity: SafetySeverityV1::HardFailure,
                    subject: Some(authority_blocker_subject(action)),
                });
            }
            AuthorityReconciliationStateV1::RequiresExternalAction => {
                blockers.push(SafetyFindingV1 {
                    code:
                        DeploymentExecutionPreflightBlockerCode::AUTHORITY_EXTERNAL_ACTION_REQUIRED
                            .as_str()
                            .to_string(),
                    message: action.reason.clone(),
                    severity: SafetySeverityV1::HardFailure,
                    subject: Some(authority_blocker_subject(action)),
                });
            }
            AuthorityReconciliationStateV1::UnsafeBlocked => {
                blockers.push(SafetyFindingV1 {
                    code: AUTHORITY_UNSAFE_BLOCKED_CODE.to_string(),
                    message: action.reason.clone(),
                    severity: SafetySeverityV1::HardFailure,
                    subject: Some(authority_blocker_subject(action)),
                });
            }
            AuthorityReconciliationStateV1::Unknown => {
                if allow_unknown_authority {
                    continue;
                }
                blockers.push(SafetyFindingV1 {
                    code: DeploymentExecutionPreflightBlockerCode::AUTHORITY_OBSERVATION_MISSING
                        .as_str()
                        .to_string(),
                    message: action.reason.clone(),
                    severity: SafetySeverityV1::HardFailure,
                    subject: Some(authority_blocker_subject(action)),
                });
            }
        }
    }

    for capability in missing_capabilities {
        blockers.push(SafetyFindingV1 {
            code: DeploymentExecutionPreflightBlockerCode::EXECUTOR_CAPABILITY_MISSING
                .as_str()
                .to_string(),
            message: format!("executor backend is missing required capability: {capability:?}"),
            severity: SafetySeverityV1::HardFailure,
            subject: Some(format!("{capability:?}")),
        });
    }

    blockers
}

fn authority_blocker_subject(action: &CanisterAuthorityActionV1) -> String {
    action
        .canister_id
        .clone()
        .or_else(|| action.role.clone())
        .unwrap_or_else(|| {
            DeploymentExecutionPreflightSubjectLabel::AUTHORITY
                .as_str()
                .to_string()
        })
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
    field: DeploymentExecutionPreflightFieldLabel,
    value: &str,
) -> Result<(), DeploymentExecutionPreflightError> {
    if value.trim().is_empty() {
        return Err(DeploymentExecutionPreflightError::MissingRequiredField {
            field: field.as_str(),
        });
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
    field: DeploymentExecutionPreflightFieldLabel,
    capabilities: &[DeploymentExecutorCapabilityV1],
) -> Result<(), DeploymentExecutionPreflightError> {
    let mut seen = BTreeSet::new();
    for capability in capabilities {
        if !seen.insert(*capability) {
            return Err(DeploymentExecutionPreflightError::DuplicateCapability {
                field: field.as_str(),
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
    field: DeploymentExecutionPreflightFieldLabel,
    preflight_value: &str,
    check_value: &str,
) -> Result<(), DeploymentExecutionPreflightError> {
    if preflight_value != check_value {
        return Err(DeploymentExecutionPreflightError::SourceCheckMismatch {
            field: field.as_str(),
            preflight_value: preflight_value.to_string(),
            check_value: check_value.to_string(),
        });
    }
    Ok(())
}
