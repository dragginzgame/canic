use super::super::CanisterControlClassV1;
use serde::{Deserialize, Serialize};

///
/// LifecycleAuthorityReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LifecycleAuthorityReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub report_digest: String,
    pub check_id: String,
    pub plan_id: String,
    pub inventory_id: String,
    pub authorities: Vec<LifecycleAuthorityV1>,
    pub external_action_required_count: usize,
    pub blocked_count: usize,
}

///
/// LifecycleAuthorityV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LifecycleAuthorityV1 {
    pub subject: String,
    pub canister_id: Option<String>,
    pub role: Option<String>,
    pub control_class: CanisterControlClassV1,
    pub lifecycle_mode: LifecycleModeV1,
    pub observed_controllers: Vec<String>,
    pub expected_deployment_controllers: Vec<String>,
    pub external_controllers: Vec<String>,
    pub required_controllers: Vec<String>,
    pub consent_requirements: Vec<ConsentRequirementV1>,
    pub allowed_upgrade_modes: Vec<LifecycleUpgradeModeV1>,
    pub verification_requirements: Vec<LifecycleVerificationRequirementV1>,
    pub external_action_required: bool,
    pub blocked: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub reason: String,
}

///
/// LifecycleModeV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum LifecycleModeV1 {
    DirectDeploymentAuthority,
    ProposalRequired,
    DelegatedInstallRequired,
    ExternalCompletionOnly,
    VerifyOnly,
    MustNotTouch,
    UnknownUnsafeBlocked,
}

impl LifecycleModeV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::DirectDeploymentAuthority => "direct_deployment_authority",
            Self::ProposalRequired => "proposal_required",
            Self::DelegatedInstallRequired => "delegated_install_required",
            Self::ExternalCompletionOnly => "external_completion_only",
            Self::VerifyOnly => "verify_only",
            Self::MustNotTouch => "must_not_touch",
            Self::UnknownUnsafeBlocked => "unknown_unsafe_blocked",
        }
    }
}

///
/// LifecycleUpgradeModeV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum LifecycleUpgradeModeV1 {
    DirectByDeploymentAuthority,
    ExternalProposal,
    ExternalExecution,
    VerifyExternalCompletion,
    ObserveOnly,
    Blocked,
}

///
/// LifecycleVerificationRequirementV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum LifecycleVerificationRequirementV1 {
    LiveInventory,
    ControllerObservation,
    ModuleHash,
    CanonicalEmbeddedConfig,
    ProtectedCallReadiness,
}

impl LifecycleVerificationRequirementV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::LiveInventory => "live_inventory",
            Self::ControllerObservation => "controller_observation",
            Self::ModuleHash => "module_hash",
            Self::CanonicalEmbeddedConfig => "canonical_embedded_config",
            Self::ProtectedCallReadiness => "protected_call_readiness",
        }
    }
}

///
/// ConsentRequirementV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConsentRequirementV1 {
    pub consent_subject_kind: ConsentSubjectKindV1,
    pub required_principals: Vec<String>,
    pub required_controller_set_digest: Option<String>,
    pub consent_channel_kind: ConsentChannelKindV1,
    pub required_action: ExternalUpgradeAuthorizationModeV1,
}

///
/// ConsentSubjectKindV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ConsentSubjectKindV1 {
    UserPrincipal,
    ProjectHub,
    GovernanceCanister,
    CustomerController,
    DelegatedInstallCanister,
    MultisigAuthority,
    UnknownExternalController,
}

impl ConsentSubjectKindV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::UserPrincipal => "user_principal",
            Self::ProjectHub => "project_hub",
            Self::GovernanceCanister => "governance_canister",
            Self::CustomerController => "customer_controller",
            Self::DelegatedInstallCanister => "delegated_install_canister",
            Self::MultisigAuthority => "multisig_authority",
            Self::UnknownExternalController => "unknown_external_controller",
        }
    }
}

