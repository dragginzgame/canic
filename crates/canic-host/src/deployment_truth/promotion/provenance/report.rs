use super::super::{
    digest::artifact_promotion_provenance_digest,
    ensure::{ensure_provenance_report_field, ensure_provenance_report_sha256},
    error::ArtifactPromotionProvenanceReportError,
    request::ArtifactPromotionProvenanceReportRequest,
};
use crate::deployment_truth::{
    ArtifactPromotionPlanV1, ArtifactPromotionProvenanceReportV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION,
    PromotionMaterializationIdentityReportV1, PromotionReadinessStatusV1,
    PromotionWasmStoreCatalogVerificationV1, PromotionWasmStoreIdentityReportV1,
    RolePromotionPlanTransformV1, RolePromotionProvenanceV1, SafetyFindingV1, SafetySeverityV1,
};
use std::collections::BTreeSet;

pub fn artifact_promotion_provenance_report(
    request: ArtifactPromotionProvenanceReportRequest,
) -> Result<ArtifactPromotionProvenanceReportV1, ArtifactPromotionProvenanceReportError> {
    ensure_provenance_report_field("report_id", &request.report_id)?;
    super::super::validate_artifact_promotion_plan(&request.artifact_promotion_plan)?;
    if let Some(report) = &request.wasm_store_identity_report {
        super::super::validate_promotion_wasm_store_identity_report(report)?;
    }
    if let Some(verification) = &request.wasm_store_catalog_verification {
        super::super::validate_promotion_wasm_store_catalog_verification(verification)?;
    }
    if let Some(report) = &request.materialization_identity_report {
        super::super::validate_promotion_materialization_identity_report(report)?;
    }
    let report = build_artifact_promotion_provenance_report(request);
    validate_artifact_promotion_provenance_report(&report)?;
    Ok(report)
}

pub fn validate_artifact_promotion_provenance_report(
    report: &ArtifactPromotionProvenanceReportV1,
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            ArtifactPromotionProvenanceReportError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: report.schema_version,
            },
        );
    }
    ensure_provenance_report_field("report_id", &report.report_id)?;
    ensure_provenance_report_field(
        "artifact_promotion_plan_id",
        &report.artifact_promotion_plan_id,
    )?;
    ensure_provenance_report_sha256(
        "artifact_promotion_plan_digest",
        &report.artifact_promotion_plan_digest,
    )?;
    ensure_provenance_report_field("target_plan_id", &report.target_plan_id)?;
    ensure_provenance_report_field("promoted_plan_id", &report.promoted_plan_id)?;
    ensure_provenance_report_field(
        "promotion_plan_lineage_digest",
        &report.promotion_plan_lineage_digest,
    )?;
    ensure_provenance_report_sha256("provenance_report_digest", &report.provenance_report_digest)?;
    ensure_provenance_report_field("readiness_id", &report.readiness_id)?;
    ensure_provenance_report_field(
        "artifact_identity_report_id",
        &report.artifact_identity_report_id,
    )?;
    ensure_provenance_report_field("transform_id", &report.transform_id)?;
    if let Some(lineage_id) = &report.target_execution_lineage_id {
        ensure_provenance_report_field("target_execution_lineage_id", lineage_id)?;
    }
    if let Some(report_id) = &report.wasm_store_identity_report_id {
        ensure_provenance_report_field("wasm_store_identity_report_id", report_id)?;
    }
    if let Some(digest) = &report.wasm_store_identity_report_digest {
        ensure_provenance_report_sha256("wasm_store_identity_report_digest", digest)?;
        if report.wasm_store_identity_report_id.is_none() {
            return Err(ArtifactPromotionProvenanceReportError::LinkageMismatch {
                field: "wasm_store_identity_report_digest",
            });
        }
    }
    if let Some(verification_id) = &report.wasm_store_catalog_verification_id {
        ensure_provenance_report_field("wasm_store_catalog_verification_id", verification_id)?;
        if report.wasm_store_identity_report_id.is_none() {
            return Err(ArtifactPromotionProvenanceReportError::LinkageMismatch {
                field: "wasm_store_catalog_verification_id",
            });
        }
    }
    if let Some(digest) = &report.wasm_store_catalog_verification_digest {
        ensure_provenance_report_sha256("wasm_store_catalog_verification_digest", digest)?;
        if report.wasm_store_catalog_verification_id.is_none() {
            return Err(ArtifactPromotionProvenanceReportError::LinkageMismatch {
                field: "wasm_store_catalog_verification_digest",
            });
        }
    }
    if let Some(report_id) = &report.materialization_identity_report_id {
        ensure_provenance_report_field("materialization_identity_report_id", report_id)?;
    }
    if let Some(digest) = &report.materialization_identity_report_digest {
        ensure_provenance_report_sha256("materialization_identity_report_digest", digest)?;
        if report.materialization_identity_report_id.is_none() {
            return Err(ArtifactPromotionProvenanceReportError::LinkageMismatch {
                field: "materialization_identity_report_digest",
            });
        }
    }
    ensure_provenance_report_status_matches_blockers(report)?;
    ensure_unique_provenance_roles(&report.roles)?;
    for role in &report.roles {
        validate_role_promotion_provenance(role)?;
    }
    validate_provenance_report_blockers(&report.blockers)?;
    if report.provenance_report_digest != artifact_promotion_provenance_digest(report) {
        return Err(ArtifactPromotionProvenanceReportError::LinkageMismatch {
            field: "provenance_report_digest",
        });
    }
    Ok(())
}

