use super::model::PolicyBuildProvenanceRuleV1;
use super::{
    CiPolicyV1, PolicyBuildProvenanceRulesV1, PolicyEvaluationStatusV1, PolicyFindingV1,
    PolicyGateReportV1, PolicyRequiredInputRuleV1, PolicyRequirementV1, PolicySummaryRulesV1,
};
use crate::{
    build_provenance::{
        ArtifactProvenanceKindV1, BUILD_PROVENANCE_SCHEMA_ID, BuildProvenanceV1,
        SourceDirtyPolicyV1,
    },
    evidence_envelope::{
        EvidenceEnvelopeV1, EvidenceMessageV1, EvidenceSummaryV1, EvidenceTargetV1, ExitClassV1,
        InputFingerprintV1, combine_exit_classes,
    },
};

pub(super) fn evaluate_policy(
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

const fn is_success_exit_class(exit_class: ExitClassV1) -> bool {
    matches!(
        exit_class,
        ExitClassV1::Success | ExitClassV1::SuccessWithWarnings
    )
}

fn message_codes(messages: &[EvidenceMessageV1]) -> Vec<String> {
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
