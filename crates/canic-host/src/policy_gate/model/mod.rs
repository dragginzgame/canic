use crate::evidence_envelope::{
    EvidenceEnvelopeV1, EvidenceTargetV1, ExitClassV1, InputFingerprintV1, PayloadSchemaRefV1,
    PayloadSchemaStabilityV1,
};
use serde::{Deserialize, Serialize, de};
use std::{collections::BTreeMap, path::Path};
use thiserror::Error as ThisError;

///
/// PolicyGateError
///
#[derive(Debug, ThisError)]
pub enum PolicyGateError {
    #[error("invalid policy: {0}")]
    InvalidPolicy(String),

    #[error("failed to parse policy TOML: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("failed to parse evidence envelope JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("failed to fingerprint policy input: {0}")]
    Io(#[from] std::io::Error),
}

///
/// CiPolicyV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CiPolicyV1 {
    pub schema_version: u32,
    pub envelope: PolicyEnvelopeRulesV1,
    pub exit_class: PolicyExitClassRulesV1,
    pub summary: Option<PolicySummaryRulesV1>,
    pub build_provenance: Option<PolicyBuildProvenanceRulesV1>,
    #[serde(default)]
    pub required_input: Vec<PolicyRequiredInputRuleV1>,
}

///
/// PolicyEnvelopeRulesV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PolicyEnvelopeRulesV1 {
    pub required_schema: String,
    pub allowed_payload_schemas: Option<Vec<String>>,
    pub allowed_payload_stability: Option<Vec<PayloadSchemaStabilityV1>>,
}

///
/// PolicyExitClassRulesV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PolicyExitClassRulesV1 {
    pub allowed: Vec<ExitClassV1>,
}

///
/// PolicySummaryRulesV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PolicySummaryRulesV1 {
    #[serde(default)]
    pub fail_on_evidence_conflicts: bool,
    #[serde(default)]
    pub fail_on_blocked_actions: bool,
    pub allow_missing_or_stale_evidence: Option<bool>,
}

///
/// PolicyBuildProvenanceRulesV1
///
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PolicyBuildProvenanceRulesV1 {
    pub(super) rules: Vec<PolicyBuildProvenanceRuleV1>,
}

///
/// PolicyBuildProvenanceRuleV1
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PolicyBuildProvenanceRuleV1 {
    CleanSource,
    CargoLock,
    WasmGzip,
    Sha256,
    PackageIdentityMatchesTarget,
}

impl PolicyBuildProvenanceRulesV1 {
    pub(super) fn is_enabled(&self, rule: PolicyBuildProvenanceRuleV1) -> bool {
        self.rules.contains(&rule)
    }
}

impl<'de> Deserialize<'de> for PolicyBuildProvenanceRulesV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "require_clean_source",
            "require_cargo_lock",
            "require_wasm_gzip",
            "require_sha256",
            "require_package_identity_matches_target",
        ];
        let values = BTreeMap::<String, bool>::deserialize(deserializer)?;
        let mut rules = Vec::new();
        for (key, enabled) in values {
            let rule = match key.as_str() {
                "require_clean_source" => PolicyBuildProvenanceRuleV1::CleanSource,
                "require_cargo_lock" => PolicyBuildProvenanceRuleV1::CargoLock,
                "require_wasm_gzip" => PolicyBuildProvenanceRuleV1::WasmGzip,
                "require_sha256" => PolicyBuildProvenanceRuleV1::Sha256,
                "require_package_identity_matches_target" => {
                    PolicyBuildProvenanceRuleV1::PackageIdentityMatchesTarget
                }
                unknown => return Err(de::Error::unknown_field(unknown, FIELDS)),
            };
            if enabled {
                rules.push(rule);
            }
        }
        Ok(Self { rules })
    }
}

///
/// PolicyRequiredInputRuleV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PolicyRequiredInputRuleV1 {
    pub kind: String,
    pub schema: Option<String>,
}

///
/// PolicyGateRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyGateRequest<'a> {
    pub policy_source: &'a str,
    pub policy_path: &'a Path,
    pub envelope_path: &'a Path,
    pub fingerprint_root: &'a Path,
    pub envelope: EvidenceEnvelopeV1,
}

///
/// ProjectEvidenceManifestGateRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectEvidenceManifestGateRequest<'a> {
    pub policy_source: &'a str,
    pub policy_path: &'a Path,
    pub manifest_source: &'a str,
    pub manifest_path: &'a Path,
    pub fingerprint_root: &'a Path,
}

///
/// ProjectEvidenceManifestV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProjectEvidenceManifestV1 {
    pub schema_version: u32,
    pub project: ProjectEvidenceManifestProjectV1,
    pub evidence: Vec<ProjectEvidenceManifestEntryV1>,
}

///
/// ProjectEvidenceManifestProjectV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProjectEvidenceManifestProjectV1 {
    pub name: String,
    pub root: String,
}

///
/// ProjectEvidenceManifestEntryV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProjectEvidenceManifestEntryV1 {
    pub kind: String,
    pub path: String,
    pub required: bool,
    pub payload_schema: String,
    pub target: ProjectEvidenceManifestTargetV1,
}

///
/// ProjectEvidenceManifestTargetV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectEvidenceManifestTargetV1 {
    pub deployment: Option<String>,
    pub fleet: Option<String>,
    pub role: Option<String>,
    pub profile: Option<String>,
    pub network: Option<String>,
}