fn build_artifact_promotion_provenance_report(
    request: ArtifactPromotionProvenanceReportRequest,
) -> ArtifactPromotionProvenanceReportV1 {
    let plan = request.artifact_promotion_plan;
    let mut roles = plan
        .transform
        .roles
        .iter()
        .map(role_promotion_provenance_from_transform)
        .collect::<Vec<_>>();
    attach_wasm_store_provenance(&mut roles, request.wasm_store_identity_report.as_ref());
    attach_wasm_store_catalog_provenance(
        &mut roles,
        request.wasm_store_catalog_verification.as_ref(),
    );
    attach_materialization_provenance(&mut roles, request.materialization_identity_report.as_ref());
    let blockers = artifact_promotion_provenance_blockers(
        &plan,
        request.wasm_store_identity_report.as_ref(),
        request.wasm_store_catalog_verification.as_ref(),
        request.materialization_identity_report.as_ref(),
        &roles,
    );
    let status = if blockers.is_empty() {
        PromotionReadinessStatusV1::Ready
    } else {
        PromotionReadinessStatusV1::Blocked
    };
    let mut report = ArtifactPromotionProvenanceReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: request.report_id,
        status,
        artifact_promotion_plan_id: plan.plan_id,
        artifact_promotion_plan_digest: plan.artifact_promotion_plan_digest,
        target_plan_id: plan.target_plan_id,
        promoted_plan_id: plan.promoted_plan_id,
        promotion_plan_lineage_digest: plan.promotion_plan_lineage_digest,
        provenance_report_digest: String::new(),
        readiness_id: plan.readiness.readiness_id,
        artifact_identity_report_id: plan.artifact_identity_report.report_id,
        transform_id: plan.transform.transform_id,
        target_execution_lineage_id: plan
            .target_execution_lineage
            .map(|lineage| lineage.lineage_id),
        wasm_store_identity_report_id: request
            .wasm_store_identity_report
            .as_ref()
            .map(|report| report.report_id.clone()),
        wasm_store_identity_report_digest: request
            .wasm_store_identity_report
            .map(|report| report.wasm_store_identity_report_digest),
        wasm_store_catalog_verification_id: request
            .wasm_store_catalog_verification
            .as_ref()
            .map(|verification| verification.verification_id.clone()),
        wasm_store_catalog_verification_digest: request
            .wasm_store_catalog_verification
            .map(|verification| verification.wasm_store_catalog_verification_digest),
        materialization_identity_report_id: request
            .materialization_identity_report
            .as_ref()
            .map(|report| report.report_id.clone()),
        materialization_identity_report_digest: request
            .materialization_identity_report
            .map(|report| report.materialization_identity_report_digest),
        execution_attempted: false,
        roles,
        blockers,
    };
    report.provenance_report_digest = artifact_promotion_provenance_digest(&report);
    report
}

fn artifact_promotion_provenance_blockers(
    plan: &ArtifactPromotionPlanV1,
    wasm_store_report: Option<&PromotionWasmStoreIdentityReportV1>,
    wasm_store_catalog: Option<&PromotionWasmStoreCatalogVerificationV1>,
    materialization_report: Option<&PromotionMaterializationIdentityReportV1>,
    roles: &[RolePromotionProvenanceV1],
) -> Vec<SafetyFindingV1> {
    let mut blockers = plan.blockers.clone();
    let role_names = roles
        .iter()
        .map(|role| role.role.as_str())
        .collect::<BTreeSet<_>>();
    if let Some(report) = wasm_store_report {
        blockers.extend(report.blockers.iter().cloned());
    }
    append_wasm_store_catalog_provenance_blockers(
        &mut blockers,
        wasm_store_report,
        wasm_store_catalog,
        &role_names,
    );
    if let Some(report) = materialization_report {
        blockers.extend(report.blockers.iter().cloned());
    }
    append_optional_report_unknown_role_blockers(
        &mut blockers,
        wasm_store_report,
        wasm_store_catalog,
        materialization_report,
        &role_names,
    );
    blockers
}

