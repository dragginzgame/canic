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

    #[must_use]
    pub const fn variant_label(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::Blocked => "Blocked",
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

impl DeploymentExecutorBackendV1 {
    #[must_use]
    pub fn variant_label(&self) -> String {
        match self {
            Self::CurrentCli => "CurrentCli".to_string(),
            Self::PocketIc => "PocketIc".to_string(),
            Self::DirectAgent => "DirectAgent".to_string(),
            Self::Other { name } => format!("Other {{ name: {name:?} }}"),
        }
    }
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

    #[must_use]
    pub const fn variant_label(self) -> &'static str {
        match self {
            Self::NotStarted => "NotStarted",
            Self::InProgress => "InProgress",
            Self::FailedBeforeMutation => "FailedBeforeMutation",
            Self::PartiallyApplied => "PartiallyApplied",
            Self::FailedAfterMutation => "FailedAfterMutation",
            Self::Complete => "Complete",
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

impl DeploymentCommandResultV1 {
    #[must_use]
    pub fn variant_label(&self) -> String {
        match self {
            Self::NotFinished => "NotFinished".to_string(),
            Self::Succeeded => "Succeeded".to_string(),
            Self::Failed { code, message } => {
                format!("Failed {{ code: {code:?}, message: {message:?} }}")
            }
        }
    }
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

impl RolePhaseResultV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Applied => "Applied",
            Self::Failed => "Failed",
            Self::Skipped => "Skipped",
            Self::NotAttempted => "NotAttempted",
            Self::VerifiedAlreadyApplied => "VerifiedAlreadyApplied",
        }
    }
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
        assert_eq!(
            DeploymentExecutionPreflightStatusV1::Ready.variant_label(),
            "Ready"
        );
        assert_eq!(
            DeploymentExecutionPreflightStatusV1::Blocked.variant_label(),
            "Blocked"
        );
    }

    #[test]
    fn deployment_executor_backend_owns_variant_labels() {
        assert_eq!(
            DeploymentExecutorBackendV1::CurrentCli.variant_label(),
            "CurrentCli"
        );
        assert_eq!(
            DeploymentExecutorBackendV1::PocketIc.variant_label(),
            "PocketIc"
        );
        assert_eq!(
            DeploymentExecutorBackendV1::DirectAgent.variant_label(),
            "DirectAgent"
        );
        assert_eq!(
            DeploymentExecutorBackendV1::Other {
                name: "fixture".to_string()
            }
            .variant_label(),
            "Other { name: \"fixture\" }"
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
        assert_eq!(
            DeploymentExecutionStatusV1::NotStarted.variant_label(),
            "NotStarted"
        );
        assert_eq!(
            DeploymentExecutionStatusV1::InProgress.variant_label(),
            "InProgress"
        );
        assert_eq!(
            DeploymentExecutionStatusV1::FailedBeforeMutation.variant_label(),
            "FailedBeforeMutation"
        );
        assert_eq!(
            DeploymentExecutionStatusV1::PartiallyApplied.variant_label(),
            "PartiallyApplied"
        );
        assert_eq!(
            DeploymentExecutionStatusV1::FailedAfterMutation.variant_label(),
            "FailedAfterMutation"
        );
        assert_eq!(
            DeploymentExecutionStatusV1::Complete.variant_label(),
            "Complete"
        );
    }

    #[test]
    fn deployment_command_result_owns_variant_labels() {
        assert_eq!(
            DeploymentCommandResultV1::NotFinished.variant_label(),
            "NotFinished"
        );
        assert_eq!(
            DeploymentCommandResultV1::Succeeded.variant_label(),
            "Succeeded"
        );
        assert_eq!(
            DeploymentCommandResultV1::Failed {
                code: "install_failed".to_string(),
                message: "install \"root\" failed".to_string(),
            }
            .variant_label(),
            "Failed { code: \"install_failed\", message: \"install \\\"root\\\" failed\" }"
        );
    }

    #[test]
    fn role_phase_result_owns_text_labels() {
        assert_eq!(RolePhaseResultV1::Applied.label(), "Applied");
        assert_eq!(RolePhaseResultV1::Failed.label(), "Failed");
        assert_eq!(RolePhaseResultV1::Skipped.label(), "Skipped");
        assert_eq!(RolePhaseResultV1::NotAttempted.label(), "NotAttempted");
        assert_eq!(
            RolePhaseResultV1::VerifiedAlreadyApplied.label(),
            "VerifiedAlreadyApplied"
        );
    }
}
