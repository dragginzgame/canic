//! Passive CI policy gates over stable evidence envelopes.

use crate::evidence_envelope::{evidence_envelope_schema, file_input_fingerprint};

mod evaluation;
mod manifest_gate;
mod model;
mod validation;

use evaluation::evaluate_policy;
pub use manifest_gate::evaluate_project_evidence_manifest_gate;
#[cfg(test)]
use model::PolicyBuildProvenanceRuleV1;
pub use model::{
    CiPolicyV1, PolicyBuildProvenanceRulesV1, PolicyEnvelopeRulesV1, PolicyEvaluationStatusV1,
    PolicyExitClassRulesV1, PolicyFindingSeverityV1, PolicyFindingV1, PolicyGateError,
    PolicyGateReportV1, PolicyGateRequest, PolicyRequiredInputRuleV1, PolicyRequirementV1,
    PolicySummaryRulesV1, ProjectEvidenceGateEntryReportV1, ProjectEvidenceGateReportV1,
    ProjectEvidenceManifestEntryV1, ProjectEvidenceManifestGateRequest,
    ProjectEvidenceManifestProjectV1, ProjectEvidenceManifestTargetV1, ProjectEvidenceManifestV1,
};
use validation::{validate_ci_policy_v1, validate_project_evidence_manifest_v1};

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

#[cfg(test)]
mod tests;
