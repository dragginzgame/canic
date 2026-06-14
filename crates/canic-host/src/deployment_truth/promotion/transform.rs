use super::super::{
    ArtifactSourceV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentPlanV1, PromotionArtifactLevelV1,
    PromotionPlanTransformEvidenceV1, PromotionPlanTransformV1, PromotionReadinessStatusV1,
    PromotionReadinessV1, RoleArtifactSourceKindV1, RoleArtifactSourceV1, RoleArtifactV1,
    RolePromotionInputV1, RolePromotionPlanTransformV1, RolePromotionPolicyV1,
    RolePromotionReadinessV1, SafetyFindingV1, SafetySeverityV1,
};
use super::digest::{
    promotion_plan_lineage_digest, promotion_plan_transform_evidence_digest,
    promotion_readiness_digest,
};
use super::ensure::{
    ensure_digest_requirement, ensure_evidence_field, ensure_evidence_sha256, ensure_field,
    ensure_locator_requirement, ensure_optional_sha256,
    ensure_previous_receipt_lineage_digest_requirement, ensure_previous_receipt_requirement,
    ensure_readiness_field, ensure_readiness_optional_sha256, ensure_readiness_sha256,
    ensure_transform_field,
};
use super::error::{
    PromotionArtifactSourceError, PromotionPlanTransformError, PromotionPlanTransformEvidenceError,
    PromotionReadinessError,
};
use super::identity::{
    artifact_identity_changed, role_materialization_identity_matches,
    role_summary_artifact_identity_changed,
};
use super::request::{
    PromotionPlanTransformEvidenceRequest, PromotionPlanTransformRequest,
    PromotionPlanTransformWithMaterializationRequest, PromotionReadinessRequest,
    PromotionReadinessWithPolicyRequest,
};

pub fn promoted_deployment_plan_from_inputs(
    request: &PromotionPlanTransformRequest,
) -> Result<DeploymentPlanV1, PromotionPlanTransformError> {
    Ok(promoted_deployment_plan_transform_from_inputs(request)?.promoted_plan)
}

pub fn promoted_deployment_plan_transform_from_inputs(
    request: &PromotionPlanTransformRequest,
) -> Result<PromotionPlanTransformV1, PromotionPlanTransformError> {
    ensure_transform_field("promoted_plan_id", &request.promoted_plan_id)?;
    let readiness = promotion_readiness_from_inputs(
        &request.promoted_plan_id,
        &request.target_plan,
        &request.inputs,
    );
    validate_promotion_readiness(&readiness)?;
    if readiness.status == PromotionReadinessStatusV1::Blocked {
        return Err(PromotionPlanTransformError::ReadinessBlocked {
            blocker_count: readiness.blockers.len(),
        });
    }

    let mut promoted_plan = request.target_plan.clone();
    promoted_plan.plan_id.clone_from(&request.promoted_plan_id);
    for input in &request.inputs {
        let Some(role_artifact) = promoted_plan
            .role_artifacts
            .iter_mut()
            .find(|artifact| artifact.role == input.role)
        else {
            return Err(PromotionPlanTransformError::TargetRoleMissing {
                role: input.role.clone(),
            });
        };
        apply_promotion_input_to_role_artifact(role_artifact, input);
    }
    let transform =
        promotion_plan_transform_from_parts(&request.target_plan, promoted_plan, &request.inputs);
    validate_promotion_plan_transform(&transform)?;
    Ok(transform)
}

pub fn promoted_deployment_plan_transform_from_inputs_with_materialization(
    request: &PromotionPlanTransformWithMaterializationRequest,
) -> Result<PromotionPlanTransformV1, PromotionPlanTransformError> {
    let base_request = PromotionPlanTransformRequest {
        promoted_plan_id: request.promoted_plan_id.clone(),
        target_plan: request.target_plan.clone(),
        inputs: request.inputs.clone(),
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&base_request)?;
    super::materialization::attach_source_build_materialization(
        &mut transform,
        &request.inputs,
        &request.materialization_evidence,
    )?;
    refresh_promotion_plan_lineage_digest(&mut transform);
    validate_promotion_plan_transform(&transform)?;
    Ok(transform)
}

