//! Passive CI policy gates over stable evidence envelopes.

use crate::build_provenance::{
    ArtifactProvenanceKindV1, BUILD_PROVENANCE_SCHEMA_ID, BuildProvenanceV1, SourceDirtyPolicyV1,
};
use crate::evidence_envelope::{
    EvidenceEnvelopeV1, EvidenceSummaryV1, EvidenceTargetV1, ExitClassV1, InputFingerprintV1,
    PayloadSchemaRefV1, PayloadSchemaStabilityV1, combine_exit_classes, evidence_envelope_schema,
    file_input_fingerprint, project_evidence_manifest_schema,
};
use serde::{Deserialize, Serialize, de};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};
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
    rules: Vec<PolicyBuildProvenanceRuleV1>,
}

///
/// PolicyBuildProvenanceRuleV1
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PolicyBuildProvenanceRuleV1 {
    CleanSource,
    CargoLock,
    WasmGzip,
    Sha256,
    PackageIdentityMatchesTarget,
}

impl PolicyBuildProvenanceRulesV1 {
    fn is_enabled(&self, rule: PolicyBuildProvenanceRuleV1) -> bool {
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

pub fn parse_ci_policy_v1(source: &str) -> Result<CiPolicyV1, PolicyGateError> {
    let policy = toml::from_str::<CiPolicyV1>(source)?;
    validate_ci_policy_v1(&policy)?;
    Ok(policy)
}

pub fn parse_project_evidence_manifest_v1(
    source: &str,
) -> Result<ProjectEvidenceManifestV1, PolicyGateError> {
    let manifest = toml::from_str::<ProjectEvidenceManifestV1>(source)?;
    validate_project_evidence_manifest_v1(&manifest)?;
    Ok(manifest)
}

pub fn evaluate_policy_gate(
    request: PolicyGateRequest<'_>,
) -> Result<PolicyGateReportV1, PolicyGateError> {
    let policy = parse_ci_policy_v1(request.policy_source)?;
    let policy_file_fingerprint = file_input_fingerprint(
        "ci_policy",
        request.policy_path,
        request.fingerprint_root,
        None,
        None,
    )?;
    let evaluated_envelope_fingerprint = file_input_fingerprint(
        "evidence_envelope",
        request.envelope_path,
        request.fingerprint_root,
        Some(evidence_envelope_schema()),
        None,
    )?;
    Ok(evaluate_policy(
        &policy,
        policy_file_fingerprint,
        evaluated_envelope_fingerprint,
        request.envelope,
    ))
}

pub fn evaluate_project_evidence_manifest_gate(
    request: ProjectEvidenceManifestGateRequest<'_>,
) -> Result<ProjectEvidenceGateReportV1, PolicyGateError> {
    let policy = parse_ci_policy_v1(request.policy_source)?;
    let manifest = parse_project_evidence_manifest_v1(request.manifest_source)?;
    let policy_file_fingerprint = file_input_fingerprint(
        "ci_policy",
        request.policy_path,
        request.fingerprint_root,
        None,
        None,
    )?;
    let manifest_file_fingerprint = file_input_fingerprint(
        "project_evidence_manifest",
        request.manifest_path,
        request.fingerprint_root,
        Some(project_evidence_manifest_schema()),
        None,
    )?;
    let project_root = manifest_project_root(request.manifest_path, &manifest.project.root);
    let mut evidence = Vec::new();

    for entry in &manifest.evidence {
        evidence.push(evaluate_manifest_entry(
            &policy,
            &policy_file_fingerprint,
            &project_root,
            entry,
        )?);
    }

    let has_failures = evidence
        .iter()
        .any(|entry| entry.status == PolicyEvaluationStatusV1::Failed);
    let gate_exit_class = combine_exit_classes(evidence.iter().map(|entry| entry.gate_exit_class));

    Ok(ProjectEvidenceGateReportV1 {
        schema_version: 1,
        manifest_schema_version: manifest.schema_version,
        project_name: manifest.project.name,
        policy_file_fingerprint,
        manifest_file_fingerprint,
        policy_status: if has_failures {
            PolicyEvaluationStatusV1::Failed
        } else {
            PolicyEvaluationStatusV1::Passed
        },
        gate_exit_class,
        evidence,
    })
}

fn validate_ci_policy_v1(policy: &CiPolicyV1) -> Result<(), PolicyGateError> {
    if policy.schema_version != 1 {
        return Err(PolicyGateError::InvalidPolicy(format!(
            "unsupported schema_version {}; expected 1",
            policy.schema_version
        )));
    }
    ensure_nonempty("envelope.required_schema", &policy.envelope.required_schema)?;
    ensure_optional_allow_list(
        "envelope.allowed_payload_schemas",
        policy.envelope.allowed_payload_schemas.as_deref(),
    )?;
    ensure_optional_allow_list(
        "envelope.allowed_payload_stability",
        policy.envelope.allowed_payload_stability.as_deref(),
    )?;
    if policy.exit_class.allowed.is_empty() {
        return Err(PolicyGateError::InvalidPolicy(
            "exit_class.allowed must not be empty".to_string(),
        ));
    }
    if policy
        .build_provenance
        .as_ref()
        .is_some_and(|rules| rules.rules.is_empty())
    {
        return Err(PolicyGateError::InvalidPolicy(
            "build_provenance must enable at least one rule".to_string(),
        ));
    }
    for (index, rule) in policy.required_input.iter().enumerate() {
        ensure_nonempty(&format!("required_input[{index}].kind"), &rule.kind)?;
        if let Some(schema) = &rule.schema {
            ensure_nonempty(&format!("required_input[{index}].schema"), schema)?;
        }
    }
    Ok(())
}

fn validate_project_evidence_manifest_v1(
    manifest: &ProjectEvidenceManifestV1,
) -> Result<(), PolicyGateError> {
    if manifest.schema_version != 1 {
        return Err(PolicyGateError::InvalidPolicy(format!(
            "unsupported project evidence manifest schema_version {}; expected 1",
            manifest.schema_version
        )));
    }
    ensure_nonempty("project.name", &manifest.project.name)?;
    ensure_nonempty("project.root", &manifest.project.root)?;
    if manifest.evidence.is_empty() {
        return Err(PolicyGateError::InvalidPolicy(
            "evidence must not be empty".to_string(),
        ));
    }
    let mut seen_paths = BTreeSet::new();
    for (index, entry) in manifest.evidence.iter().enumerate() {
        ensure_nonempty(&format!("evidence[{index}].kind"), &entry.kind)?;
        ensure_nonempty(&format!("evidence[{index}].path"), &entry.path)?;
        let path_key = manifest_evidence_path_key(&entry.path);
        if !seen_paths.insert(path_key.clone()) {
            return Err(PolicyGateError::InvalidPolicy(format!(
                "evidence[{index}].path duplicates an earlier evidence path after normalization: {path_key}"
            )));
        }
        ensure_nonempty(
            &format!("evidence[{index}].payload_schema"),
            &entry.payload_schema,
        )?;
        if !entry.target.has_selector() {
            return Err(PolicyGateError::InvalidPolicy(format!(
                "evidence[{index}].target must include at least one target field"
            )));
        }
    }
    Ok(())
}

fn manifest_evidence_path_key(path: &str) -> String {
    let mut components = Vec::new();

    for component in Path::new(path.trim()).components() {
        match component {
            Component::Prefix(prefix) => {
                components.push(prefix.as_os_str().to_string_lossy().to_string());
            }
            Component::RootDir => components.push(String::new()),
            Component::CurDir => {}
            Component::ParentDir => {
                if components
                    .last()
                    .is_some_and(|component| !component.is_empty() && component != "..")
                {
                    components.pop();
                } else {
                    components.push("..".to_string());
                }
            }
            Component::Normal(segment) => {
                components.push(segment.to_string_lossy().to_string());
            }
        }
    }

    if components.is_empty() {
        ".".to_string()
    } else {
        components.join("/")
    }
}

fn ensure_optional_allow_list<T>(field: &str, value: Option<&[T]>) -> Result<(), PolicyGateError> {
    if value.is_some_and(<[T]>::is_empty) {
        return Err(PolicyGateError::InvalidPolicy(format!(
            "{field} must not be empty when present"
        )));
    }
    Ok(())
}

fn ensure_nonempty(field: &str, value: &str) -> Result<(), PolicyGateError> {
    if value.trim().is_empty() {
        return Err(PolicyGateError::InvalidPolicy(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

impl ProjectEvidenceManifestTargetV1 {
    const fn has_selector(&self) -> bool {
        self.deployment.is_some()
            || self.fleet.is_some()
            || self.role.is_some()
            || self.profile.is_some()
            || self.network.is_some()
    }

    fn matches_envelope_target(&self, target: &EvidenceTargetV1) -> bool {
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

fn evaluate_policy(
    policy: &CiPolicyV1,
    policy_file_fingerprint: InputFingerprintV1,
    evaluated_envelope_fingerprint: InputFingerprintV1,
    envelope: EvidenceEnvelopeV1,
) -> PolicyGateReportV1 {
    let mut builder = PolicyReportBuilder::default();
    builder.evaluate_envelope_schema(policy, &envelope);
    builder.evaluate_payload_schema(policy, &envelope);
    builder.evaluate_payload_stability(policy, &envelope);
    builder.evaluate_exit_class(policy, &envelope);
    builder.evaluate_summary(policy.summary.as_ref(), &envelope.summary);
    builder.evaluate_build_provenance(policy.build_provenance.as_ref(), &envelope);
    builder.evaluate_required_inputs(&policy.required_input, &envelope.inputs);

    let has_failures = builder
        .requirements
        .iter()
        .any(|requirement| requirement.status == PolicyEvaluationStatusV1::Failed);
    let gate_exit_class = if has_failures {
        combine_exit_classes(builder.findings.iter().map(PolicyFindingV1::exit_class))
    } else if envelope.exit_class == ExitClassV1::SuccessWithWarnings
        || !envelope.summary.warnings.is_empty()
        || !envelope.summary.missing_or_stale_evidence.is_empty()
    {
        ExitClassV1::SuccessWithWarnings
    } else {
        ExitClassV1::Success
    };

    PolicyGateReportV1 {
        schema_version: 1,
        policy_schema_version: policy.schema_version,
        policy_file_fingerprint,
        evaluated_envelope_fingerprint,
        evaluated_envelope_exit_class: envelope.exit_class,
        evaluated_payload_schema: envelope.payload_schema,
        evaluated_target: envelope.target,
        policy_status: if has_failures {
            PolicyEvaluationStatusV1::Failed
        } else {
            PolicyEvaluationStatusV1::Passed
        },
        gate_exit_class,
        requirements: builder.requirements,
        findings: builder.findings,
    }
}

fn evaluate_manifest_entry(
    policy: &CiPolicyV1,
    policy_file_fingerprint: &InputFingerprintV1,
    project_root: &Path,
    entry: &ProjectEvidenceManifestEntryV1,
) -> Result<ProjectEvidenceGateEntryReportV1, PolicyGateError> {
    let evidence_path = resolve_manifest_entry_path(project_root, &entry.path);
    if !evidence_path.is_file() {
        return Ok(missing_manifest_entry_report(entry));
    }

    let envelope_source = fs::read_to_string(&evidence_path)?;
    let envelope = serde_json::from_str::<EvidenceEnvelopeV1>(&envelope_source)?;
    let evaluated_envelope_fingerprint = file_input_fingerprint(
        "evidence_envelope",
        &evidence_path,
        project_root,
        Some(evidence_envelope_schema()),
        None,
    )?;
    let mut policy_report = evaluate_policy(
        policy,
        policy_file_fingerprint.clone(),
        evaluated_envelope_fingerprint.clone(),
        envelope.clone(),
    );
    let mut findings = Vec::new();
    let mut gate_exit_classes = vec![policy_report.gate_exit_class];

    if envelope.payload_schema.id != entry.payload_schema {
        let finding = PolicyFindingV1::error(
            "policy.manifest.payload_schema_mismatch",
            "manifest evidence payload schema does not match the evaluated envelope",
            "manifest.evidence.payload_schema",
            ExitClassV1::BlockedByPolicy,
        )
        .expected(serde_json::json!(entry.payload_schema))
        .actual(serde_json::json!(envelope.payload_schema.id));
        gate_exit_classes.push(finding.exit_class());
        findings.push(finding);
    }

    if !entry.target.matches_envelope_target(&envelope.target) {
        let finding = PolicyFindingV1::error(
            "policy.manifest.target_mismatch",
            "manifest evidence target does not match the evaluated envelope target",
            "manifest.evidence.target",
            ExitClassV1::BlockedByPolicy,
        )
        .expected(serde_json::json!(entry.target))
        .actual(serde_json::json!(envelope.target));
        gate_exit_classes.push(finding.exit_class());
        findings.push(finding);
    }

    policy_report.findings.extend(findings.clone());
    let gate_exit_class = combine_exit_classes(gate_exit_classes);
    policy_report.gate_exit_class = gate_exit_class;
    if !findings.is_empty() {
        policy_report.policy_status = PolicyEvaluationStatusV1::Failed;
    }

    Ok(ProjectEvidenceGateEntryReportV1 {
        kind: entry.kind.clone(),
        path: entry.path.clone(),
        required: entry.required,
        expected_payload_schema: entry.payload_schema.clone(),
        expected_target: entry.target.clone(),
        status: if policy_report.policy_status == PolicyEvaluationStatusV1::Failed {
            PolicyEvaluationStatusV1::Failed
        } else {
            PolicyEvaluationStatusV1::Passed
        },
        gate_exit_class,
        evaluated_envelope_fingerprint: Some(evaluated_envelope_fingerprint),
        policy_report: Some(policy_report),
        findings,
    })
}

fn missing_manifest_entry_report(
    entry: &ProjectEvidenceManifestEntryV1,
) -> ProjectEvidenceGateEntryReportV1 {
    let (status, gate_exit_class, findings) = if entry.required {
        (
            PolicyEvaluationStatusV1::Failed,
            ExitClassV1::MissingRequiredEvidence,
            vec![
                PolicyFindingV1::error(
                    "policy.manifest.required_evidence_missing",
                    "required manifest evidence file is missing",
                    "manifest.evidence.path",
                    ExitClassV1::MissingRequiredEvidence,
                )
                .expected(serde_json::json!(entry.path)),
            ],
        )
    } else {
        (
            PolicyEvaluationStatusV1::Passed,
            ExitClassV1::SuccessWithWarnings,
            vec![
                PolicyFindingV1::warning(
                    "policy.manifest.optional_evidence_missing",
                    "optional manifest evidence file is missing",
                    "manifest.evidence.path",
                )
                .expected(serde_json::json!(entry.path)),
            ],
        )
    };

    ProjectEvidenceGateEntryReportV1 {
        kind: entry.kind.clone(),
        path: entry.path.clone(),
        required: entry.required,
        expected_payload_schema: entry.payload_schema.clone(),
        expected_target: entry.target.clone(),
        status,
        gate_exit_class,
        evaluated_envelope_fingerprint: None,
        policy_report: None,
        findings,
    }
}

fn manifest_project_root(manifest_path: &Path, root: &str) -> PathBuf {
    let root_path = PathBuf::from(root);
    if root_path.is_absolute() {
        return root_path;
    }
    manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(root_path)
}

fn resolve_manifest_entry_path(project_root: &Path, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        project_root.join(path)
    }
}

#[derive(Default)]
struct PolicyReportBuilder {
    requirements: Vec<PolicyRequirementV1>,
    findings: Vec<PolicyFindingV1>,
}

impl PolicyReportBuilder {
    fn evaluate_envelope_schema(&mut self, policy: &CiPolicyV1, envelope: &EvidenceEnvelopeV1) {
        let requirement_id = "envelope.required_schema";
        let actual = envelope.envelope_schema.id.clone();
        if actual == policy.envelope.required_schema {
            self.pass(requirement_id);
            return;
        }
        self.fail(
            requirement_id,
            PolicyFindingV1::error(
                "policy.envelope_schema.mismatch",
                format!(
                    "evidence envelope schema '{}' does not match required schema '{}'",
                    actual, policy.envelope.required_schema
                ),
                requirement_id,
                ExitClassV1::BlockedByPolicy,
            )
            .expected(serde_json::json!(policy.envelope.required_schema))
            .actual(serde_json::json!(actual)),
        );
    }

    fn evaluate_payload_schema(&mut self, policy: &CiPolicyV1, envelope: &EvidenceEnvelopeV1) {
        let requirement_id = "envelope.allowed_payload_schemas";
        let Some(allowed) = policy.envelope.allowed_payload_schemas.as_ref() else {
            return;
        };
        let actual = envelope.payload_schema.id.clone();
        if allowed.contains(&actual) {
            self.pass(requirement_id);
            return;
        }
        self.fail(
            requirement_id,
            PolicyFindingV1::error(
                "policy.payload_schema.disallowed",
                format!("payload schema '{actual}' is not allowed by policy"),
                requirement_id,
                ExitClassV1::BlockedByPolicy,
            )
            .expected(serde_json::json!(allowed))
            .actual(serde_json::json!(actual)),
        );
    }

    fn evaluate_payload_stability(&mut self, policy: &CiPolicyV1, envelope: &EvidenceEnvelopeV1) {
        let requirement_id = "envelope.allowed_payload_stability";
        let Some(allowed) = policy.envelope.allowed_payload_stability.as_ref() else {
            return;
        };
        let actual = envelope.payload_schema.stability;
        if allowed.contains(&actual) {
            self.pass(requirement_id);
            return;
        }
        self.fail(
            requirement_id,
            PolicyFindingV1::error(
                "policy.payload_stability.disallowed",
                "payload schema stability is not allowed by policy",
                requirement_id,
                ExitClassV1::BlockedByPolicy,
            )
            .expected(serde_json::json!(allowed))
            .actual(serde_json::json!(actual)),
        );
    }

    fn evaluate_exit_class(&mut self, policy: &CiPolicyV1, envelope: &EvidenceEnvelopeV1) {
        let requirement_id = "exit_class.allowed";
        let actual = envelope.exit_class;
        if policy.exit_class.allowed.contains(&actual) && is_success_exit_class(actual) {
            self.pass(requirement_id);
            return;
        }

        let exit_class = match actual {
            ExitClassV1::EvidenceConflict => ExitClassV1::EvidenceConflict,
            ExitClassV1::MissingRequiredEvidence => ExitClassV1::MissingRequiredEvidence,
            _ => ExitClassV1::BlockedByPolicy,
        };
        self.fail(
            requirement_id,
            PolicyFindingV1::error(
                "policy.exit_class.disallowed",
                format!("evidence exit class '{actual:?}' is not allowed by policy"),
                requirement_id,
                exit_class,
            )
            .expected(serde_json::json!(policy.exit_class.allowed))
            .actual(serde_json::json!(actual)),
        );
    }

    fn evaluate_summary(
        &mut self,
        policy: Option<&PolicySummaryRulesV1>,
        summary: &EvidenceSummaryV1,
    ) {
        let Some(policy) = policy else {
            return;
        };

        if policy.fail_on_evidence_conflicts {
            let requirement_id = "summary.fail_on_evidence_conflicts";
            if summary.evidence_conflicts.is_empty() {
                self.pass(requirement_id);
            } else {
                self.fail(
                    requirement_id,
                    PolicyFindingV1::error(
                        "policy.summary.evidence_conflict",
                        "evidence summary contains conflicts",
                        requirement_id,
                        ExitClassV1::EvidenceConflict,
                    )
                    .actual(serde_json::json!(message_codes(
                        &summary.evidence_conflicts
                    ))),
                );
            }
        }

        if policy.fail_on_blocked_actions {
            let requirement_id = "summary.fail_on_blocked_actions";
            if summary.blocked_actions.is_empty() {
                self.pass(requirement_id);
            } else {
                self.fail(
                    requirement_id,
                    PolicyFindingV1::error(
                        "policy.summary.blocked_action",
                        "evidence summary contains blocked actions",
                        requirement_id,
                        ExitClassV1::BlockedByPolicy,
                    )
                    .actual(serde_json::json!(message_codes(&summary.blocked_actions))),
                );
            }
        }

        if policy.allow_missing_or_stale_evidence == Some(false) {
            let requirement_id = "summary.allow_missing_or_stale_evidence";
            if summary.missing_or_stale_evidence.is_empty() {
                self.pass(requirement_id);
            } else {
                self.fail(
                    requirement_id,
                    PolicyFindingV1::error(
                        "policy.summary.missing_or_stale_evidence",
                        "evidence summary contains missing or stale evidence",
                        requirement_id,
                        ExitClassV1::MissingRequiredEvidence,
                    )
                    .actual(serde_json::json!(message_codes(
                        &summary.missing_or_stale_evidence
                    ))),
                );
            }
        }
    }

    fn evaluate_required_inputs(
        &mut self,
        rules: &[PolicyRequiredInputRuleV1],
        inputs: &[InputFingerprintV1],
    ) {
        for (index, rule) in rules.iter().enumerate() {
            let requirement_id = format!("required_input.{index}");
            let kind_matches = inputs
                .iter()
                .filter(|input| input.kind == rule.kind)
                .collect::<Vec<_>>();
            let matched = kind_matches.iter().any(|input| {
                rule.schema.as_ref().is_none_or(|schema| {
                    input
                        .schema
                        .as_ref()
                        .is_some_and(|input_schema| input_schema.id == *schema)
                })
            });
            if matched {
                self.pass(&requirement_id);
                continue;
            }

            let actual = if kind_matches.is_empty() {
                serde_json::json!([])
            } else {
                serde_json::json!(
                    kind_matches
                        .iter()
                        .map(|input| input.schema.as_ref().map(|schema| schema.id.clone()))
                        .collect::<Vec<_>>()
                )
            };
            self.fail(
                &requirement_id,
                PolicyFindingV1::error(
                    "policy.required_input.missing",
                    format!("required input '{}' was not found", rule.kind),
                    &requirement_id,
                    ExitClassV1::MissingRequiredEvidence,
                )
                .expected(serde_json::json!({
                    "kind": rule.kind,
                    "schema": rule.schema,
                }))
                .actual(actual),
            );
        }
    }

    fn evaluate_build_provenance(
        &mut self,
        rules: Option<&PolicyBuildProvenanceRulesV1>,
        envelope: &EvidenceEnvelopeV1,
    ) {
        let Some(rules) = rules else {
            return;
        };

        if envelope.payload_schema.id.as_str() != BUILD_PROVENANCE_SCHEMA_ID {
            self.fail_enabled_build_provenance_rules(
                rules,
                "policy.build_provenance.payload_schema",
                "build-provenance policy rules require a canic.build_provenance.v1 payload",
                ExitClassV1::BlockedByPolicy,
                serde_json::json!(BUILD_PROVENANCE_SCHEMA_ID),
                serde_json::json!(envelope.payload_schema.id.clone()),
            );
            return;
        }

        let provenance = match serde_json::from_value::<BuildProvenanceV1>(envelope.payload.clone())
        {
            Ok(provenance) => provenance,
            Err(err) => {
                self.fail_enabled_build_provenance_rules(
                    rules,
                    "policy.build_provenance.invalid_payload",
                    "build-provenance policy rules could not decode the envelope payload",
                    ExitClassV1::BlockedByPolicy,
                    serde_json::json!("BuildProvenanceV1"),
                    serde_json::json!(err.to_string()),
                );
                return;
            }
        };

        if rules.is_enabled(PolicyBuildProvenanceRuleV1::CleanSource) {
            self.evaluate_clean_source(&provenance);
        }
        if rules.is_enabled(PolicyBuildProvenanceRuleV1::CargoLock) {
            self.evaluate_cargo_lock(&provenance);
        }
        if rules.is_enabled(PolicyBuildProvenanceRuleV1::WasmGzip) {
            self.evaluate_wasm_gzip(&provenance);
        }
        if rules.is_enabled(PolicyBuildProvenanceRuleV1::Sha256) {
            self.evaluate_sha256(&provenance);
        }
        if rules.is_enabled(PolicyBuildProvenanceRuleV1::PackageIdentityMatchesTarget) {
            self.evaluate_package_identity_matches_target(&provenance, &envelope.target);
        }
    }

    fn evaluate_clean_source(&mut self, provenance: &BuildProvenanceV1) {
        let requirement_id = "build_provenance.require_clean_source";
        if provenance.source.dirty == Some(false)
            && provenance.source.dirty_policy == SourceDirtyPolicyV1::Clean
        {
            self.pass(requirement_id);
            return;
        }

        self.fail(
            requirement_id,
            PolicyFindingV1::error(
                "policy.build_provenance.source_not_clean",
                "build provenance does not prove a clean source checkout",
                requirement_id,
                ExitClassV1::BlockedByPolicy,
            )
            .expected(serde_json::json!({
                "dirty": false,
                "dirty_policy": SourceDirtyPolicyV1::Clean,
            }))
            .actual(serde_json::json!({
                "dirty": provenance.source.dirty,
                "dirty_policy": provenance.source.dirty_policy,
            })),
        );
    }

    fn evaluate_cargo_lock(&mut self, provenance: &BuildProvenanceV1) {
        let requirement_id = "build_provenance.require_cargo_lock";
        if provenance.cargo.cargo_lock_sha256.is_some() {
            self.pass(requirement_id);
            return;
        }

        self.fail(
            requirement_id,
            PolicyFindingV1::error(
                "policy.build_provenance.cargo_lock_missing",
                "build provenance does not include Cargo.lock evidence",
                requirement_id,
                ExitClassV1::MissingRequiredEvidence,
            ),
        );
    }

    fn evaluate_wasm_gzip(&mut self, provenance: &BuildProvenanceV1) {
        let requirement_id = "build_provenance.require_wasm_gzip";
        if provenance
            .artifacts
            .iter()
            .any(|artifact| artifact.artifact_kind == ArtifactProvenanceKindV1::WasmGzip)
        {
            self.pass(requirement_id);
            return;
        }

        self.fail(
            requirement_id,
            PolicyFindingV1::error(
                "policy.build_provenance.wasm_gzip_missing",
                "build provenance does not include a gzip Wasm artifact",
                requirement_id,
                ExitClassV1::MissingRequiredEvidence,
            ),
        );
    }

    fn evaluate_sha256(&mut self, provenance: &BuildProvenanceV1) {
        let requirement_id = "build_provenance.require_sha256";
        if !provenance.artifacts.is_empty()
            && provenance.artifacts.iter().all(|artifact| {
                artifact.hash_algorithm == "sha256" && is_sha256_hex(&artifact.sha256)
            })
        {
            self.pass(requirement_id);
            return;
        }

        self.fail(
            requirement_id,
            PolicyFindingV1::error(
                "policy.build_provenance.sha256_missing_or_invalid",
                "build provenance has missing or invalid artifact SHA-256 evidence",
                requirement_id,
                ExitClassV1::MissingRequiredEvidence,
            )
            .actual(serde_json::json!(
                provenance
                    .artifacts
                    .iter()
                    .map(|artifact| serde_json::json!({
                        "artifact_kind": artifact.artifact_kind,
                        "hash_algorithm": artifact.hash_algorithm,
                        "sha256": artifact.sha256,
                    }))
                    .collect::<Vec<_>>()
            )),
        );
    }

    fn evaluate_package_identity_matches_target(
        &mut self,
        provenance: &BuildProvenanceV1,
        target: &EvidenceTargetV1,
    ) {
        let requirement_id = "build_provenance.require_package_identity_matches_target";
        let target_fleet = target.fleet.as_deref();
        let target_role = target.role.as_deref();
        let package_fleet = provenance.cargo.package_metadata_fleet.as_str();
        let package_role = provenance.cargo.package_metadata_role.as_str();

        if target_fleet == Some(package_fleet) && target_role == Some(package_role) {
            self.pass(requirement_id);
            return;
        }

        let exit_class = if target_fleet.is_none() || target_role.is_none() {
            ExitClassV1::MissingRequiredEvidence
        } else {
            ExitClassV1::BlockedByPolicy
        };
        self.fail(
            requirement_id,
            PolicyFindingV1::error(
                "policy.build_provenance.package_identity_mismatch",
                "build provenance package metadata does not match the envelope target",
                requirement_id,
                exit_class,
            )
            .expected(serde_json::json!({
                "target_fleet": target_fleet,
                "target_role": target_role,
            }))
            .actual(serde_json::json!({
                "package_metadata_fleet": package_fleet,
                "package_metadata_role": package_role,
            })),
        );
    }

    fn fail_enabled_build_provenance_rules(
        &mut self,
        rules: &PolicyBuildProvenanceRulesV1,
        code: &str,
        message: &str,
        exit_class: ExitClassV1,
        expected: serde_json::Value,
        actual: serde_json::Value,
    ) {
        for requirement_id in build_provenance_requirement_ids(rules) {
            self.fail(
                requirement_id,
                PolicyFindingV1::error(code, message, requirement_id, exit_class)
                    .expected(expected.clone())
                    .actual(actual.clone()),
            );
        }
    }

    fn pass(&mut self, requirement_id: &str) {
        self.requirements.push(PolicyRequirementV1 {
            requirement_id: requirement_id.to_string(),
            status: PolicyEvaluationStatusV1::Passed,
            exit_class: ExitClassV1::Success,
            finding_codes: Vec::new(),
        });
    }

    fn fail(&mut self, requirement_id: &str, finding: PolicyFindingV1) {
        let finding_code = finding.code.clone();
        let exit_class = finding.exit_class();
        self.findings.push(finding);
        self.requirements.push(PolicyRequirementV1 {
            requirement_id: requirement_id.to_string(),
            status: PolicyEvaluationStatusV1::Failed,
            exit_class,
            finding_codes: vec![finding_code],
        });
    }
}

impl PolicyFindingV1 {
    fn error(
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

    fn warning(code: &str, message: impl Into<String>, requirement_id: &str) -> Self {
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

    fn expected(mut self, expected: serde_json::Value) -> Self {
        self.expected = Some(expected);
        self
    }

    fn actual(mut self, actual: serde_json::Value) -> Self {
        self.actual = Some(actual);
        self
    }

    fn exit_class(&self) -> ExitClassV1 {
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

const fn is_success_exit_class(exit_class: ExitClassV1) -> bool {
    matches!(
        exit_class,
        ExitClassV1::Success | ExitClassV1::SuccessWithWarnings
    )
}

fn message_codes(messages: &[crate::evidence_envelope::EvidenceMessageV1]) -> Vec<String> {
    messages
        .iter()
        .map(|message| message.code.clone())
        .collect()
}

fn build_provenance_requirement_ids(rules: &PolicyBuildProvenanceRulesV1) -> Vec<&'static str> {
    rules
        .rules
        .iter()
        .map(|rule| match rule {
            PolicyBuildProvenanceRuleV1::CleanSource => "build_provenance.require_clean_source",
            PolicyBuildProvenanceRuleV1::CargoLock => "build_provenance.require_cargo_lock",
            PolicyBuildProvenanceRuleV1::WasmGzip => "build_provenance.require_wasm_gzip",
            PolicyBuildProvenanceRuleV1::Sha256 => "build_provenance.require_sha256",
            PolicyBuildProvenanceRuleV1::PackageIdentityMatchesTarget => {
                "build_provenance.require_package_identity_matches_target"
            }
        })
        .collect()
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_provenance::{
        ArtifactProvenanceV1, BuildProvenanceStatusV1, BuildScriptInputStateV1, CargoProvenanceV1,
        SourceProvenanceV1, SourceVcsV1,
    };
    use crate::evidence_envelope::{
        CommandProvenanceV1, EvidenceMessageSeverityV1, EvidenceMessageV1, EvidenceTargetKindV1,
        InputPathDisplayV1, PayloadSchemaStabilityV1, evidence_envelope_schema,
        policy_gate_report_schema,
    };
    use crate::test_support::temp_dir;
    use serde_json::json;
    use std::fs;

    #[test]
    fn policy_parser_accepts_minimal_policy() {
        let policy = parse_ci_policy_v1(MINIMAL_POLICY).expect("parse policy");

        assert_eq!(policy.schema_version, 1);
        assert_eq!(
            policy.envelope.required_schema,
            "canic.evidence_envelope.v1"
        );
        assert_eq!(policy.exit_class.allowed, vec![ExitClassV1::Success]);
    }

    #[test]
    fn policy_parser_rejects_unknown_keys_and_empty_allow_lists() {
        let unknown = parse_ci_policy_v1(
            r#"
schema_version = 1
unexpected = true

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]
"#,
        )
        .expect_err("unknown policy keys fail");
        assert!(unknown.to_string().contains("failed to parse policy TOML"));

        let empty = parse_ci_policy_v1(
            r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = []
"#,
        )
        .expect_err("empty allow list fails");
        assert!(empty.to_string().contains("exit_class.allowed"));
    }

    #[test]
    fn policy_parser_accepts_build_provenance_rules() {
        let policy = parse_ci_policy_v1(BUILD_PROVENANCE_POLICY).expect("parse policy");

        let rules = policy
            .build_provenance
            .expect("build provenance rules present");
        assert!(rules.is_enabled(PolicyBuildProvenanceRuleV1::CleanSource));
        assert!(rules.is_enabled(PolicyBuildProvenanceRuleV1::CargoLock));
        assert!(rules.is_enabled(PolicyBuildProvenanceRuleV1::WasmGzip));
        assert!(rules.is_enabled(PolicyBuildProvenanceRuleV1::Sha256));
        assert!(rules.is_enabled(PolicyBuildProvenanceRuleV1::PackageIdentityMatchesTarget));
    }

    #[test]
    fn policy_parser_rejects_empty_build_provenance_rules() {
        let err = parse_ci_policy_v1(
            r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]

[build_provenance]
"#,
        )
        .expect_err("empty build provenance rules fail");

        assert!(err.to_string().contains("build_provenance"));
    }

    #[test]
    fn policy_parser_rejects_unknown_build_provenance_keys() {
        let err = parse_ci_policy_v1(
            r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]

[build_provenance]
require_magic = true
"#,
        )
        .expect_err("unknown build provenance keys fail");

        assert!(err.to_string().contains("failed to parse policy TOML"));
    }

    #[test]
    fn minimal_policy_passes_success_envelope() {
        let root = temp_dir("canic-policy-pass");
        fs::create_dir_all(&root).expect("create root");
        let policy_path = root.join("policy.toml");
        let envelope_path = root.join("envelope.json");
        fs::write(&policy_path, MINIMAL_POLICY).expect("write policy");
        fs::write(&envelope_path, "{}").expect("write envelope placeholder");

        let report = evaluate_policy_gate(PolicyGateRequest {
            policy_source: MINIMAL_POLICY,
            policy_path: &policy_path,
            envelope_path: &envelope_path,
            fingerprint_root: &root,
            envelope: sample_envelope(),
        })
        .expect("evaluate policy");

        fs::remove_dir_all(root).expect("clean");
        assert_eq!(report.policy_status, PolicyEvaluationStatusV1::Passed);
        assert_eq!(report.gate_exit_class, ExitClassV1::Success);
        assert!(report.findings.is_empty());
        assert_eq!(
            report.evaluated_payload_schema.id,
            "canic.build_provenance.v1"
        );
    }

    #[test]
    fn policy_rejects_disallowed_exit_class_but_preserves_evaluated_class() {
        let mut envelope = sample_envelope();
        envelope.exit_class = ExitClassV1::SuccessWithWarnings;

        let report = evaluate_policy_for_test(MINIMAL_POLICY, envelope);

        assert_eq!(
            report.evaluated_envelope_exit_class,
            ExitClassV1::SuccessWithWarnings
        );
        assert_eq!(report.policy_status, PolicyEvaluationStatusV1::Failed);
        assert_eq!(report.gate_exit_class, ExitClassV1::BlockedByPolicy);
        assert_eq!(report.findings[0].code, "policy.exit_class.disallowed");
    }

    #[test]
    fn policy_accepts_success_with_warnings_when_allowed() {
        let mut envelope = sample_envelope();
        envelope.exit_class = ExitClassV1::SuccessWithWarnings;
        envelope.summary.warnings.push(EvidenceMessageV1::new(
            "test.warning",
            "warning",
            EvidenceMessageSeverityV1::Warning,
        ));

        let report = evaluate_policy_for_test(
            r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success", "success_with_warnings"]
"#,
            envelope,
        );

        assert_eq!(report.policy_status, PolicyEvaluationStatusV1::Passed);
        assert_eq!(report.gate_exit_class, ExitClassV1::SuccessWithWarnings);
    }

    #[test]
    fn summary_conflicts_and_missing_required_inputs_map_to_policy_exit_classes() {
        let mut conflict = sample_envelope();
        conflict
            .summary
            .evidence_conflicts
            .push(EvidenceMessageV1::new(
                "test.conflict",
                "conflict",
                EvidenceMessageSeverityV1::Error,
            ));
        let conflict_report = evaluate_policy_for_test(SUMMARY_POLICY, conflict);

        assert_eq!(
            conflict_report.gate_exit_class,
            ExitClassV1::EvidenceConflict
        );
        assert_eq!(
            conflict_report.findings[0].code,
            "policy.summary.evidence_conflict"
        );

        let missing_report = evaluate_policy_for_test(
            r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]

[[required_input]]
kind = "canic_config"
schema = "canic.config.toml"
"#,
            sample_envelope(),
        );

        assert_eq!(
            missing_report.gate_exit_class,
            ExitClassV1::MissingRequiredEvidence
        );
        assert_eq!(
            missing_report.findings[0].code,
            "policy.required_input.missing"
        );
    }

    #[test]
    fn required_input_passes_on_matching_kind_and_schema() {
        let mut envelope = sample_envelope();
        envelope.inputs.push(InputFingerprintV1 {
            kind: "canic_config".to_string(),
            path: Some("canic.toml".to_string()),
            path_display: InputPathDisplayV1::Relative,
            sha256: None,
            size_bytes: None,
            modified_unix_secs: None,
            schema: Some(PayloadSchemaRefV1::stable("canic.config.toml", "1")),
            note: None,
        });

        let report = evaluate_policy_for_test(
            r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]

[[required_input]]
kind = "canic_config"
schema = "canic.config.toml"
"#,
            envelope,
        );

        assert_eq!(report.policy_status, PolicyEvaluationStatusV1::Passed);
        assert_eq!(report.gate_exit_class, ExitClassV1::Success);
    }

    #[test]
    fn build_provenance_policy_passes_matching_payload() {
        let report = evaluate_policy_for_test(BUILD_PROVENANCE_POLICY, sample_envelope());

        assert_eq!(report.policy_status, PolicyEvaluationStatusV1::Passed);
        assert_eq!(report.gate_exit_class, ExitClassV1::Success);
        assert!(report.findings.is_empty());
        assert!(report.requirements.iter().any(
            |requirement| requirement.requirement_id == "build_provenance.require_clean_source"
        ));
    }

    #[test]
    fn build_provenance_policy_rejects_dirty_or_unknown_source() {
        let mut dirty = sample_build_provenance_payload();
        dirty.source.dirty = Some(true);
        dirty.source.dirty_policy = SourceDirtyPolicyV1::DirtyRecorded;
        let dirty_report = evaluate_policy_for_test(
            BUILD_PROVENANCE_POLICY,
            sample_envelope_with_payload(serde_json::to_value(dirty).expect("payload json")),
        );

        assert_eq!(dirty_report.gate_exit_class, ExitClassV1::BlockedByPolicy);
        assert!(
            dirty_report
                .findings
                .iter()
                .any(|finding| finding.code == "policy.build_provenance.source_not_clean")
        );

        let mut unknown = sample_build_provenance_payload();
        unknown.source.vcs = SourceVcsV1::Unknown;
        unknown.source.dirty = None;
        unknown.source.dirty_policy = SourceDirtyPolicyV1::Unknown;
        let unknown_report = evaluate_policy_for_test(
            BUILD_PROVENANCE_POLICY,
            sample_envelope_with_payload(serde_json::to_value(unknown).expect("payload json")),
        );

        assert_eq!(unknown_report.gate_exit_class, ExitClassV1::BlockedByPolicy);
    }

    #[test]
    fn build_provenance_policy_requires_cargo_lock_and_gzip_wasm() {
        let mut no_lock = sample_build_provenance_payload();
        no_lock.cargo.cargo_lock_sha256 = None;
        let no_lock_report = evaluate_policy_for_test(
            BUILD_PROVENANCE_POLICY,
            sample_envelope_with_payload(serde_json::to_value(no_lock).expect("payload json")),
        );

        assert_eq!(
            no_lock_report.gate_exit_class,
            ExitClassV1::MissingRequiredEvidence
        );
        assert!(
            no_lock_report
                .findings
                .iter()
                .any(|finding| finding.code == "policy.build_provenance.cargo_lock_missing")
        );

        let mut no_gzip = sample_build_provenance_payload();
        no_gzip
            .artifacts
            .retain(|artifact| artifact.artifact_kind != ArtifactProvenanceKindV1::WasmGzip);
        let no_gzip_report = evaluate_policy_for_test(
            BUILD_PROVENANCE_POLICY,
            sample_envelope_with_payload(serde_json::to_value(no_gzip).expect("payload json")),
        );

        assert_eq!(
            no_gzip_report.gate_exit_class,
            ExitClassV1::MissingRequiredEvidence
        );
        assert!(
            no_gzip_report
                .findings
                .iter()
                .any(|finding| finding.code == "policy.build_provenance.wasm_gzip_missing")
        );
    }

    #[test]
    fn build_provenance_policy_requires_sha256_artifact_evidence() {
        let mut payload = sample_build_provenance_payload();
        payload.artifacts[0].sha256 = "not-a-sha".to_string();
        let report = evaluate_policy_for_test(
            BUILD_PROVENANCE_POLICY,
            sample_envelope_with_payload(serde_json::to_value(payload).expect("payload json")),
        );

        assert_eq!(report.gate_exit_class, ExitClassV1::MissingRequiredEvidence);
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.code == "policy.build_provenance.sha256_missing_or_invalid")
        );
    }

    #[test]
    fn build_provenance_policy_requires_package_identity_to_match_target() {
        let mut payload = sample_build_provenance_payload();
        payload.cargo.package_metadata_role = "other".to_string();
        let report = evaluate_policy_for_test(
            BUILD_PROVENANCE_POLICY,
            sample_envelope_with_payload(serde_json::to_value(payload).expect("payload json")),
        );

        assert_eq!(report.gate_exit_class, ExitClassV1::BlockedByPolicy);
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.code == "policy.build_provenance.package_identity_mismatch")
        );
    }

    #[test]
    fn build_provenance_policy_rejects_wrong_or_invalid_payload() {
        let mut wrong_schema = sample_envelope();
        wrong_schema.payload_schema = PayloadSchemaRefV1::stable("canic.adoption_report.v1", "1");
        let wrong_schema_report = evaluate_policy_for_test(BUILD_PROVENANCE_POLICY, wrong_schema);

        assert_eq!(
            wrong_schema_report.gate_exit_class,
            ExitClassV1::BlockedByPolicy
        );
        assert!(
            wrong_schema_report
                .findings
                .iter()
                .any(|finding| finding.code == "policy.build_provenance.payload_schema")
        );

        let invalid_report = evaluate_policy_for_test(
            BUILD_PROVENANCE_POLICY,
            sample_envelope_with_payload(json!({ "schema_version": 1 })),
        );

        assert_eq!(invalid_report.gate_exit_class, ExitClassV1::BlockedByPolicy);
        assert!(
            invalid_report
                .findings
                .iter()
                .any(|finding| finding.code == "policy.build_provenance.invalid_payload")
        );
    }

    #[test]
    fn project_evidence_manifest_gate_evaluates_required_envelope() {
        let root = temp_dir("canic-policy-manifest-pass");
        fs::create_dir_all(&root).expect("create root");
        let policy_path = root.join("policy.toml");
        let manifest_path = root.join("evidence.toml");
        let envelope_path = root.join("build.json");
        fs::write(&policy_path, BUILD_PROVENANCE_POLICY).expect("write policy");
        fs::write(
            &envelope_path,
            serde_json::to_vec(&sample_envelope()).expect("encode envelope"),
        )
        .expect("write envelope");
        let manifest_source = sample_manifest_source("build.json", true);
        fs::write(&manifest_path, &manifest_source).expect("write manifest");

        let report = evaluate_project_evidence_manifest_gate(ProjectEvidenceManifestGateRequest {
            policy_source: BUILD_PROVENANCE_POLICY,
            policy_path: &policy_path,
            manifest_source: &manifest_source,
            manifest_path: &manifest_path,
            fingerprint_root: &root,
        })
        .expect("evaluate manifest gate");

        fs::remove_dir_all(root).expect("clean");
        assert_eq!(report.policy_status, PolicyEvaluationStatusV1::Passed);
        assert_eq!(report.gate_exit_class, ExitClassV1::Success);
        assert_eq!(report.evidence.len(), 1);
        assert_eq!(report.evidence[0].status, PolicyEvaluationStatusV1::Passed);
        assert!(report.evidence[0].policy_report.is_some());
    }

    #[test]
    fn project_evidence_manifest_gate_reports_missing_required_and_optional_evidence() {
        let required_report = evaluate_manifest_gate_for_test(
            &sample_manifest_source("missing.json", true),
            BUILD_PROVENANCE_POLICY,
        );

        assert_eq!(
            required_report.gate_exit_class,
            ExitClassV1::MissingRequiredEvidence
        );
        assert_eq!(
            required_report.evidence[0].status,
            PolicyEvaluationStatusV1::Failed
        );
        assert_eq!(
            required_report.evidence[0].findings[0].code,
            "policy.manifest.required_evidence_missing"
        );

        let optional_report = evaluate_manifest_gate_for_test(
            &sample_manifest_source("missing.json", false),
            BUILD_PROVENANCE_POLICY,
        );

        assert_eq!(
            optional_report.gate_exit_class,
            ExitClassV1::SuccessWithWarnings
        );
        assert_eq!(
            optional_report.evidence[0].status,
            PolicyEvaluationStatusV1::Passed
        );
        assert_eq!(
            optional_report.evidence[0].findings[0].code,
            "policy.manifest.optional_evidence_missing"
        );
    }

    #[test]
    fn project_evidence_manifest_gate_checks_target_and_payload_schema_expectations() {
        let mut wrong_schema = sample_envelope();
        wrong_schema.payload_schema = PayloadSchemaRefV1::stable("canic.other.v1", "1");
        let wrong_schema_report = evaluate_manifest_gate_with_envelope(
            &sample_manifest_source("build.json", true),
            wrong_schema,
        );

        assert_eq!(
            wrong_schema_report.gate_exit_class,
            ExitClassV1::BlockedByPolicy
        );
        assert!(
            wrong_schema_report.evidence[0]
                .findings
                .iter()
                .any(|finding| finding.code == "policy.manifest.payload_schema_mismatch")
        );

        let mut wrong_target = sample_envelope();
        wrong_target.target.role = Some("other".to_string());
        let wrong_target_report = evaluate_manifest_gate_with_envelope(
            &sample_manifest_source("build.json", true),
            wrong_target,
        );

        assert_eq!(
            wrong_target_report.gate_exit_class,
            ExitClassV1::BlockedByPolicy
        );
        assert!(
            wrong_target_report.evidence[0]
                .findings
                .iter()
                .any(|finding| finding.code == "policy.manifest.target_mismatch")
        );
    }

    #[test]
    fn project_evidence_manifest_rejects_duplicate_evidence_paths() {
        let manifest_source = r#"
schema_version = 1

[project]
name = "demo"
root = "."

[[evidence]]
kind = "build_provenance"
path = "build.json"
required = true
payload_schema = "canic.build_provenance.v1"

[evidence.target]
fleet = "demo"
role = "app"

[[evidence]]
kind = "deployment_check"
path = " ./build.json "
required = true
payload_schema = "canic.deployment_check.v1"

[evidence.target]
deployment = "demo-staging"
"#;

        let error = parse_project_evidence_manifest_v1(manifest_source)
            .expect_err("duplicate evidence path should fail");

        assert!(matches!(error, PolicyGateError::InvalidPolicy(_)));
        assert!(
            error
                .to_string()
                .contains("duplicates an earlier evidence path")
        );
    }

    #[test]
    fn policy_gate_report_schema_is_stable() {
        assert_eq!(
            policy_gate_report_schema(),
            PayloadSchemaRefV1 {
                id: "canic.policy_gate_report.v1".to_string(),
                version: "1".to_string(),
                stability: PayloadSchemaStabilityV1::Stable,
            }
        );
    }

    fn evaluate_policy_for_test(
        policy_source: &str,
        envelope: EvidenceEnvelopeV1,
    ) -> PolicyGateReportV1 {
        let root = temp_dir("canic-policy-test");
        fs::create_dir_all(&root).expect("create root");
        let policy_path = root.join("policy.toml");
        let envelope_path = root.join("envelope.json");
        fs::write(&policy_path, policy_source).expect("write policy");
        fs::write(&envelope_path, "{}").expect("write envelope placeholder");

        let report = evaluate_policy_gate(PolicyGateRequest {
            policy_source,
            policy_path: &policy_path,
            envelope_path: &envelope_path,
            fingerprint_root: &root,
            envelope,
        })
        .expect("evaluate policy");

        fs::remove_dir_all(root).expect("clean");
        report
    }

    fn evaluate_manifest_gate_for_test(
        manifest_source: &str,
        policy_source: &str,
    ) -> ProjectEvidenceGateReportV1 {
        let root = temp_dir("canic-policy-manifest-test");
        fs::create_dir_all(&root).expect("create root");
        let policy_path = root.join("policy.toml");
        let manifest_path = root.join("evidence.toml");
        fs::write(&policy_path, policy_source).expect("write policy");
        fs::write(&manifest_path, manifest_source).expect("write manifest");

        let report = evaluate_project_evidence_manifest_gate(ProjectEvidenceManifestGateRequest {
            policy_source,
            policy_path: &policy_path,
            manifest_source,
            manifest_path: &manifest_path,
            fingerprint_root: &root,
        })
        .expect("evaluate manifest gate");

        fs::remove_dir_all(root).expect("clean");
        report
    }

    fn evaluate_manifest_gate_with_envelope(
        manifest_source: &str,
        envelope: EvidenceEnvelopeV1,
    ) -> ProjectEvidenceGateReportV1 {
        let root = temp_dir("canic-policy-manifest-envelope-test");
        fs::create_dir_all(&root).expect("create root");
        let policy_path = root.join("policy.toml");
        let manifest_path = root.join("evidence.toml");
        let envelope_path = root.join("build.json");
        fs::write(&policy_path, BUILD_PROVENANCE_POLICY).expect("write policy");
        fs::write(&manifest_path, manifest_source).expect("write manifest");
        fs::write(
            &envelope_path,
            serde_json::to_vec(&envelope).expect("encode envelope"),
        )
        .expect("write envelope");

        let report = evaluate_project_evidence_manifest_gate(ProjectEvidenceManifestGateRequest {
            policy_source: BUILD_PROVENANCE_POLICY,
            policy_path: &policy_path,
            manifest_source,
            manifest_path: &manifest_path,
            fingerprint_root: &root,
        })
        .expect("evaluate manifest gate");

        fs::remove_dir_all(root).expect("clean");
        report
    }

    fn sample_envelope() -> EvidenceEnvelopeV1 {
        sample_envelope_with_payload(
            serde_json::to_value(sample_build_provenance_payload()).expect("payload json"),
        )
    }

    fn sample_envelope_with_payload(payload: serde_json::Value) -> EvidenceEnvelopeV1 {
        EvidenceEnvelopeV1 {
            envelope_schema: evidence_envelope_schema(),
            canic_version: env!("CARGO_PKG_VERSION").to_string(),
            command: CommandProvenanceV1 {
                name: "canic build".to_string(),
                argv_normalized: Vec::new(),
                argv_redactions: Vec::new(),
                format: "envelope-json".to_string(),
            },
            target: EvidenceTargetV1 {
                kind: EvidenceTargetKindV1::Artifact,
                deployment: None,
                fleet: Some("demo".to_string()),
                role: Some("app".to_string()),
                profile: None,
                network: None,
            },
            generated_at: "unix:1".to_string(),
            source_config: None,
            inputs: Vec::new(),
            payload_schema: PayloadSchemaRefV1::stable("canic.build_provenance.v1", "1"),
            payload_sha256: Some("0".repeat(64)),
            payload,
            summary: EvidenceSummaryV1 {
                warnings: Vec::new(),
                blocked_actions: Vec::new(),
                missing_or_stale_evidence: Vec::new(),
                evidence_conflicts: Vec::new(),
            },
            exit_class: ExitClassV1::Success,
        }
    }

    fn sample_build_provenance_payload() -> BuildProvenanceV1 {
        BuildProvenanceV1 {
            schema_version: 1,
            generated_at: "unix:1".to_string(),
            canic_version: env!("CARGO_PKG_VERSION").to_string(),
            command: CommandProvenanceV1 {
                name: "canic build".to_string(),
                argv_normalized: vec![
                    "canic".to_string(),
                    "build".to_string(),
                    "demo".to_string(),
                    "app".to_string(),
                ],
                argv_redactions: Vec::new(),
                format: "provenance".to_string(),
            },
            build_status: BuildProvenanceStatusV1::Success,
            source: SourceProvenanceV1 {
                schema_version: 1,
                vcs: SourceVcsV1::Git,
                revision: Some("abc123".to_string()),
                branch: Some("main".to_string()),
                dirty: Some(false),
                dirty_policy: SourceDirtyPolicyV1::Clean,
                dirty_summary_digest: None,
                dirty_summary_algorithm: None,
            },
            cargo: CargoProvenanceV1 {
                cargo_lock_sha256: Some("1".repeat(64)),
                package_manifest_sha256: Some("2".repeat(64)),
                package_name: "demo_app".to_string(),
                package_manifest: "fleets/demo/app/Cargo.toml".to_string(),
                package_metadata_fleet: "demo".to_string(),
                package_metadata_role: "app".to_string(),
                rustc_version: Some("rustc 1.88.0".to_string()),
                cargo_version: Some("cargo 1.88.0".to_string()),
                target: Some("wasm32-unknown-unknown".to_string()),
                profile: "fast".to_string(),
                features: Vec::new(),
                default_features: None,
                rustflags_digest: None,
                rustflags_digest_algorithm: None,
                cargo_config_fingerprints: Vec::new(),
                build_script_inputs: BuildScriptInputStateV1::NotRecorded,
            },
            artifacts: vec![
                sample_artifact(ArtifactProvenanceKindV1::Wasm, "a"),
                sample_artifact(ArtifactProvenanceKindV1::WasmGzip, "b"),
            ],
            warnings: Vec::new(),
        }
    }

    fn sample_artifact(kind: ArtifactProvenanceKindV1, hash_char: &str) -> ArtifactProvenanceV1 {
        ArtifactProvenanceV1 {
            role: "app".to_string(),
            fleet: "demo".to_string(),
            artifact_kind: kind,
            path: Some("target/app.wasm.gz".to_string()),
            path_display: InputPathDisplayV1::Relative,
            hash_algorithm: "sha256".to_string(),
            sha256: hash_char.repeat(64),
            size_bytes: 123,
            produced_by: "canic build".to_string(),
        }
    }

    fn sample_manifest_source(path: &str, required: bool) -> String {
        format!(
            r#"
schema_version = 1

[project]
name = "demo"
root = "."

[[evidence]]
kind = "build_provenance"
path = "{path}"
required = {required}
payload_schema = "canic.build_provenance.v1"

[evidence.target]
fleet = "demo"
role = "app"
"#
        )
    }

    const MINIMAL_POLICY: &str = r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]
"#;

    const SUMMARY_POLICY: &str = r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success", "success_with_warnings"]

[summary]
fail_on_evidence_conflicts = true
fail_on_blocked_actions = true
allow_missing_or_stale_evidence = false
"#;

    const BUILD_PROVENANCE_POLICY: &str = r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]

[build_provenance]
require_clean_source = true
require_cargo_lock = true
require_wasm_gzip = true
require_sha256 = true
require_package_identity_matches_target = true
"#;
}