///
/// ConsentChannelKindV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ConsentChannelKindV1 {
    OutOfBand,
    GeneratedCommand,
    DelegatedInstall,
    GovernanceProposal,
    ApplicationSpecific,
}

impl ConsentChannelKindV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::OutOfBand => "out_of_band",
            Self::GeneratedCommand => "generated_command",
            Self::DelegatedInstall => "delegated_install",
            Self::GovernanceProposal => "governance_proposal",
            Self::ApplicationSpecific => "application_specific",
        }
    }
}

///
/// ExternalUpgradeAuthorizationModeV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ExternalUpgradeAuthorizationModeV1 {
    ConsentForDirectInstall,
    DelegatedInstallAuthority,
    ExternalControllerExecution,
    ObserveAndVerifyOnly,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_mode_owns_text_labels() {
        assert_eq!(
            LifecycleModeV1::DirectDeploymentAuthority.label(),
            "direct_deployment_authority"
        );
        assert_eq!(
            LifecycleModeV1::ProposalRequired.label(),
            "proposal_required"
        );
        assert_eq!(
            LifecycleModeV1::DelegatedInstallRequired.label(),
            "delegated_install_required"
        );
        assert_eq!(
            LifecycleModeV1::ExternalCompletionOnly.label(),
            "external_completion_only"
        );
        assert_eq!(LifecycleModeV1::VerifyOnly.label(), "verify_only");
        assert_eq!(LifecycleModeV1::MustNotTouch.label(), "must_not_touch");
        assert_eq!(
            LifecycleModeV1::UnknownUnsafeBlocked.label(),
            "unknown_unsafe_blocked"
        );
    }

    #[test]
    fn lifecycle_verification_requirement_owns_text_labels() {
        assert_eq!(
            LifecycleVerificationRequirementV1::LiveInventory.label(),
            "live_inventory"
        );
        assert_eq!(
            LifecycleVerificationRequirementV1::ControllerObservation.label(),
            "controller_observation"
        );
        assert_eq!(
            LifecycleVerificationRequirementV1::ModuleHash.label(),
            "module_hash"
        );
        assert_eq!(
            LifecycleVerificationRequirementV1::CanonicalEmbeddedConfig.label(),
            "canonical_embedded_config"
        );
        assert_eq!(
            LifecycleVerificationRequirementV1::ProtectedCallReadiness.label(),
            "protected_call_readiness"
        );
    }

    #[test]
    fn consent_subject_kind_owns_text_labels() {
        assert_eq!(
            ConsentSubjectKindV1::UserPrincipal.label(),
            "user_principal"
        );
        assert_eq!(ConsentSubjectKindV1::ProjectHub.label(), "project_hub");
        assert_eq!(
            ConsentSubjectKindV1::GovernanceCanister.label(),
            "governance_canister"
        );
        assert_eq!(
            ConsentSubjectKindV1::CustomerController.label(),
            "customer_controller"
        );
        assert_eq!(
            ConsentSubjectKindV1::DelegatedInstallCanister.label(),
            "delegated_install_canister"
        );
        assert_eq!(
            ConsentSubjectKindV1::MultisigAuthority.label(),
            "multisig_authority"
        );
        assert_eq!(
            ConsentSubjectKindV1::UnknownExternalController.label(),
            "unknown_external_controller"
        );
    }

    #[test]
    fn consent_channel_kind_owns_text_labels() {
        assert_eq!(ConsentChannelKindV1::OutOfBand.label(), "out_of_band");
        assert_eq!(
            ConsentChannelKindV1::GeneratedCommand.label(),
            "generated_command"
        );
        assert_eq!(
            ConsentChannelKindV1::DelegatedInstall.label(),
            "delegated_install"
        );
        assert_eq!(
            ConsentChannelKindV1::GovernanceProposal.label(),
            "governance_proposal"
        );
        assert_eq!(
            ConsentChannelKindV1::ApplicationSpecific.label(),
            "application_specific"
        );
    }
}
