use super::super::super::stable_json_sha256_hex;
use crate::deployment_truth::{
    CanisterControlClassV1, ConsentChannelKindV1, ConsentRequirementV1, ConsentSubjectKindV1,
    ExternalUpgradeAuthorizationModeV1, LifecycleModeV1, LifecycleUpgradeModeV1,
    LifecycleVerificationRequirementV1,
};
use std::collections::BTreeSet;

pub(super) fn required_lifecycle_controllers(
    control_class: CanisterControlClassV1,
    expected_controllers: &[String],
) -> Vec<String> {
    match control_class {
        CanisterControlClassV1::DeploymentControlled
        | CanisterControlClassV1::JointlyControlled => sorted_unique(expected_controllers.to_vec()),
        CanisterControlClassV1::CanicManagedPool
        | CanisterControlClassV1::ExternallyImported
        | CanisterControlClassV1::UserControlled
        | CanisterControlClassV1::UnknownUnsafe => Vec::new(),
    }
}

pub(super) fn external_lifecycle_controllers(
    control_class: CanisterControlClassV1,
    observed_controllers: &[String],
    required_controllers: &[String],
) -> Vec<String> {
    match control_class {
        CanisterControlClassV1::DeploymentControlled | CanisterControlClassV1::UnknownUnsafe => {
            Vec::new()
        }
        CanisterControlClassV1::JointlyControlled => {
            let required = required_controllers.iter().collect::<BTreeSet<_>>();
            sorted_unique(
                observed_controllers
                    .iter()
                    .filter(|controller| !required.contains(controller))
                    .cloned()
                    .collect(),
            )
        }
        CanisterControlClassV1::CanicManagedPool
        | CanisterControlClassV1::ExternallyImported
        | CanisterControlClassV1::UserControlled => sorted_unique(observed_controllers.to_vec()),
    }
}

pub(super) fn lifecycle_consent_requirements(
    control_class: CanisterControlClassV1,
    external_controllers: &[String],
) -> Vec<ConsentRequirementV1> {
    if !lifecycle_external_action_required(control_class) {
        return Vec::new();
    }
    vec![ConsentRequirementV1 {
        consent_subject_kind: consent_subject_kind(control_class),
        required_principals: sorted_unique(external_controllers.to_vec()),
        required_controller_set_digest: Some(stable_json_sha256_hex(&external_controllers)),
        consent_channel_kind: consent_channel_kind(control_class),
        required_action: required_consent_action(control_class),
    }]
}

pub(super) const fn lifecycle_mode(control_class: CanisterControlClassV1) -> LifecycleModeV1 {
    match control_class {
        CanisterControlClassV1::DeploymentControlled => LifecycleModeV1::DirectDeploymentAuthority,
        CanisterControlClassV1::CanicManagedPool => LifecycleModeV1::DelegatedInstallRequired,
        CanisterControlClassV1::ExternallyImported | CanisterControlClassV1::UserControlled => {
            LifecycleModeV1::ExternalCompletionOnly
        }
        CanisterControlClassV1::JointlyControlled => LifecycleModeV1::ProposalRequired,
        CanisterControlClassV1::UnknownUnsafe => LifecycleModeV1::UnknownUnsafeBlocked,
    }
}

pub(super) fn lifecycle_blockers(control_class: CanisterControlClassV1) -> Vec<String> {
    if control_class == CanisterControlClassV1::UnknownUnsafe {
        vec!["unknown unsafe controller state blocks lifecycle action".to_string()]
    } else {
        Vec::new()
    }
}

pub(super) fn lifecycle_warnings(control_class: CanisterControlClassV1) -> Vec<String> {
    match control_class {
        CanisterControlClassV1::CanicManagedPool => {
            vec!["pool-aware lifecycle policy is required before mutation".to_string()]
        }
        CanisterControlClassV1::ExternallyImported => {
            vec!["external controller action or verification is required".to_string()]
        }
        CanisterControlClassV1::JointlyControlled => {
            vec!["joint controller consent or delegation is required".to_string()]
        }
        CanisterControlClassV1::UserControlled => {
            vec!["user or delegated lifecycle action is required".to_string()]
        }
        CanisterControlClassV1::DeploymentControlled | CanisterControlClassV1::UnknownUnsafe => {
            Vec::new()
        }
    }
}

pub(super) fn lifecycle_upgrade_modes(
    control_class: CanisterControlClassV1,
) -> Vec<LifecycleUpgradeModeV1> {
    match control_class {
        CanisterControlClassV1::DeploymentControlled => vec![
            LifecycleUpgradeModeV1::DirectByDeploymentAuthority,
            LifecycleUpgradeModeV1::VerifyExternalCompletion,
        ],
        CanisterControlClassV1::CanicManagedPool
        | CanisterControlClassV1::ExternallyImported
        | CanisterControlClassV1::JointlyControlled
        | CanisterControlClassV1::UserControlled => vec![
            LifecycleUpgradeModeV1::ExternalProposal,
            LifecycleUpgradeModeV1::ExternalExecution,
            LifecycleUpgradeModeV1::VerifyExternalCompletion,
            LifecycleUpgradeModeV1::ObserveOnly,
        ],
        CanisterControlClassV1::UnknownUnsafe => vec![LifecycleUpgradeModeV1::Blocked],
    }
}

