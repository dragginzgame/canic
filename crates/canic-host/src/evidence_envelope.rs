//! Stable evidence envelopes for CI/GitOps automation.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Component, Path},
    time::UNIX_EPOCH,
};

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
    PolicyGate,
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
    pub path_display: InputPathDisplayV1,
    pub sha256: Option<String>,
    pub size_bytes: Option<u64>,
    pub modified_unix_secs: Option<u64>,
    pub schema: Option<PayloadSchemaRefV1>,
    pub note: Option<String>,
}

///
/// InputPathDisplayV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InputPathDisplayV1 {
    Relative,
    AbsoluteRedacted,
    Omitted,
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

impl ExitClassV1 {
    #[must_use]
    pub const fn precedence(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::SuccessWithWarnings => 1,
            Self::BlockedByPolicy => 2,
            Self::MissingRequiredEvidence => 3,
            Self::EvidenceConflict => 4,
            Self::InvalidInput => 5,
            Self::ExecutionFailed => 6,
            Self::InternalError => 7,
        }
    }

    #[must_use]
    pub const fn dominates(self, other: Self) -> bool {
        self.precedence() >= other.precedence()
    }
}

#[must_use]
pub fn combine_exit_classes(classes: impl IntoIterator<Item = ExitClassV1>) -> ExitClassV1 {
    classes
        .into_iter()
        .max_by_key(|class| class.precedence())
        .unwrap_or(ExitClassV1::Success)
}

#[must_use]
pub const fn evidence_summary_exit_class(
    summary: &EvidenceSummaryV1,
    missing_required_evidence: bool,
) -> ExitClassV1 {
    if !summary.evidence_conflicts.is_empty() {
        return ExitClassV1::EvidenceConflict;
    }
    if missing_required_evidence {
        return ExitClassV1::MissingRequiredEvidence;
    }
    if !summary.blocked_actions.is_empty() {
        return ExitClassV1::BlockedByPolicy;
    }
    if !summary.warnings.is_empty() || !summary.missing_or_stale_evidence.is_empty() {
        return ExitClassV1::SuccessWithWarnings;
    }

    ExitClassV1::Success
}

pub const EVIDENCE_ENVELOPE_SCHEMA_ID: &str = "canic.evidence_envelope.v1";
pub const ADOPTION_REPORT_SCHEMA_ID: &str = "canic.adoption_report.v1";
pub const DEPLOYMENT_CHECK_SCHEMA_ID: &str = "canic.deployment_check.v1";
pub const POLICY_GATE_REPORT_SCHEMA_ID: &str = "canic.policy_gate_report.v1";

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
pub fn policy_gate_report_schema() -> PayloadSchemaRefV1 {
    PayloadSchemaRefV1::stable(POLICY_GATE_REPORT_SCHEMA_ID, "1")
}

#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex_bytes(Sha256::digest(bytes))
}

pub fn json_payload_sha256<T>(payload: &T) -> Result<String, serde_json::Error>
where
    T: Serialize,
{
    Ok(sha256_hex(&serde_json::to_vec(payload)?))
}

pub fn file_input_fingerprint(
    kind: &str,
    path: &Path,
    root: &Path,
    schema: Option<PayloadSchemaRefV1>,
    note: Option<String>,
) -> io::Result<InputFingerprintV1> {
    let bytes = fs::read(path)?;
    let metadata = fs::metadata(path)?;
    let modified_unix_secs = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs());
    let path_summary = input_path_summary(path, root);

    Ok(InputFingerprintV1 {
        kind: kind.to_string(),
        path: path_summary.path,
        path_display: path_summary.display,
        sha256: Some(sha256_hex(&bytes)),
        size_bytes: Some(metadata.len()),
        modified_unix_secs,
        schema,
        note,
    })
}

#[must_use]
pub fn command_path_for_root(path: &Path, root: &Path) -> String {
    input_path_summary(path, root)
        .path
        .unwrap_or_else(|| "<redacted:absolute-outside-root>".to_string())
}

///
/// InputPathSummaryV1
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct InputPathSummaryV1 {
    path: Option<String>,
    display: InputPathDisplayV1,
}

fn input_path_summary(path: &Path, root: &Path) -> InputPathSummaryV1 {
    let canonical_path = fs::canonicalize(path).ok();
    let canonical_root = fs::canonicalize(root).ok();

    if let (Some(canonical_path), Some(canonical_root)) = (canonical_path, canonical_root) {
        if let Ok(relative) = canonical_path.strip_prefix(canonical_root) {
            return InputPathSummaryV1 {
                path: Some(path_to_display(relative)),
                display: InputPathDisplayV1::Relative,
            };
        }
        return InputPathSummaryV1 {
            path: None,
            display: InputPathDisplayV1::AbsoluteRedacted,
        };
    }

    if path.is_absolute() {
        return InputPathSummaryV1 {
            path: None,
            display: InputPathDisplayV1::AbsoluteRedacted,
        };
    }

    InputPathSummaryV1 {
        path: Some(path_to_display(path)),
        display: InputPathDisplayV1::Relative,
    }
}

