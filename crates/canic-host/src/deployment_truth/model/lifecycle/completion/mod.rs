use super::proposal::{
    ExternalUpgradeConsentEvidenceV1, ExternalUpgradeConsentStateV1, ExternalUpgradeProposalV1,
    ExternalUpgradeVerificationResultV1,
};
use super::verification::{
    ExternalUpgradeVerificationCheckV1, ExternalVerificationObservationSourceV1,
};
use serde::{Deserialize, Serialize};

///
/// ExternalUpgradeCompletionReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUpgradeCompletionReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub report_digest: String,
    pub proposal_id: String,
    pub proposal_digest: String,
    pub consent_evidence_id: String,
    pub consent_evidence_digest: String,
    pub verification_check_id: String,
    pub verification_check_digest: String,
    pub subject: String,
    pub canister_id: Option<String>,
    pub role: Option<String>,
    pub consent_state: ExternalUpgradeConsentStateV1,
    pub verification_result: ExternalUpgradeVerificationResultV1,
    pub verification_observation_source: ExternalVerificationObservationSourceV1,
    pub completion_status: ExternalUpgradeCompletionStatusV1,
    pub blockers: Vec<String>,
    pub next_actions: Vec<String>,
    pub status_summary: String,
}

///
/// ExternalUpgradeCompletionStatusV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ExternalUpgradeCompletionStatusV1 {
    AwaitingConsent,
    ConsentRefused,
    SuppliedEvidenceConsistent,
    AwaitingVerification,
    VerifiedComplete,
    VerificationFailed,
}

impl ExternalUpgradeCompletionStatusV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::AwaitingConsent => "awaiting_consent",
            Self::ConsentRefused => "consent_refused",
            Self::SuppliedEvidenceConsistent => "supplied_evidence_consistent",
            Self::AwaitingVerification => "awaiting_verification",
            Self::VerifiedComplete => "verified_complete",
            Self::VerificationFailed => "verification_failed",
        }
    }
}

///
/// ExternalUpgradeCompletionReportRequest
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUpgradeCompletionReportRequest {
    pub report_id: String,
    pub proposal: ExternalUpgradeProposalV1,
    pub consent_evidence: ExternalUpgradeConsentEvidenceV1,
    pub verification_check: ExternalUpgradeVerificationCheckV1,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_upgrade_completion_status_owns_text_labels() {
        assert_eq!(
            ExternalUpgradeCompletionStatusV1::AwaitingConsent.label(),
            "awaiting_consent"
        );
        assert_eq!(
            ExternalUpgradeCompletionStatusV1::ConsentRefused.label(),
            "consent_refused"
        );
        assert_eq!(
            ExternalUpgradeCompletionStatusV1::SuppliedEvidenceConsistent.label(),
            "supplied_evidence_consistent"
        );
        assert_eq!(
            ExternalUpgradeCompletionStatusV1::AwaitingVerification.label(),
            "awaiting_verification"
        );
        assert_eq!(
            ExternalUpgradeCompletionStatusV1::VerifiedComplete.label(),
            "verified_complete"
        );
        assert_eq!(
            ExternalUpgradeCompletionStatusV1::VerificationFailed.label(),
            "verification_failed"
        );
    }
}