pub(super) fn lifecycle_verification_requirements(
    verifier_required: bool,
) -> Vec<LifecycleVerificationRequirementV1> {
    let mut requirements = vec![
        LifecycleVerificationRequirementV1::LiveInventory,
        LifecycleVerificationRequirementV1::ControllerObservation,
        LifecycleVerificationRequirementV1::ModuleHash,
        LifecycleVerificationRequirementV1::CanonicalEmbeddedConfig,
    ];
    if verifier_required {
        requirements.push(LifecycleVerificationRequirementV1::ProtectedCallReadiness);
    }
    requirements
}

pub(super) const fn lifecycle_external_action_required(
    control_class: CanisterControlClassV1,
) -> bool {
    matches!(
        control_class,
        CanisterControlClassV1::CanicManagedPool
            | CanisterControlClassV1::ExternallyImported
            | CanisterControlClassV1::JointlyControlled
            | CanisterControlClassV1::UserControlled
    )
}

pub(super) fn lifecycle_reason(control_class: CanisterControlClassV1) -> String {
    match control_class {
        CanisterControlClassV1::DeploymentControlled => {
            "deployment authority can execute lifecycle directly".to_string()
        }
        CanisterControlClassV1::CanicManagedPool => {
            "Canic-managed pool lifecycle requires pool-aware external action".to_string()
        }
        CanisterControlClassV1::ExternallyImported => {
            "externally imported canister requires external controller action".to_string()
        }
        CanisterControlClassV1::JointlyControlled => {
            "jointly controlled canister requires non-deployment-controller consent".to_string()
        }
        CanisterControlClassV1::UserControlled => {
            "user-controlled canister requires user or delegated lifecycle action".to_string()
        }
        CanisterControlClassV1::UnknownUnsafe => {
            "unknown or unsafe controller state blocks lifecycle action".to_string()
        }
    }
}

pub(super) const fn required_external_action(lifecycle_mode: LifecycleModeV1) -> &'static str {
    match lifecycle_mode {
        LifecycleModeV1::DirectDeploymentAuthority => "none",
        LifecycleModeV1::ProposalRequired => "proposal_and_consent",
        LifecycleModeV1::DelegatedInstallRequired => "delegated_install_or_pool_policy",
        LifecycleModeV1::ExternalCompletionOnly => "external_controller_execution",
        LifecycleModeV1::VerifyOnly => "verify_external_completion",
        LifecycleModeV1::MustNotTouch | LifecycleModeV1::UnknownUnsafeBlocked => "blocked",
    }
}

pub(super) fn sorted_unique(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

const fn consent_subject_kind(control_class: CanisterControlClassV1) -> ConsentSubjectKindV1 {
    match control_class {
        CanisterControlClassV1::CanicManagedPool => ConsentSubjectKindV1::ProjectHub,
        CanisterControlClassV1::ExternallyImported | CanisterControlClassV1::JointlyControlled => {
            ConsentSubjectKindV1::CustomerController
        }
        CanisterControlClassV1::UserControlled => ConsentSubjectKindV1::UserPrincipal,
        CanisterControlClassV1::DeploymentControlled | CanisterControlClassV1::UnknownUnsafe => {
            ConsentSubjectKindV1::UnknownExternalController
        }
    }
}

const fn consent_channel_kind(control_class: CanisterControlClassV1) -> ConsentChannelKindV1 {
    match control_class {
        CanisterControlClassV1::CanicManagedPool => ConsentChannelKindV1::DelegatedInstall,
        CanisterControlClassV1::ExternallyImported
        | CanisterControlClassV1::JointlyControlled
        | CanisterControlClassV1::UserControlled => ConsentChannelKindV1::GeneratedCommand,
        CanisterControlClassV1::DeploymentControlled | CanisterControlClassV1::UnknownUnsafe => {
            ConsentChannelKindV1::OutOfBand
        }
    }
}

const fn required_consent_action(
    control_class: CanisterControlClassV1,
) -> ExternalUpgradeAuthorizationModeV1 {
    match control_class {
        CanisterControlClassV1::JointlyControlled => {
            ExternalUpgradeAuthorizationModeV1::ConsentForDirectInstall
        }
        CanisterControlClassV1::CanicManagedPool => {
            ExternalUpgradeAuthorizationModeV1::DelegatedInstallAuthority
        }
        CanisterControlClassV1::ExternallyImported | CanisterControlClassV1::UserControlled => {
            ExternalUpgradeAuthorizationModeV1::ExternalControllerExecution
        }
        CanisterControlClassV1::DeploymentControlled | CanisterControlClassV1::UnknownUnsafe => {
            ExternalUpgradeAuthorizationModeV1::ObserveAndVerifyOnly
        }
    }
}
