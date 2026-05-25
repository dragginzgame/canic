use super::{
    DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentPlanV1, PromotionArtifactLevelV1,
    PromotionReadinessStatusV1, PromotionReadinessV1, RoleArtifactSourceKindV1,
    RoleArtifactSourceV1, RoleArtifactV1, RolePromotionInputV1, RolePromotionReadinessV1,
    SafetyFindingV1, SafetySeverityV1,
};
use thiserror::Error as ThisError;

///
/// PromotionArtifactSourceError
///
#[derive(Debug, ThisError)]
pub enum PromotionArtifactSourceError {
    #[error("promotion artifact source is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error("promotion artifact source field {field} must be a lowercase sha256 hex digest")]
    InvalidSha256Digest { field: &'static str },
    #[error("promotion artifact source kind {kind:?} requires a digest pin")]
    MissingDigestPin { kind: RoleArtifactSourceKindV1 },
    #[error("promotion artifact source kind {kind:?} cannot carry previous receipt kind")]
    UnexpectedPreviousReceiptKind { kind: RoleArtifactSourceKindV1 },
    #[error(
        "promotion artifact source kind PreviousReceiptArtifact requires an eligible receipt kind"
    )]
    MissingPreviousReceiptKind,
}

///
/// PromotionReadinessError
///
#[derive(Debug, ThisError)]
pub enum PromotionReadinessError {
    #[error("promotion readiness schema mismatch: expected {expected}, found {found}")]
    SchemaVersionMismatch { expected: u32, found: u32 },
    #[error("promotion readiness is missing required field: {field}")]
    MissingRequiredField { field: &'static str },
    #[error("promotion readiness status {status:?} does not match blocker count {blocker_count}")]
    StatusBlockerMismatch {
        status: PromotionReadinessStatusV1,
        blocker_count: usize,
    },
    #[error("promotion readiness contains duplicate role: {role}")]
    DuplicateRole { role: String },
    #[error("promotion readiness role {role} has inconsistent restage state")]
    RestageStateMismatch { role: String },
    #[error("promotion readiness finding in {field} has severity {severity:?}")]
    FindingSeverityMismatch {
        field: &'static str,
        severity: SafetySeverityV1,
    },
    #[error("promotion readiness field {field} must be a lowercase sha256 hex digest")]
    InvalidSha256Digest { field: &'static str },
}

///
/// PromotionReadinessRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionReadinessRequest {
    pub readiness_id: String,
    pub target_plan: DeploymentPlanV1,
    pub inputs: Vec<RolePromotionInputV1>,
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
            blockers.push(promotion_finding(
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

    PromotionReadinessV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        readiness_id: readiness_id.into(),
        target_plan_id: target_plan.plan_id.clone(),
        status,
        roles,
        blockers,
        warnings,
    }
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
    Ok(())
}

pub fn validate_role_artifact_source(
    source: &RoleArtifactSourceV1,
) -> Result<(), PromotionArtifactSourceError> {
    ensure_field("role", &source.role)?;
    ensure_locator_requirement(source)?;
    ensure_previous_receipt_requirement(source)?;
    ensure_digest_requirement(source)?;
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
    Ok(())
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
        blockers.push(promotion_finding(
            "promotion_artifact_source_invalid",
            err.to_string(),
            SafetySeverityV1::HardFailure,
            &input.role,
        ));
    }

    if input.role != input.source.role {
        blockers.push(promotion_finding(
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
        blockers.push(promotion_finding(
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
        blockers.push(promotion_finding(
            "promotion_target_embedded_config_missing",
            "promotion requires target canonical embedded config but target plan has no digest",
            SafetySeverityV1::HardFailure,
            &input.role,
        ));
    }

    if input.promotion_level == PromotionArtifactLevelV1::SealedWasm
        && readiness.embedded_config_identical != Some(true)
    {
        blockers.push(promotion_finding(
            "promotion_sealed_wasm_embedded_config_mismatch",
            "sealed wasm promotion requires embedded config identity to be acceptable for the target",
            SafetySeverityV1::HardFailure,
            &input.role,
        ));
    }

    if readiness.restage_required {
        warnings.push(promotion_finding(
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

fn promotion_finding(
    code: impl Into<String>,
    message: impl Into<String>,
    severity: SafetySeverityV1,
    role: &str,
) -> SafetyFindingV1 {
    SafetyFindingV1 {
        code: code.into(),
        message: message.into(),
        severity,
        subject: Some(role.to_string()),
    }
}

fn ensure_locator_requirement(
    source: &RoleArtifactSourceV1,
) -> Result<(), PromotionArtifactSourceError> {
    match source.kind {
        RoleArtifactSourceKindV1::CanonicalWasmStoreDefault => Ok(()),
        _ => ensure_option_field("locator", source.locator.as_deref()),
    }
}

const fn ensure_previous_receipt_requirement(
    source: &RoleArtifactSourceV1,
) -> Result<(), PromotionArtifactSourceError> {
    match (source.kind, source.previous_receipt_kind) {
        (RoleArtifactSourceKindV1::PreviousReceiptArtifact, Some(_)) => Ok(()),
        (RoleArtifactSourceKindV1::PreviousReceiptArtifact, None) => {
            Err(PromotionArtifactSourceError::MissingPreviousReceiptKind)
        }
        (_, Some(_)) => {
            Err(PromotionArtifactSourceError::UnexpectedPreviousReceiptKind { kind: source.kind })
        }
        (_, None) => Ok(()),
    }
}

const fn ensure_digest_requirement(
    source: &RoleArtifactSourceV1,
) -> Result<(), PromotionArtifactSourceError> {
    let has_digest =
        source.expected_wasm_sha256.is_some() || source.expected_wasm_gz_sha256.is_some();
    match source.kind {
        RoleArtifactSourceKindV1::LocalWasm if source.expected_wasm_sha256.is_none() => {
            Err(PromotionArtifactSourceError::MissingDigestPin { kind: source.kind })
        }
        RoleArtifactSourceKindV1::LocalWasmGz if source.expected_wasm_gz_sha256.is_none() => {
            Err(PromotionArtifactSourceError::MissingDigestPin { kind: source.kind })
        }
        RoleArtifactSourceKindV1::PublishedPackage
        | RoleArtifactSourceKindV1::PreviousReceiptArtifact
            if !has_digest =>
        {
            Err(PromotionArtifactSourceError::MissingDigestPin { kind: source.kind })
        }
        _ => Ok(()),
    }
}

fn ensure_option_field(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), PromotionArtifactSourceError> {
    match value {
        Some(value) => ensure_field(field, value),
        None => Err(PromotionArtifactSourceError::MissingRequiredField { field }),
    }
}

fn ensure_field(field: &'static str, value: &str) -> Result<(), PromotionArtifactSourceError> {
    if value.trim().is_empty() {
        return Err(PromotionArtifactSourceError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_optional_sha256(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), PromotionArtifactSourceError> {
    let Some(value) = value else {
        return Ok(());
    };
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionArtifactSourceError::InvalidSha256Digest { field })
    }
}

fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
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

fn ensure_readiness_field(field: &'static str, value: &str) -> Result<(), PromotionReadinessError> {
    if value.trim().is_empty() {
        return Err(PromotionReadinessError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_readiness_optional_sha256(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), PromotionReadinessError> {
    let Some(value) = value else {
        return Ok(());
    };
    if is_lower_hex_sha256(value) {
        Ok(())
    } else {
        Err(PromotionReadinessError::InvalidSha256Digest { field })
    }
}