pub fn check_promotion_readiness(
    request: &PromotionReadinessRequest,
) -> Result<PromotionReadinessV1, PromotionReadinessError> {
    ensure_readiness_field("readiness_id", &request.readiness_id)?;
    let readiness = promotion_readiness_from_inputs(
        &request.readiness_id,
        &request.target_plan,
        &request.inputs,
    );
    validate_promotion_readiness(&readiness)?;
    Ok(readiness)
}

pub fn check_promotion_readiness_with_policy(
    request: &PromotionReadinessWithPolicyRequest,
) -> Result<PromotionReadinessV1, PromotionReadinessError> {
    ensure_readiness_field("readiness_id", &request.readiness_id)?;
    let readiness = promotion_readiness_from_inputs_with_policy(
        &request.readiness_id,
        &request.target_plan,
        &request.inputs,
        &request.policies,
    );
    validate_promotion_readiness(&readiness)?;
    Ok(readiness)
}

pub fn promotion_plan_transform_evidence(
    request: PromotionPlanTransformEvidenceRequest,
) -> Result<PromotionPlanTransformEvidenceV1, PromotionPlanTransformEvidenceError> {
    ensure_evidence_field("evidence_id", &request.evidence_id)?;
    ensure_evidence_field("generated_at", &request.generated_at)?;
    validate_promotion_plan_transform(&request.transform)?;
    let mut evidence = PromotionPlanTransformEvidenceV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        evidence_id: request.evidence_id,
        promotion_plan_transform_evidence_digest: String::new(),
        generated_at: request.generated_at,
        transform: request.transform,
    };
    evidence.promotion_plan_transform_evidence_digest =
        promotion_plan_transform_evidence_digest(&evidence);
    validate_promotion_plan_transform_evidence(&evidence)?;
    Ok(evidence)
}

pub fn validate_promotion_plan_transform_evidence(
    evidence: &PromotionPlanTransformEvidenceV1,
) -> Result<(), PromotionPlanTransformEvidenceError> {
    if evidence.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(PromotionPlanTransformEvidenceError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found: evidence.schema_version,
        });
    }
    ensure_evidence_field("evidence_id", &evidence.evidence_id)?;
    ensure_evidence_sha256(
        "promotion_plan_transform_evidence_digest",
        &evidence.promotion_plan_transform_evidence_digest,
    )?;
    ensure_evidence_field("generated_at", &evidence.generated_at)?;
    validate_promotion_plan_transform(&evidence.transform)?;
    if evidence.promotion_plan_transform_evidence_digest
        != promotion_plan_transform_evidence_digest(evidence)
    {
        return Err(PromotionPlanTransformEvidenceError::LinkageMismatch {
            field: "promotion_plan_transform_evidence_digest",
        });
    }
    Ok(())
}

pub fn validate_promotion_plan_transform(
    transform: &PromotionPlanTransformV1,
) -> Result<(), PromotionPlanTransformError> {
    if transform.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(PromotionPlanTransformError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found: transform.schema_version,
        });
    }
    ensure_transform_field("transform_id", &transform.transform_id)?;
    ensure_transform_field("target_plan_id", &transform.target_plan_id)?;
    ensure_transform_field("promoted_plan_id", &transform.promoted_plan_id)?;
    ensure_transform_field(
        "promotion_plan_lineage_digest",
        &transform.promotion_plan_lineage_digest,
    )?;
    ensure_transform_field("promoted_plan.plan_id", &transform.promoted_plan.plan_id)?;
    if transform.promoted_plan.plan_id != transform.promoted_plan_id {
        return Err(PromotionPlanTransformError::PromotedPlanIdMismatch {
            expected: transform.promoted_plan_id.clone(),
            found: transform.promoted_plan.plan_id.clone(),
        });
    }
    ensure_unique_transform_roles(&transform.roles)?;
    for role in &transform.roles {
        validate_role_plan_transform(role, &transform.promoted_plan)?;
    }
    let expected = promotion_plan_lineage_digest(
        &transform.target_plan_id,
        &transform.promoted_plan_id,
        &transform.promoted_plan,
        &transform.roles,
    );
    if expected != transform.promotion_plan_lineage_digest {
        return Err(PromotionPlanTransformError::RoleStateMismatch {
            role: "promotion_plan_lineage".to_string(),
            field: "promotion_plan_lineage_digest",
        });
    }
    Ok(())
}

