//! Passive CI policy gates over stable evidence envelopes.

use crate::evidence_envelope::{
    EvidenceEnvelopeV1, EvidenceSummaryV1, EvidenceTargetV1, ExitClassV1, InputFingerprintV1,
    PayloadSchemaRefV1, PayloadSchemaStabilityV1, combine_exit_classes, evidence_envelope_schema,
    file_input_fingerprint,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
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
    for (index, rule) in policy.required_input.iter().enumerate() {
        ensure_nonempty(&format!("required_input[{index}].kind"), &rule.kind)?;
        if let Some(schema) = &rule.schema {
            ensure_nonempty(&format!("required_input[{index}].schema"), schema)?;
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
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

    fn sample_envelope() -> EvidenceEnvelopeV1 {
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
            payload: json!({ "schema_version": 1 }),
            summary: EvidenceSummaryV1 {
                warnings: Vec::new(),
                blocked_actions: Vec::new(),
                missing_or_stale_evidence: Vec::new(),
                evidence_conflicts: Vec::new(),
            },
            exit_class: ExitClassV1::Success,
        }
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
}