fn append_wasm_store_catalog_provenance_blockers(
    blockers: &mut Vec<SafetyFindingV1>,
    wasm_store_report: Option<&PromotionWasmStoreIdentityReportV1>,
    wasm_store_catalog: Option<&PromotionWasmStoreCatalogVerificationV1>,
    role_names: &BTreeSet<&str>,
) {
    let Some(verification) = wasm_store_catalog else {
        return;
    };
    blockers.extend(verification.blockers.iter().cloned());
    match wasm_store_report {
        Some(report) if verification.wasm_store_identity_report_id == report.report_id => {}
        Some(report) => blockers.push(super::super::promotion_finding(
            "promotion_provenance_wasm_store_catalog_identity_mismatch",
            format!(
                "wasm-store catalog verification references identity report {}, but provenance uses {}",
                verification.wasm_store_identity_report_id, report.report_id
            ),
            SafetySeverityV1::HardFailure,
            "wasm_store_catalog",
        )),
        None => blockers.push(super::super::promotion_finding(
            "promotion_provenance_wasm_store_catalog_identity_missing",
            "wasm-store catalog verification requires the referenced wasm-store identity report",
            SafetySeverityV1::HardFailure,
            "wasm_store_catalog",
        )),
    }
    if let Some(report) = wasm_store_report {
        append_wasm_store_catalog_locator_blockers(blockers, report, verification, role_names);
    }
}

fn append_wasm_store_catalog_locator_blockers(
    blockers: &mut Vec<SafetyFindingV1>,
    report: &PromotionWasmStoreIdentityReportV1,
    verification: &PromotionWasmStoreCatalogVerificationV1,
    role_names: &BTreeSet<&str>,
) {
    for catalog_role in &verification.roles {
        if !role_names.contains(catalog_role.role.as_str()) {
            continue;
        }
        match report.roles.iter().find(|role| role.role == catalog_role.role) {
            Some(identity_role)
                if identity_role.wasm_store_locator.as_deref()
                    == Some(catalog_role.wasm_store_locator.as_str()) => {}
            Some(identity_role) => blockers.push(super::super::promotion_finding(
                "promotion_provenance_wasm_store_catalog_locator_mismatch",
                format!(
                    "wasm-store catalog verification role {} uses locator {}, but identity report uses {}",
                    catalog_role.role,
                    catalog_role.wasm_store_locator,
                    identity_role.wasm_store_locator.as_deref().unwrap_or("none")
                ),
                SafetySeverityV1::HardFailure,
                &catalog_role.role,
            )),
            None => blockers.push(super::super::promotion_finding(
                "promotion_provenance_wasm_store_catalog_role_identity_missing",
                format!(
                    "wasm-store catalog verification role {} is missing from the wasm-store identity report",
                    catalog_role.role
                ),
                SafetySeverityV1::HardFailure,
                &catalog_role.role,
            )),
        }
    }
}

fn append_optional_report_unknown_role_blockers(
    blockers: &mut Vec<SafetyFindingV1>,
    wasm_store_report: Option<&PromotionWasmStoreIdentityReportV1>,
    wasm_store_catalog: Option<&PromotionWasmStoreCatalogVerificationV1>,
    materialization_report: Option<&PromotionMaterializationIdentityReportV1>,
    role_names: &BTreeSet<&str>,
) {
    if let Some(report) = wasm_store_report {
        for role in &report.roles {
            if !role_names.contains(role.role.as_str()) {
                blockers.push(super::super::promotion_finding(
                    "promotion_provenance_unknown_wasm_store_role",
                    format!(
                        "wasm-store identity report contains unknown role {}",
                        role.role
                    ),
                    SafetySeverityV1::HardFailure,
                    &role.role,
                ));
            }
        }
    }
    if let Some(verification) = wasm_store_catalog {
        for role in &verification.roles {
            if !role_names.contains(role.role.as_str()) {
                blockers.push(super::super::promotion_finding(
                    "promotion_provenance_unknown_wasm_store_catalog_role",
                    format!(
                        "wasm-store catalog verification contains unknown role {}",
                        role.role
                    ),
                    SafetySeverityV1::HardFailure,
                    &role.role,
                ));
            }
        }
    }
    if let Some(report) = materialization_report {
        for role in &report.roles {
            if !role_names.contains(role.role.as_str()) {
                blockers.push(super::super::promotion_finding(
                    "promotion_provenance_unknown_materialization_role",
                    format!(
                        "materialization identity report contains unknown role {}",
                        role.role
                    ),
                    SafetySeverityV1::HardFailure,
                    &role.role,
                ));
            }
        }
    }
}