#[must_use]
pub fn promotion_readiness_from_inputs(
    readiness_id: impl Into<String>,
    target_plan: &DeploymentPlanV1,
    inputs: &[RolePromotionInputV1],
) -> PromotionReadinessV1 {
    let mut roles = Vec::with_capacity(inputs.len());
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    for input in inputs {
        let target_artifact = target_plan
            .role_artifacts
            .iter()
            .find(|artifact| artifact.role == input.role);
        let Some(target_artifact) = target_artifact else {
            blockers.push(super::promotion_finding(
                "promotion_target_role_missing",
                format!("target plan does not contain role {}", input.role),
                SafetySeverityV1::HardFailure,
                &input.role,
            ));
            continue;
        };

        let role_readiness = role_promotion_readiness(input, target_artifact);
        collect_role_findings(input, &role_readiness, &mut blockers, &mut warnings);
        roles.push(role_readiness);
    }

    let status = if blockers.is_empty() {
        PromotionReadinessStatusV1::Ready
    } else {
        PromotionReadinessStatusV1::Blocked
    };

    let mut readiness = PromotionReadinessV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        readiness_id: readiness_id.into(),
        promotion_readiness_digest: String::new(),
        target_plan_id: target_plan.plan_id.clone(),
        status,
        roles,
        blockers,
        warnings,
    };
    readiness.promotion_readiness_digest = promotion_readiness_digest(&readiness);
    readiness
}

#[must_use]
pub fn promotion_readiness_from_inputs_with_policy(
    readiness_id: impl Into<String>,
    target_plan: &DeploymentPlanV1,
    inputs: &[RolePromotionInputV1],
    policies: &[RolePromotionPolicyV1],
) -> PromotionReadinessV1 {
    let readiness_id = readiness_id.into();
    let policy_check = super::policy::promotion_policy_check_from_inputs(
        format!("{readiness_id}:policy"),
        inputs,
        policies,
    );
    let mut readiness = promotion_readiness_from_inputs(readiness_id, target_plan, inputs);
    readiness.blockers.extend(policy_check.blockers);
    readiness.status = if readiness.blockers.is_empty() {
        PromotionReadinessStatusV1::Ready
    } else {
        PromotionReadinessStatusV1::Blocked
    };
    readiness.promotion_readiness_digest = promotion_readiness_digest(&readiness);
    readiness
}

pub fn validate_promotion_readiness(
    readiness: &PromotionReadinessV1,
) -> Result<(), PromotionReadinessError> {
    if readiness.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(PromotionReadinessError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found: readiness.schema_version,
        });
    }
    ensure_readiness_field("readiness_id", &readiness.readiness_id)?;
    ensure_readiness_sha256(
        "promotion_readiness_digest",
        &readiness.promotion_readiness_digest,
    )?;
    ensure_readiness_field("target_plan_id", &readiness.target_plan_id)?;
    ensure_readiness_status_matches_blockers(readiness)?;
    ensure_unique_readiness_roles(&readiness.roles)?;
    for role in &readiness.roles {
        validate_role_readiness(role)?;
    }
    validate_readiness_findings(
        "blockers",
        &readiness.blockers,
        SafetySeverityV1::HardFailure,
    )?;
    validate_readiness_findings("warnings", &readiness.warnings, SafetySeverityV1::Warning)?;
    if readiness.promotion_readiness_digest != promotion_readiness_digest(readiness) {
        return Err(PromotionReadinessError::LinkageMismatch {
            field: "promotion_readiness_digest",
        });
    }
    Ok(())
}