fn path_to_display(path: &Path) -> String {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                components.push(prefix.as_os_str().to_string_lossy().to_string());
            }
            Component::RootDir | Component::CurDir => {}
            Component::ParentDir => components.push("..".to_string()),
            Component::Normal(segment) => components.push(segment.to_string_lossy().to_string()),
        }
    }

    if components.is_empty() {
        ".".to_string()
    } else {
        components.join("/")
    }
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
    use std::path::PathBuf;

    #[test]
    fn exit_class_serializes_to_snake_case() {
        let encoded = serde_json::to_string(&ExitClassV1::SuccessWithWarnings).expect("serialize");

        assert_eq!(encoded, "\"success_with_warnings\"");
    }

    #[test]
    fn exit_class_precedence_prefers_policy_relevant_failures() {
        assert_eq!(
            combine_exit_classes([
                ExitClassV1::SuccessWithWarnings,
                ExitClassV1::BlockedByPolicy,
                ExitClassV1::EvidenceConflict,
            ]),
            ExitClassV1::EvidenceConflict
        );
        assert!(ExitClassV1::InvalidInput.dominates(ExitClassV1::EvidenceConflict));
        assert!(ExitClassV1::InternalError.dominates(ExitClassV1::ExecutionFailed));
    }

    #[test]
    fn evidence_summary_exit_class_uses_stable_precedence() {
        let mut summary = EvidenceSummaryV1 {
            warnings: vec![EvidenceMessageV1::new(
                "test.warning",
                "warning",
                EvidenceMessageSeverityV1::Warning,
            )],
            blocked_actions: Vec::new(),
            missing_or_stale_evidence: Vec::new(),
            evidence_conflicts: Vec::new(),
        };

        assert_eq!(
            evidence_summary_exit_class(&summary, false),
            ExitClassV1::SuccessWithWarnings
        );

        summary.blocked_actions.push(EvidenceMessageV1::new(
            "test.blocked",
            "blocked",
            EvidenceMessageSeverityV1::Error,
        ));
        assert_eq!(
            evidence_summary_exit_class(&summary, false),
            ExitClassV1::BlockedByPolicy
        );
        assert_eq!(
            evidence_summary_exit_class(&summary, true),
            ExitClassV1::MissingRequiredEvidence
        );

        summary.evidence_conflicts.push(EvidenceMessageV1::new(
            "test.conflict",
            "conflict",
            EvidenceMessageSeverityV1::Error,
        ));
        assert_eq!(
            evidence_summary_exit_class(&summary, true),
            ExitClassV1::EvidenceConflict
        );
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

    #[test]
    fn file_input_fingerprint_uses_relative_path_under_root() {
        let root = temp_dir("canic-envelope-relative");
        let input = root.join("evidence").join("input.json");
        fs::create_dir_all(input.parent().expect("input parent")).expect("create parent");
        fs::write(&input, b"{\"ok\":true}").expect("write input");

        let fingerprint =
            file_input_fingerprint("input", &input, &root, None, None).expect("fingerprint");

        fs::remove_dir_all(&root).expect("clean temp dir");
        assert_eq!(fingerprint.path.as_deref(), Some("evidence/input.json"));
        assert_eq!(fingerprint.path_display, InputPathDisplayV1::Relative);
        assert_eq!(fingerprint.size_bytes, Some(11));
        assert!(
            fingerprint
                .sha256
                .as_deref()
                .is_some_and(|hash| hash.len() == 64)
        );
    }

    #[test]
    fn file_input_fingerprint_redacts_absolute_path_outside_root() {
        let root = temp_dir("canic-envelope-root");
        let outside = temp_dir("canic-envelope-outside");
        fs::create_dir_all(&root).expect("create root");
        fs::create_dir_all(&outside).expect("create outside");
        let input = outside.join("secret.json");
        fs::write(&input, b"secret").expect("write input");

        let fingerprint =
            file_input_fingerprint("input", &input, &root, None, None).expect("fingerprint");
        let command_path = command_path_for_root(&input, &root);

        fs::remove_dir_all(&root).expect("clean root");
        fs::remove_dir_all(&outside).expect("clean outside");
        assert_eq!(fingerprint.path, None);
        assert_eq!(
            fingerprint.path_display,
            InputPathDisplayV1::AbsoluteRedacted
        );
        assert_eq!(command_path, "<redacted:absolute-outside-root>");
    }

    fn temp_dir(name: &str) -> PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{suffix}"))
    }
}
