use super::inventory::ObservationStatusV1;
use super::safety::SafetyFindingV1;
use serde::{Deserialize, Serialize};

///
/// DeploymentReceiptV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentReceiptV1 {
    pub schema_version: u32,
    pub operation_id: String,
    pub plan_id: String,
    pub execution_context: Option<DeploymentExecutionContextV1>,
    pub operation_status: DeploymentExecutionStatusV1,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub operator_principal: Option<String>,
    pub root_principal: Option<String>,
    pub previous_observed_deployment_epoch: Option<u64>,
    pub phase_receipts: Vec<PhaseReceiptV1>,
    pub role_phase_receipts: Vec<RolePhaseReceiptV1>,
    pub final_inventory_id: Option<String>,
    pub command_result: DeploymentCommandResultV1,
}

///
/// DeploymentExecutionContextV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentExecutionContextV1 {
    pub workspace_root: Option<String>,
    pub icp_root: Option<String>,
    pub artifact_roots: Vec<String>,
    pub backend: DeploymentExecutorBackendV1,
    pub backend_capabilities: Vec<DeploymentExecutorCapabilityV1>,
}

///
/// DeploymentExecutionPreflightV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentExecutionPreflightV1 {
    pub schema_version: u32,
    pub plan_id: String,
    pub safety_report_id: String,
    pub authority_plan_id: String,
    pub backend: DeploymentExecutorBackendV1,
    pub status: DeploymentExecutionPreflightStatusV1,
    pub planned_phases: Vec<String>,
    pub required_capabilities: Vec<DeploymentExecutorCapabilityV1>,
    pub missing_capabilities: Vec<DeploymentExecutorCapabilityV1>,
    pub blockers: Vec<SafetyFindingV1>,
}

///
/// DeploymentExecutionPreflightStatusV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DeploymentExecutionPreflightStatusV1 {
    Ready,
    Blocked,
}

impl DeploymentExecutionPreflightStatusV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Blocked => "blocked",
        }
    }
}

///
/// DeploymentExecutorBackendV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DeploymentExecutorBackendV1 {
    CurrentCli,
    PocketIc,
    DirectAgent,
    Other { name: String },
}

///
/// DeploymentExecutorCapabilityV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum DeploymentExecutorCapabilityV1 {
    CreateCanister,
    CanisterStatus,
    UpdateSettings,
    InstallCode,
    Call,
    Query,
    StageArtifact,
}

///
/// PhaseReceiptV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PhaseReceiptV1 {
    pub phase: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub attempted_action: String,
    pub verified_postcondition: VerifiedPostconditionV1,
}

///
/// VerifiedPostconditionV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerifiedPostconditionV1 {
    pub status: ObservationStatusV1,
    pub evidence: Vec<String>,
}

///
/// DeploymentExecutionStatusV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DeploymentExecutionStatusV1 {
    NotStarted,
    InProgress,
    FailedBeforeMutation,
    PartiallyApplied,
    FailedAfterMutation,
    Complete,
}

impl DeploymentExecutionStatusV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::InProgress => "in_progress",
            Self::FailedBeforeMutation => "failed_before_mutation",
            Self::PartiallyApplied => "partially_applied",
            Self::FailedAfterMutation => "failed_after_mutation",
            Self::Complete => "complete",
        }
    }
}

///
/// DeploymentCommandResultV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DeploymentCommandResultV1 {
    NotFinished,
    Succeeded,
    Failed { code: String, message: String },
}

///
/// RolePhaseReceiptV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePhaseReceiptV1 {
    pub role: String,
    pub phase: String,
    pub result: RolePhaseResultV1,
    pub previous_module_hash: Option<String>,
    pub target_module_hash: Option<String>,
    pub observed_module_hash_after: Option<String>,
    pub artifact_digest: Option<String>,
    pub canonical_embedded_config_sha256: Option<String>,
    pub error: Option<String>,
}

///
/// RolePhaseResultV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RolePhaseResultV1 {
    Applied,
    Failed,
    Skipped,
    NotAttempted,
    VerifiedAlreadyApplied,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deployment_execution_preflight_status_owns_text_labels() {
        assert_eq!(DeploymentExecutionPreflightStatusV1::Ready.label(), "ready");
        assert_eq!(
            DeploymentExecutionPreflightStatusV1::Blocked.label(),
            "blocked"
        );
    }

    #[test]
    fn deployment_execution_status_owns_text_labels() {
        assert_eq!(
            DeploymentExecutionStatusV1::NotStarted.label(),
            "not_started"
        );
        assert_eq!(
            DeploymentExecutionStatusV1::InProgress.label(),
            "in_progress"
        );
        assert_eq!(
            DeploymentExecutionStatusV1::FailedBeforeMutation.label(),
            "failed_before_mutation"
        );
        assert_eq!(
            DeploymentExecutionStatusV1::PartiallyApplied.label(),
            "partially_applied"
        );
        assert_eq!(
            DeploymentExecutionStatusV1::FailedAfterMutation.label(),
            "failed_after_mutation"
        );
        assert_eq!(DeploymentExecutionStatusV1::Complete.label(), "complete");
    }
}
