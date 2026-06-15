use super::super::super::*;

pub(super) const fn root_verification_transition(
    status: DeploymentRootVerificationEvidenceStatusV1,
    current: DeploymentRootVerificationStateV1,
) -> DeploymentRootVerificationStateTransitionV1 {
    match (status, current) {
        (
            DeploymentRootVerificationEvidenceStatusV1::EvidenceSatisfied,
            DeploymentRootVerificationStateV1::NotVerified,
        ) => DeploymentRootVerificationStateTransitionV1::WouldPromoteNotVerifiedToVerified,
        (
            DeploymentRootVerificationEvidenceStatusV1::EvidenceSatisfied,
            DeploymentRootVerificationStateV1::Verified,
        ) => DeploymentRootVerificationStateTransitionV1::NoStateChange,
        _ => DeploymentRootVerificationStateTransitionV1::Blocked,
    }
}

pub(super) fn root_verification_next_actions(
    status: DeploymentRootVerificationEvidenceStatusV1,
) -> Vec<String> {
    match status {
        DeploymentRootVerificationEvidenceStatusV1::EvidenceSatisfied => vec![
            "run the explicit root verification command to write verified local state".to_string(),
        ],
        DeploymentRootVerificationEvidenceStatusV1::VerificationFailed => vec![
            "collect a deployment-truth check with matching root evidence before verifying"
                .to_string(),
        ],
        DeploymentRootVerificationEvidenceStatusV1::NotApplicable => Vec::new(),
    }
}
