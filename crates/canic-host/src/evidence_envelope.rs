//! Stable evidence envelopes for CI/GitOps automation.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

///
/// EvidenceEnvelopeV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceEnvelopeV1 {
    pub envelope_schema: PayloadSchemaRefV1,
    pub canic_version: String,
    pub command: CommandProvenanceV1,
    pub target: EvidenceTargetV1,
    pub generated_at: String,
    pub source_config: Option<InputFingerprintV1>,
    pub inputs: Vec<InputFingerprintV1>,
    pub payload_schema: PayloadSchemaRefV1,
    pub payload_sha256: Option<String>,
    pub payload: serde_json::Value,
    pub summary: EvidenceSummaryV1,
    pub exit_class: ExitClassV1,
}

///
/// CommandProvenanceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CommandProvenanceV1 {
    pub name: String,
    pub argv_normalized: Vec<String>,
    pub argv_redactions: Vec<String>,
    pub format: String,
}

///
/// EvidenceTargetV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceTargetV1 {
    pub kind: EvidenceTargetKindV1,
    pub deployment: Option<String>,
    pub fleet: Option<String>,
    pub role: Option<String>,
    pub profile: Option<String>,
    pub network: Option<String>,
}

///
/// EvidenceTargetKindV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceTargetKindV1 {
    Deployment,
    Fleet,
    FleetAdoption,
    Artifact,
    Unknown,
}

///
/// PayloadSchemaRefV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PayloadSchemaRefV1 {
    pub id: String,
    pub version: String,
    pub stability: PayloadSchemaStabilityV1,
}

impl PayloadSchemaRefV1 {
    #[must_use]
    pub fn stable(id: &str, version: &str) -> Self {
        Self {
            id: id.to_string(),
            version: version.to_string(),
            stability: PayloadSchemaStabilityV1::Stable,
        }
    }

    #[must_use]
    pub fn experimental(id: &str, version: &str) -> Self {
        Self {
            id: id.to_string(),
            version: version.to_string(),
            stability: PayloadSchemaStabilityV1::Experimental,
        }
    }

    #[must_use]
    pub fn internal(id: &str, version: &str) -> Self {
        Self {
            id: id.to_string(),
            version: version.to_string(),
            stability: PayloadSchemaStabilityV1::Internal,
        }
    }
}

///
/// PayloadSchemaStabilityV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadSchemaStabilityV1 {
    Stable,
    Experimental,
    Internal,
}

///
/// InputFingerprintV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InputFingerprintV1 {
    pub kind: String,
    pub path: Option<String>,
    pub sha256: Option<String>,
    pub size_bytes: Option<u64>,
    pub modified_unix_secs: Option<u64>,
    pub schema: Option<PayloadSchemaRefV1>,
    pub note: Option<String>,
}

///
/// EvidenceSummaryV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceSummaryV1 {
    pub warnings: Vec<EvidenceMessageV1>,
    pub blocked_actions: Vec<EvidenceMessageV1>,
    pub missing_or_stale_evidence: Vec<EvidenceMessageV1>,
    pub evidence_conflicts: Vec<EvidenceMessageV1>,
}

///
/// EvidenceMessageV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvidenceMessageV1 {
    pub code: String,
    pub message: String,
    pub severity: EvidenceMessageSeverityV1,
    pub source: Option<String>,
    pub related_input: Option<String>,
}

impl EvidenceMessageV1 {
    #[must_use]
    pub fn new(
        code: &str,
        message: impl Into<String>,
        severity: EvidenceMessageSeverityV1,
    ) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            severity,
            source: None,
            related_input: None,
        }
    }
}

///
/// EvidenceMessageSeverityV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceMessageSeverityV1 {
    Info,
    Warning,
    Error,
}

///
/// ExitClassV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitClassV1 {
    Success,
    SuccessWithWarnings,
    BlockedByPolicy,
    EvidenceConflict,
    MissingRequiredEvidence,
    InvalidInput,
    ExecutionFailed,
    InternalError,
}

pub const EVIDENCE_ENVELOPE_SCHEMA_ID: &str = "canic.evidence_envelope.v1";
pub const ADOPTION_REPORT_SCHEMA_ID: &str = "canic.adoption_report.v1";
pub const DEPLOYMENT_CHECK_SCHEMA_ID: &str = "canic.deployment_check.v1";

#[must_use]
pub fn evidence_envelope_schema() -> PayloadSchemaRefV1 {
    PayloadSchemaRefV1::stable(EVIDENCE_ENVELOPE_SCHEMA_ID, "1")
}

#[must_use]
pub fn adoption_report_schema() -> PayloadSchemaRefV1 {
    PayloadSchemaRefV1::experimental(ADOPTION_REPORT_SCHEMA_ID, "1")
}

#[must_use]
pub fn deployment_check_schema() -> PayloadSchemaRefV1 {
    PayloadSchemaRefV1::internal(DEPLOYMENT_CHECK_SCHEMA_ID, "1")
}

#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex_bytes(Sha256::digest(bytes))
}

fn hex_bytes(bytes: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = bytes.as_ref();
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_class_serializes_to_snake_case() {
        let encoded = serde_json::to_string(&ExitClassV1::SuccessWithWarnings).expect("serialize");

        assert_eq!(encoded, "\"success_with_warnings\"");
    }

    #[test]
    fn schema_refs_record_stability() {
        assert_eq!(
            evidence_envelope_schema(),
            PayloadSchemaRefV1 {
                id: "canic.evidence_envelope.v1".to_string(),
                version: "1".to_string(),
                stability: PayloadSchemaStabilityV1::Stable,
            }
        );
        assert_eq!(
            adoption_report_schema().stability,
            PayloadSchemaStabilityV1::Experimental
        );
        assert_eq!(
            deployment_check_schema().stability,
            PayloadSchemaStabilityV1::Internal
        );
    }
}
