use crate::evidence_envelope::{
    EvidenceEnvelopeV1, EvidenceMessageSeverityV1, EvidenceMessageV1, EvidenceSummaryV1,
    EvidenceTargetKindV1, EvidenceTargetV1, ExitClassV1, PayloadSchemaRefV1,
    evidence_envelope_schema, file_input_fingerprint, json_payload_sha256,
};

use super::{
    artifacts::{artifact_provenance, artifact_transform_provenance},
    cargo::cargo_provenance,
    inputs::build_input_fingerprints,
    model::{
        BUILD_PROVENANCE_SCHEMA_ID, BuildProvenanceRequest, BuildProvenanceStatusV1,
        BuildProvenanceV1, SourceVcsV1,
    },
    source::source_provenance,
};

#[must_use]
pub fn build_provenance_schema() -> PayloadSchemaRefV1 {
    PayloadSchemaRefV1::stable(BUILD_PROVENANCE_SCHEMA_ID, "1")
}

pub fn build_provenance_envelope(
    request: &BuildProvenanceRequest,
) -> Result<EvidenceEnvelopeV1, Box<dyn std::error::Error>> {
    let payload = build_provenance_payload(request)?;
    let payload_sha256 = Some(json_payload_sha256(&payload)?);
    let payload_value = serde_json::to_value(&payload)?;
    let summary = EvidenceSummaryV1 {
        warnings: payload.warnings.clone(),
        blocked_actions: Vec::new(),
        missing_or_stale_evidence: Vec::new(),
        evidence_conflicts: Vec::new(),
    };
    let generated_at = payload.generated_at;
    let exit_class = if summary.warnings.is_empty() {
        ExitClassV1::Success
    } else {
        ExitClassV1::SuccessWithWarnings
    };

    Ok(EvidenceEnvelopeV1 {
        envelope_schema: evidence_envelope_schema(),
        canic_version: request.canic_version.clone(),
        command: request.command.clone(),
        target: EvidenceTargetV1 {
            kind: EvidenceTargetKindV1::Artifact,
            deployment: None,
            fleet: Some(request.fleet.clone()),
            role: Some(request.role.clone()),
            profile: Some(request.profile.target_dir_name().to_string()),
            environment: Some(request.environment.clone()),
        },
        generated_at,
        source_config: Some(file_input_fingerprint(
            "canic_config",
            &request.config_path,
            &request.workspace_root,
            Some(PayloadSchemaRefV1::internal("canic.config.toml", "1")),
            None,
        )?),
        inputs: build_input_fingerprints(request)?,
        payload_schema: build_provenance_schema(),
        payload_sha256,
        payload: payload_value,
        summary,
        exit_class,
    })
}

fn build_provenance_payload(
    request: &BuildProvenanceRequest,
) -> Result<BuildProvenanceV1, Box<dyn std::error::Error>> {
    let mut warnings = Vec::new();
    let source = source_provenance(&request.workspace_root);
    if source.dirty == Some(true) {
        warnings.push(EvidenceMessageV1::new(
            "build_provenance.source_dirty",
            "build used uncommitted local source state",
            EvidenceMessageSeverityV1::Warning,
        ));
    }
    if source.vcs == SourceVcsV1::Unknown {
        warnings.push(EvidenceMessageV1::new(
            "build_provenance.source_unknown",
            "source revision could not be read from git",
            EvidenceMessageSeverityV1::Warning,
        ));
    }

    Ok(BuildProvenanceV1 {
        schema_version: 1,
        generated_at: request.generated_at.clone(),
        canic_version: request.canic_version.clone(),
        command: request.command.clone(),
        build_status: BuildProvenanceStatusV1::Success,
        source,
        cargo: cargo_provenance(request)?,
        artifacts: artifact_provenance(request)?,
        transforms: artifact_transform_provenance(request)?,
        warnings,
    })
}