fn role_promotion_provenance_from_transform(
    role: &RolePromotionPlanTransformV1,
) -> RolePromotionProvenanceV1 {
    RolePromotionProvenanceV1 {
        role: role.role.clone(),
        promotion_level: role.promotion_level,
        source_kind: role.source_kind,
        artifact_identity_changed: role.artifact_identity_changed,
        embedded_config_changed: role.embedded_config_changed,
        target_materialization_preserved: role.target_materialization_preserved,
        materialization_evidence_id: role
            .source_build_materialization
            .as_ref()
            .map(|materialization| materialization.evidence_id.clone()),
        materialization_evidence_digest: role
            .source_build_materialization
            .as_ref()
            .map(|materialization| materialization.materialization_evidence_digest.clone()),
        wasm_store_locator: None,
        wasm_store_catalog_observation_digest: None,
    }
}

fn attach_wasm_store_provenance(
    roles: &mut [RolePromotionProvenanceV1],
    report: Option<&PromotionWasmStoreIdentityReportV1>,
) {
    let Some(report) = report else {
        return;
    };
    for role in roles {
        if let Some(wasm_store_role) = report.roles.iter().find(|item| item.role == role.role) {
            role.wasm_store_locator = wasm_store_role.wasm_store_locator.clone();
        }
    }
}

fn attach_wasm_store_catalog_provenance(
    roles: &mut [RolePromotionProvenanceV1],
    verification: Option<&PromotionWasmStoreCatalogVerificationV1>,
) {
    let Some(verification) = verification else {
        return;
    };
    for role in roles {
        if let Some(catalog_role) = verification
            .roles
            .iter()
            .find(|item| item.role == role.role)
        {
            role.wasm_store_catalog_observation_digest =
                Some(catalog_role.catalog_observation_digest.clone());
        }
    }
}

fn attach_materialization_provenance(
    roles: &mut [RolePromotionProvenanceV1],
    report: Option<&PromotionMaterializationIdentityReportV1>,
) {
    let Some(report) = report else {
        return;
    };
    for role in roles {
        if let Some(materialization_role) = report.roles.iter().find(|item| item.role == role.role)
        {
            role.materialization_evidence_id = Some(materialization_role.evidence_id.clone());
            role.materialization_evidence_digest =
                Some(materialization_role.materialization_evidence_digest.clone());
        }
    }
}

const fn ensure_provenance_report_status_matches_blockers(
    report: &ArtifactPromotionProvenanceReportV1,
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    match (report.status, report.blockers.is_empty()) {
        (PromotionReadinessStatusV1::Ready, false)
        | (PromotionReadinessStatusV1::Blocked, true) => Err(
            ArtifactPromotionProvenanceReportError::StatusBlockerMismatch {
                status: report.status,
                blocker_count: report.blockers.len(),
            },
        ),
        _ => Ok(()),
    }
}

fn ensure_unique_provenance_roles(
    roles: &[RolePromotionProvenanceV1],
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    let mut seen = BTreeSet::new();
    for role in roles {
        if !seen.insert(role.role.as_str()) {
            return Err(ArtifactPromotionProvenanceReportError::DuplicateRole {
                role: role.role.clone(),
            });
        }
    }
    Ok(())
}

fn validate_role_promotion_provenance(
    role: &RolePromotionProvenanceV1,
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    ensure_provenance_report_field("role", &role.role)?;
    if let Some(evidence_id) = &role.materialization_evidence_id {
        ensure_provenance_report_field("materialization_evidence_id", evidence_id)?;
    }
    if let Some(digest) = &role.materialization_evidence_digest {
        ensure_provenance_report_sha256("materialization_evidence_digest", digest)?;
    }
    if let Some(locator) = &role.wasm_store_locator {
        ensure_provenance_report_field("wasm_store_locator", locator)?;
    }
    if let Some(digest) = &role.wasm_store_catalog_observation_digest {
        ensure_provenance_report_sha256("wasm_store_catalog_observation_digest", digest)?;
    }
    Ok(())
}

fn validate_provenance_report_blockers(
    blockers: &[SafetyFindingV1],
) -> Result<(), ArtifactPromotionProvenanceReportError> {
    for blocker in blockers {
        ensure_provenance_report_field("blocker.code", &blocker.code)?;
        ensure_provenance_report_field("blocker.message", &blocker.message)?;
        if blocker.severity != SafetySeverityV1::HardFailure {
            return Err(
                ArtifactPromotionProvenanceReportError::BlockerSeverityMismatch {
                    severity: blocker.severity,
                },
            );
        }
    }
    Ok(())
}
