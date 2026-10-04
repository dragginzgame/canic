//! Passive CI policy gates over stable evidence envelopes.

use crate::evidence_envelope::{bytes_input_fingerprint, evidence_envelope_schema};

mod evaluation;
mod manifest_gate;
mod model;
mod validation;

use evaluation::evaluate_policy;
pub use manifest_gate::evaluate_workspace_evidence_manifest_gate;
pub use model::{
    CiPolicyV1, PolicyBuildProvenanceRulesV1, PolicyEnvelopeRulesV1, PolicyEvaluationStatusV1,
    PolicyExitClassRulesV1, PolicyFindingSeverityV1, PolicyFindingV1, PolicyGateError,
    PolicyGateReportV1, PolicyGateRequest, PolicyRequiredInputRuleV1, PolicyRequirementV1,
    PolicySummaryRulesV1, WorkspaceEvidenceGateEntryReportV1, WorkspaceEvidenceGateReportV1,
    WorkspaceEvidenceManifestEntryV1, WorkspaceEvidenceManifestGateRequest,
    WorkspaceEvidenceManifestTargetV1, WorkspaceEvidenceManifestV1,
    WorkspaceEvidenceManifestWorkspaceV1,
};
use validation::{validate_ci_policy_v1, validate_workspace_evidence_manifest_v1};

pub fn parse_ci_policy_v1(source: &str) -> Result<CiPolicyV1, PolicyGateError> {
    let policy = toml::from_str::<CiPolicyV1>(source)?;
    validate_ci_policy_v1(&policy)?;
    Ok(policy)
}

pub fn parse_workspace_evidence_manifest_v1(
    source: &str,
) -> Result<WorkspaceEvidenceManifestV1, PolicyGateError> {
    let manifest = toml::from_str::<WorkspaceEvidenceManifestV1>(source)?;
    validate_workspace_evidence_manifest_v1(&manifest)?;
    Ok(manifest)
}

pub fn evaluate_policy_gate(
    request: PolicyGateRequest<'_>,
) -> Result<PolicyGateReportV1, PolicyGateError> {
    let policy = parse_ci_policy_v1(request.policy_source)?;
    let envelope = serde_json::from_str(request.envelope_source)?;
    let policy_file_fingerprint = bytes_input_fingerprint(
        "ci_policy",
        request.policy_path,
        request.fingerprint_root,
        request.policy_source.as_bytes(),
        None,
        None,
    );
    let evaluated_envelope_fingerprint = bytes_input_fingerprint(
        "evidence_envelope",
        request.envelope_path,
        request.fingerprint_root,
        request.envelope_source.as_bytes(),
        Some(evidence_envelope_schema()),
        None,
    );
    Ok(evaluate_policy(
        &policy,
        policy_file_fingerprint,
        evaluated_envelope_fingerprint,
        envelope,
    ))
}

#[cfg(test)]
mod tests;