pub fn validate_role_artifact_source(
    source: &RoleArtifactSourceV1,
) -> Result<(), PromotionArtifactSourceError> {
    ensure_field("role", &source.role)?;
    ensure_locator_requirement(source)?;
    ensure_previous_receipt_requirement(source)?;
    ensure_digest_requirement(source)?;
    ensure_previous_receipt_lineage_digest_requirement(source)?;
    ensure_optional_sha256(
        "expected_wasm_sha256",
        source.expected_wasm_sha256.as_deref(),
    )?;
    ensure_optional_sha256(
        "expected_wasm_gz_sha256",
        source.expected_wasm_gz_sha256.as_deref(),
    )?;
    ensure_optional_sha256(
        "expected_candid_sha256",
        source.expected_candid_sha256.as_deref(),
    )?;
    ensure_optional_sha256(
        "expected_canonical_embedded_config_sha256",
        source.expected_canonical_embedded_config_sha256.as_deref(),
    )?;
    ensure_optional_sha256(
        "previous_receipt_lineage_digest",
        source.previous_receipt_lineage_digest.as_deref(),
    )?;
    Ok(())
}

fn apply_promotion_input_to_role_artifact(
    role_artifact: &mut RoleArtifactV1,
    input: &RolePromotionInputV1,
) {
    match input.promotion_level {
        PromotionArtifactLevelV1::SealedWasm => {
            role_artifact.source = artifact_source_for_promotion_source(input.source.kind);
            apply_promotion_source_locator(role_artifact, &input.source);
            role_artifact
                .wasm_sha256
                .clone_from(&input.source.expected_wasm_sha256);
            role_artifact
                .wasm_gz_sha256
                .clone_from(&input.source.expected_wasm_gz_sha256);
            role_artifact
                .candid_sha256
                .clone_from(&input.source.expected_candid_sha256);
            role_artifact
                .canonical_embedded_config_sha256
                .clone_from(&input.source.expected_canonical_embedded_config_sha256);
        }
        PromotionArtifactLevelV1::SourceBuild => {}
    }
}

const fn artifact_source_for_promotion_source(kind: RoleArtifactSourceKindV1) -> ArtifactSourceV1 {
    match kind {
        RoleArtifactSourceKindV1::WorkspacePackage => ArtifactSourceV1::LocalBuild,
        RoleArtifactSourceKindV1::CanonicalWasmStoreDefault => ArtifactSourceV1::WasmStore,
        RoleArtifactSourceKindV1::PublishedPackage
        | RoleArtifactSourceKindV1::LocalWasm
        | RoleArtifactSourceKindV1::LocalWasmGz
        | RoleArtifactSourceKindV1::PreviousReceiptArtifact => ArtifactSourceV1::External,
    }
}

fn apply_promotion_source_locator(
    role_artifact: &mut RoleArtifactV1,
    source: &RoleArtifactSourceV1,
) {
    match source.kind {
        RoleArtifactSourceKindV1::LocalWasm => {
            role_artifact.wasm_path.clone_from(&source.locator);
        }
        RoleArtifactSourceKindV1::LocalWasmGz => {
            role_artifact.wasm_gz_path.clone_from(&source.locator);
        }
        _ => {}
    }
}

fn promotion_plan_transform_from_parts(
    target_plan: &DeploymentPlanV1,
    promoted_plan: DeploymentPlanV1,
    inputs: &[RolePromotionInputV1],
) -> PromotionPlanTransformV1 {
    let roles = inputs
        .iter()
        .filter_map(|input| {
            let before = target_plan
                .role_artifacts
                .iter()
                .find(|artifact| artifact.role == input.role)?;
            let after = promoted_plan
                .role_artifacts
                .iter()
                .find(|artifact| artifact.role == input.role)?;
            Some(role_plan_transform(input, before, after))
        })
        .collect::<Vec<_>>();
    let promotion_plan_lineage_digest = promotion_plan_lineage_digest(
        &target_plan.plan_id,
        &promoted_plan.plan_id,
        &promoted_plan,
        &roles,
    );

    PromotionPlanTransformV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        transform_id: format!("promotion-transform:{}", promoted_plan.plan_id),
        target_plan_id: target_plan.plan_id.clone(),
        promoted_plan_id: promoted_plan.plan_id.clone(),
        promotion_plan_lineage_digest,
        promoted_plan,
        roles,
    }
}