///
/// PolicyGateReportV1
///
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PolicyGateReportV1 {
    pub schema_version: u32,
    pub policy_schema_version: u32,
    pub policy_file_fingerprint: InputFingerprintV1,
    pub evaluated_envelope_fingerprint: InputFingerprintV1,
    pub evaluated_envelope_exit_class: ExitClassV1,
    pub evaluated_payload_schema: PayloadSchemaRefV1,
    pub evaluated_target: EvidenceTargetV1,
    pub policy_status: PolicyEvaluationStatusV1,
    pub gate_exit_class: ExitClassV1,
    pub requirements: Vec<PolicyRequirementV1>,
    pub findings: Vec<PolicyFindingV1>,
}

///
/// ProjectEvidenceGateReportV1
///
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProjectEvidenceGateReportV1 {
    pub schema_version: u32,
    pub manifest_schema_version: u32,
    pub project_name: String,
    pub policy_file_fingerprint: InputFingerprintV1,
    pub manifest_file_fingerprint: InputFingerprintV1,
    pub policy_status: PolicyEvaluationStatusV1,
    pub gate_exit_class: ExitClassV1,
    pub evidence: Vec<ProjectEvidenceGateEntryReportV1>,
}

///
/// ProjectEvidenceGateEntryReportV1
///
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProjectEvidenceGateEntryReportV1 {
    pub kind: String,
    pub path: String,
    pub required: bool,
    pub expected_payload_schema: String,
    pub expected_target: ProjectEvidenceManifestTargetV1,
    pub status: PolicyEvaluationStatusV1,
    pub gate_exit_class: ExitClassV1,
    pub evaluated_envelope_fingerprint: Option<InputFingerprintV1>,
    pub policy_report: Option<PolicyGateReportV1>,
    pub findings: Vec<PolicyFindingV1>,
}

///
/// PolicyRequirementV1
///
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PolicyRequirementV1 {
    pub requirement_id: String,
    pub status: PolicyEvaluationStatusV1,
    pub exit_class: ExitClassV1,
    pub finding_codes: Vec<String>,
}

///
/// PolicyFindingV1
///
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PolicyFindingV1 {
    pub code: String,
    pub severity: PolicyFindingSeverityV1,
    pub message: String,
    pub requirement_id: Option<String>,
    pub subject: Option<String>,
    pub expected: Option<serde_json::Value>,
    pub actual: Option<serde_json::Value>,
    pub evidence_path: Option<String>,
    pub target: Option<EvidenceTargetV1>,
    pub related_input: Option<String>,
}

///
/// PolicyFindingSeverityV1
///
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyFindingSeverityV1 {
    Info,
    Warning,
    Error,
}

///
/// PolicyEvaluationStatusV1
///
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyEvaluationStatusV1 {
    Passed,
    Failed,
}

impl ProjectEvidenceManifestTargetV1 {
    pub(super) const fn has_selector(&self) -> bool {
        self.deployment.is_some()
            || self.fleet.is_some()
            || self.role.is_some()
            || self.profile.is_some()
            || self.network.is_some()
    }

    pub(super) fn matches_envelope_target(&self, target: &EvidenceTargetV1) -> bool {
        self.deployment
            .as_ref()
            .is_none_or(|expected| target.deployment.as_ref() == Some(expected))
            && self
                .fleet
                .as_ref()
                .is_none_or(|expected| target.fleet.as_ref() == Some(expected))
            && self
                .role
                .as_ref()
                .is_none_or(|expected| target.role.as_ref() == Some(expected))
            && self
                .profile
                .as_ref()
                .is_none_or(|expected| target.profile.as_ref() == Some(expected))
            && self
                .network
                .as_ref()
                .is_none_or(|expected| target.network.as_ref() == Some(expected))
    }
}

impl PolicyFindingV1 {
    pub(super) fn error(
        code: &str,
        message: impl Into<String>,
        requirement_id: &str,
        exit_class: ExitClassV1,
    ) -> Self {
        Self {
            code: code.to_string(),
            severity: PolicyFindingSeverityV1::Error,
            message: message.into(),
            requirement_id: Some(requirement_id.to_string()),
            subject: Some(exit_class_subject(exit_class).to_string()),
            expected: None,
            actual: None,
            evidence_path: None,
            target: None,
            related_input: None,
        }
    }

    pub(super) fn warning(code: &str, message: impl Into<String>, requirement_id: &str) -> Self {
        Self {
            code: code.to_string(),
            severity: PolicyFindingSeverityV1::Warning,
            message: message.into(),
            requirement_id: Some(requirement_id.to_string()),
            subject: Some("success_with_warnings".to_string()),
            expected: None,
            actual: None,
            evidence_path: None,
            target: None,
            related_input: None,
        }
    }

    pub(super) fn expected(mut self, expected: serde_json::Value) -> Self {
        self.expected = Some(expected);
        self
    }

    pub(super) fn actual(mut self, actual: serde_json::Value) -> Self {
        self.actual = Some(actual);
        self
    }

    pub(super) fn exit_class(&self) -> ExitClassV1 {
        match self.subject.as_deref() {
            Some("evidence_conflict") => ExitClassV1::EvidenceConflict,
            Some("missing_required_evidence") => ExitClassV1::MissingRequiredEvidence,
            _ => ExitClassV1::BlockedByPolicy,
        }
    }
}

const fn exit_class_subject(exit_class: ExitClassV1) -> &'static str {
    match exit_class {
        ExitClassV1::EvidenceConflict => "evidence_conflict",
        ExitClassV1::MissingRequiredEvidence => "missing_required_evidence",
        _ => "blocked_by_policy",
    }
}