fn role_plan_transform(
    input: &RolePromotionInputV1,
    before: &RoleArtifactV1,
    after: &RoleArtifactV1,
) -> RolePromotionPlanTransformV1 {
    RolePromotionPlanTransformV1 {
        role: input.role.clone(),
        promotion_level: input.promotion_level,
        source_kind: input.source.kind,
        source_locator: input.source.locator.clone(),
        artifact_source_before: before.source,
        artifact_source_after: after.source,
        wasm_sha256_before: before.wasm_sha256.clone(),
        wasm_sha256_after: after.wasm_sha256.clone(),
        wasm_gz_sha256_before: before.wasm_gz_sha256.clone(),
        wasm_gz_sha256_after: after.wasm_gz_sha256.clone(),
        candid_sha256_before: before.candid_sha256.clone(),
        candid_sha256_after: after.candid_sha256.clone(),
        canonical_embedded_config_sha256_before: before.canonical_embedded_config_sha256.clone(),
        canonical_embedded_config_sha256_after: after.canonical_embedded_config_sha256.clone(),
        artifact_identity_changed: artifact_identity_changed(before, after),
        embedded_config_changed: before.canonical_embedded_config_sha256
            != after.canonical_embedded_config_sha256,
        target_materialization_preserved: input.promotion_level
            == PromotionArtifactLevelV1::SourceBuild
            && role_materialization_identity_matches(before, after),
        source_build_materialization: None,
    }
}

fn refresh_promotion_plan_lineage_digest(transform: &mut PromotionPlanTransformV1) {
    transform.promotion_plan_lineage_digest = promotion_plan_lineage_digest(
        &transform.target_plan_id,
        &transform.promoted_plan_id,
        &transform.promoted_plan,
        &transform.roles,
    );
}

fn validate_role_plan_transform(
    role: &RolePromotionPlanTransformV1,
    promoted_plan: &DeploymentPlanV1,
) -> Result<(), PromotionPlanTransformError> {
    ensure_transform_field("role", &role.role)?;
    let Some(promoted_role) = promoted_plan
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == role.role)
    else {
        return Err(PromotionPlanTransformError::PromotedRoleMissing {
            role: role.role.clone(),
        });
    };
    ensure_role_matches_promoted_artifact(role, promoted_role)?;
    ensure_role_transform_flags_are_consistent(role)?;
    super::materialization::validate_role_materialization_link(role, promoted_role)?;
    Ok(())
}

fn ensure_role_matches_promoted_artifact(
    role: &RolePromotionPlanTransformV1,
    promoted_role: &RoleArtifactV1,
) -> Result<(), PromotionPlanTransformError> {
    ensure_role_field_matches(
        role,
        "artifact_source_after",
        role.artifact_source_after == promoted_role.source,
    )?;
    ensure_role_field_matches(
        role,
        "wasm_sha256_after",
        role.wasm_sha256_after == promoted_role.wasm_sha256,
    )?;
    ensure_role_field_matches(
        role,
        "wasm_gz_sha256_after",
        role.wasm_gz_sha256_after == promoted_role.wasm_gz_sha256,
    )?;
    ensure_role_field_matches(
        role,
        "candid_sha256_after",
        role.candid_sha256_after == promoted_role.candid_sha256,
    )?;
    ensure_role_field_matches(
        role,
        "canonical_embedded_config_sha256_after",
        role.canonical_embedded_config_sha256_after
            == promoted_role.canonical_embedded_config_sha256,
    )
}

fn ensure_role_transform_flags_are_consistent(
    role: &RolePromotionPlanTransformV1,
) -> Result<(), PromotionPlanTransformError> {
    ensure_role_field_matches(
        role,
        "artifact_identity_changed",
        role.artifact_identity_changed == role_summary_artifact_identity_changed(role),
    )?;
    ensure_role_field_matches(
        role,
        "embedded_config_changed",
        role.embedded_config_changed
            == (role.canonical_embedded_config_sha256_before
                != role.canonical_embedded_config_sha256_after),
    )?;
    if role.target_materialization_preserved {
        ensure_role_field_matches(
            role,
            "target_materialization_preserved",
            role.promotion_level == PromotionArtifactLevelV1::SourceBuild
                && !role.artifact_identity_changed
                && !role.embedded_config_changed,
        )?;
    }
    Ok(())
}

pub(super) fn ensure_role_field_matches(
    role: &RolePromotionPlanTransformV1,
    field: &'static str,
    matches: bool,
) -> Result<(), PromotionPlanTransformError> {
    if matches {
        Ok(())
    } else {
        Err(PromotionPlanTransformError::RoleStateMismatch {
            role: role.role.clone(),
            field,
        })
    }
}

fn validate_role_readiness(role: &RolePromotionReadinessV1) -> Result<(), PromotionReadinessError> {
    ensure_readiness_field("role", &role.role)?;
    ensure_readiness_optional_sha256("source_wasm_sha256", role.source_wasm_sha256.as_deref())?;
    ensure_readiness_optional_sha256(
        "source_wasm_gz_sha256",
        role.source_wasm_gz_sha256.as_deref(),
    )?;
    ensure_readiness_optional_sha256("target_wasm_sha256", role.target_wasm_sha256.as_deref())?;
    ensure_readiness_optional_sha256(
        "target_wasm_gz_sha256",
        role.target_wasm_gz_sha256.as_deref(),
    )?;
    ensure_readiness_optional_sha256(
        "source_canonical_embedded_config_sha256",
        role.source_canonical_embedded_config_sha256.as_deref(),
    )?;
    ensure_readiness_optional_sha256(
        "target_canonical_embedded_config_sha256",
        role.target_canonical_embedded_config_sha256.as_deref(),
    )?;
    if role.restage_required != (role.target_store_has_artifact == Some(false)) {
        return Err(PromotionReadinessError::RestageStateMismatch {
            role: role.role.clone(),
        });
    }
    Ok(())
}

fn role_promotion_readiness(
    input: &RolePromotionInputV1,
    target_artifact: &RoleArtifactV1,
) -> RolePromotionReadinessV1 {
    let source_wasm_sha256 = input.source.expected_wasm_sha256.clone();
    let source_wasm_gz_sha256 = input.source.expected_wasm_gz_sha256.clone();
    let target_wasm_sha256 = target_artifact.wasm_sha256.clone();
    let target_wasm_gz_sha256 = target_artifact.wasm_gz_sha256.clone();
    let byte_identical_wasm =
        matching_optional_digest(source_wasm_sha256.as_ref(), target_wasm_sha256.as_ref()).or_else(
            || {
                matching_optional_digest(
                    source_wasm_gz_sha256.as_ref(),
                    target_wasm_gz_sha256.as_ref(),
                )
            },
        );
    let embedded_config_identical = matching_optional_digest(
        input
            .source
            .expected_canonical_embedded_config_sha256
            .as_ref(),
        target_artifact.canonical_embedded_config_sha256.as_ref(),
    );

    RolePromotionReadinessV1 {
        role: input.role.clone(),
        promotion_level: input.promotion_level,
        source_kind: input.source.kind,
        source_locator: input.source.locator.clone(),
        source_wasm_sha256,
        source_wasm_gz_sha256,
        target_wasm_sha256,
        target_wasm_gz_sha256,
        source_canonical_embedded_config_sha256: input
            .source
            .expected_canonical_embedded_config_sha256
            .clone(),
        target_canonical_embedded_config_sha256: target_artifact
            .canonical_embedded_config_sha256
            .clone(),
        byte_identical_wasm,
        embedded_config_identical,
        target_store_has_artifact: input.target_store_has_artifact,
        restage_required: input.target_store_has_artifact == Some(false),
    }
}

fn collect_role_findings(
    input: &RolePromotionInputV1,
    readiness: &RolePromotionReadinessV1,
    blockers: &mut Vec<SafetyFindingV1>,
    warnings: &mut Vec<SafetyFindingV1>,
) {
    if let Err(err) = validate_role_artifact_source(&input.source) {
        blockers.push(super::promotion_finding(
            "promotion_artifact_source_invalid",
            err.to_string(),
            SafetySeverityV1::HardFailure,
            &input.role,
        ));
    }

    if input.role != input.source.role {
        blockers.push(super::promotion_finding(
            "promotion_source_role_mismatch",
            format!(
                "promotion input role {} does not match artifact source role {}",
                input.role, input.source.role
            ),
            SafetySeverityV1::HardFailure,
            &input.role,
        ));
    }

    if input.require_byte_identical_wasm && readiness.byte_identical_wasm != Some(true) {
        blockers.push(super::promotion_finding(
            "promotion_wasm_digest_mismatch",
            "promotion requires byte-identical wasm but source and target digests differ or are incomplete",
            SafetySeverityV1::HardFailure,
            &input.role,
        ));
    }

    if input.require_target_embedded_config
        && readiness
            .target_canonical_embedded_config_sha256
            .as_deref()
            .is_none_or(str::is_empty)
    {
        blockers.push(super::promotion_finding(
            "promotion_target_embedded_config_missing",
            "promotion requires target canonical embedded config but target plan has no digest",
            SafetySeverityV1::HardFailure,
            &input.role,
        ));
    }

    if input.promotion_level == PromotionArtifactLevelV1::SealedWasm
        && readiness.embedded_config_identical != Some(true)
    {
        blockers.push(super::promotion_finding(
            "promotion_sealed_wasm_embedded_config_mismatch",
            "sealed wasm promotion requires embedded config identity to be acceptable for the target",
            SafetySeverityV1::HardFailure,
            &input.role,
        ));
    }

    if readiness.restage_required {
        warnings.push(super::promotion_finding(
            "promotion_target_store_restage_required",
            "target artifact store does not already contain the artifact; restaging is required",
            SafetySeverityV1::Warning,
            &input.role,
        ));
    }
}

fn matching_optional_digest(left: Option<&String>, right: Option<&String>) -> Option<bool> {
    match (left.map(String::as_str), right.map(String::as_str)) {
        (Some(left), Some(right)) => Some(left == right),
        _ => None,
    }
}

const fn ensure_readiness_status_matches_blockers(
    readiness: &PromotionReadinessV1,
) -> Result<(), PromotionReadinessError> {
    match (readiness.status, readiness.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => {
            Err(PromotionReadinessError::StatusBlockerMismatch {
                status: readiness.status,
                blocker_count: readiness.blockers.len(),
            })
        }
        _ => Ok(()),
    }
}

fn ensure_unique_readiness_roles(
    roles: &[RolePromotionReadinessV1],
) -> Result<(), PromotionReadinessError> {
    let mut seen = std::collections::BTreeSet::new();
    for role in roles {
        if !seen.insert(role.role.as_str()) {
            return Err(PromotionReadinessError::DuplicateRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

fn ensure_unique_transform_roles(
    roles: &[RolePromotionPlanTransformV1],
) -> Result<(), PromotionPlanTransformError> {
    let mut seen = std::collections::BTreeSet::new();
    for role in roles {
        if !seen.insert(role.role.as_str()) {
            return Err(PromotionPlanTransformError::DuplicateRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

fn validate_readiness_findings(
    field: &'static str,
    findings: &[SafetyFindingV1],
    expected_severity: SafetySeverityV1,
) -> Result<(), PromotionReadinessError> {
    for finding in findings {
        ensure_readiness_field("finding.code", &finding.code)?;
        ensure_readiness_field("finding.message", &finding.message)?;
        if finding.severity != expected_severity {
            return Err(PromotionReadinessError::FindingSeverityMismatch {
                field,
                severity: finding.severity,
            });
        }
    }
    Ok(())
}
