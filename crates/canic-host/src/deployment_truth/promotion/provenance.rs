use super::super::{
    ArtifactPromotionExecutionReceiptV1, ArtifactPromotionPlanV1,
    ArtifactPromotionProvenanceReportV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentReceiptV1,
    PromotionMaterializationIdentityReportV1, PromotionReadinessStatusV1,
    PromotionWasmStoreCatalogVerificationV1, PromotionWasmStoreIdentityReportV1,
    RolePromotionExecutionReceiptV1, RolePromotionPlanTransformV1, RolePromotionProvenanceV1,
    SafetyFindingV1, SafetySeverityV1,
};
use super::digest::{
    artifact_promotion_execution_receipt_digest, artifact_promotion_provenance_digest,
};
use super::ensure::{
    ensure_execution_receipt_field, ensure_execution_receipt_sha256,
    ensure_provenance_report_field, ensure_provenance_report_sha256,
};
use super::error::{
    ArtifactPromotionExecutionReceiptError, ArtifactPromotionProvenanceReportError,
};
use super::request::{
    ArtifactPromotionExecutionReceiptRequest, ArtifactPromotionProvenanceReportRequest,
};
use std::collections::BTreeSet;

pub fn artifact_promotion_provenance_report(
    request: ArtifactPromotionProvenanceReportRequest,
) -> Result<ArtifactPromotionProvenanceReportV1, ArtifactPromotionProvenanceReportError> {
    ensure_provenance_report_field("report_id", &request.report_id)?;
    super::validate_artifact_promotion_plan(&request.artifact_promotion_plan)?;
    if let Some(report) = &request.wasm_store_identity_report {
        super::validate_promotion_wasm_store_identity_report(report)?;
    }
    if let Some(verification) = &request.wasm_store_catalog_verification {
        super::validate_promotion_wasm_store_catalog_verification(verification)?;
    }
    if let Some(report) = &request.materialization_identity_report {
        super::validate_promotion_materialization_identity_report(report)?;
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

pub fn artifact_promotion_execution_receipt(
    request: ArtifactPromotionExecutionReceiptRequest,
) -> Result<ArtifactPromotionExecutionReceiptV1, ArtifactPromotionExecutionReceiptError> {
    ensure_execution_receipt_field("receipt_id", &request.receipt_id)?;
    validate_artifact_promotion_provenance_report(&request.provenance_report)?;
    ensure_execution_receipt_provenance_ready(request.provenance_report.status)?;
    validate_deployment_receipt_for_promotion(
        &request.deployment_receipt,
        &request.provenance_report,
    )?;
    let receipt = build_artifact_promotion_execution_receipt(request);
    validate_artifact_promotion_execution_receipt(&receipt)?;
    Ok(receipt)
}

pub fn validate_artifact_promotion_execution_receipt(
    receipt: &ArtifactPromotionExecutionReceiptV1,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    if receipt.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            ArtifactPromotionExecutionReceiptError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: receipt.schema_version,
            },
        );
    }
    ensure_execution_receipt_field("receipt_id", &receipt.receipt_id)?;
    ensure_execution_receipt_sha256(
        "execution_receipt_digest",
        &receipt.execution_receipt_digest,
    )?;
    ensure_execution_receipt_field(
        "artifact_promotion_plan_id",
        &receipt.artifact_promotion_plan_id,
    )?;
    ensure_execution_receipt_sha256(
        "artifact_promotion_plan_digest",
        &receipt.artifact_promotion_plan_digest,
    )?;
    ensure_execution_receipt_field("provenance_report_id", &receipt.provenance_report_id)?;
    ensure_execution_receipt_sha256(
        "provenance_report_digest",
        &receipt.provenance_report_digest,
    )?;
    ensure_execution_receipt_provenance_ready(receipt.provenance_status)?;
    ensure_execution_receipt_field("promoted_plan_id", &receipt.promoted_plan_id)?;
    ensure_execution_receipt_field(
        "promotion_plan_lineage_digest",
        &receipt.promotion_plan_lineage_digest,
    )?;
    ensure_execution_receipt_field("operation_id", &receipt.operation_id)?;
    ensure_execution_receipt_field("started_at", &receipt.started_at)?;
    if let Some(finished_at) = &receipt.finished_at {
        ensure_execution_receipt_field("finished_at", finished_at)?;
    }
    ensure_execution_receipt_linkage(receipt)?;
    if receipt.execution_receipt_digest != artifact_promotion_execution_receipt_digest(receipt) {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "execution_receipt_digest",
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
        Some(report) => blockers.push(super::promotion_finding(
            "promotion_provenance_wasm_store_catalog_identity_mismatch",
            format!(
                "wasm-store catalog verification references identity report {}, but provenance uses {}",
                verification.wasm_store_identity_report_id, report.report_id
            ),
            SafetySeverityV1::HardFailure,
            "wasm_store_catalog",
        )),
        None => blockers.push(super::promotion_finding(
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
            Some(identity_role) => blockers.push(super::promotion_finding(
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
            None => blockers.push(super::promotion_finding(
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
                blockers.push(super::promotion_finding(
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
                blockers.push(super::promotion_finding(
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
                blockers.push(super::promotion_finding(
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

fn build_artifact_promotion_execution_receipt(
    request: ArtifactPromotionExecutionReceiptRequest,
) -> ArtifactPromotionExecutionReceiptV1 {
    let roles = request
        .provenance_report
        .roles
        .iter()
        .map(|role| role_promotion_execution_receipt(role, &request.deployment_receipt))
        .collect::<Vec<_>>();
    let mut receipt = ArtifactPromotionExecutionReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        receipt_id: request.receipt_id,
        execution_receipt_digest: String::new(),
        artifact_promotion_plan_id: request.provenance_report.artifact_promotion_plan_id.clone(),
        artifact_promotion_plan_digest: request
            .provenance_report
            .artifact_promotion_plan_digest
            .clone(),
        provenance_report_id: request.provenance_report.report_id.clone(),
        provenance_report_digest: request.provenance_report.provenance_report_digest,
        provenance_status: request.provenance_report.status,
        promoted_plan_id: request.provenance_report.promoted_plan_id.clone(),
        promotion_plan_lineage_digest: request.provenance_report.promotion_plan_lineage_digest,
        operation_id: request.deployment_receipt.operation_id.clone(),
        operation_status: request.deployment_receipt.operation_status,
        command_result: request.deployment_receipt.command_result.clone(),
        started_at: request.deployment_receipt.started_at.clone(),
        finished_at: request.deployment_receipt.finished_at.clone(),
        deployment_receipt: request.deployment_receipt,
        roles,
    };
    receipt.execution_receipt_digest = artifact_promotion_execution_receipt_digest(&receipt);
    receipt
}

fn role_promotion_execution_receipt(
    role: &RolePromotionProvenanceV1,
    deployment_receipt: &DeploymentReceiptV1,
) -> RolePromotionExecutionReceiptV1 {
    let role_receipt = deployment_receipt
        .role_phase_receipts
        .iter()
        .rev()
        .find(|receipt| receipt.role == role.role);
    RolePromotionExecutionReceiptV1 {
        role: role.role.clone(),
        promotion_level: role.promotion_level,
        materialization_evidence_id: role.materialization_evidence_id.clone(),
        materialization_evidence_digest: role.materialization_evidence_digest.clone(),
        wasm_store_locator: role.wasm_store_locator.clone(),
        wasm_store_catalog_observation_digest: role.wasm_store_catalog_observation_digest.clone(),
        role_phase_result: role_receipt.map(|receipt| receipt.result),
        artifact_digest: role_receipt.and_then(|receipt| receipt.artifact_digest.clone()),
        observed_module_hash_after: role_receipt
            .and_then(|receipt| receipt.observed_module_hash_after.clone()),
        canonical_embedded_config_sha256: role_receipt
            .and_then(|receipt| receipt.canonical_embedded_config_sha256.clone()),
    }
}

fn validate_deployment_receipt_for_promotion(
    receipt: &DeploymentReceiptV1,
    provenance: &ArtifactPromotionProvenanceReportV1,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    if receipt.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            ArtifactPromotionExecutionReceiptError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: receipt.schema_version,
            },
        );
    }
    ensure_execution_receipt_field("deployment_receipt.operation_id", &receipt.operation_id)?;
    ensure_execution_receipt_field("deployment_receipt.started_at", &receipt.started_at)?;
    if receipt.plan_id != provenance.promoted_plan_id {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "deployment_receipt.plan_id",
        });
    }
    if let Some(finished_at) = &receipt.finished_at {
        ensure_execution_receipt_field("deployment_receipt.finished_at", finished_at)?;
    }
    Ok(())
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

fn ensure_execution_receipt_linkage(
    receipt: &ArtifactPromotionExecutionReceiptV1,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    if receipt.deployment_receipt.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            ArtifactPromotionExecutionReceiptError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                found: receipt.deployment_receipt.schema_version,
            },
        );
    }
    if receipt.deployment_receipt.plan_id != receipt.promoted_plan_id {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "deployment_receipt.plan_id",
        });
    }
    if receipt.deployment_receipt.operation_id != receipt.operation_id {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "operation_id",
        });
    }
    if receipt.deployment_receipt.operation_status != receipt.operation_status {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "operation_status",
        });
    }
    if receipt.deployment_receipt.command_result != receipt.command_result {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "command_result",
        });
    }
    if receipt.deployment_receipt.started_at != receipt.started_at {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "started_at",
        });
    }
    if receipt.deployment_receipt.finished_at != receipt.finished_at {
        return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
            field: "finished_at",
        });
    }
    ensure_execution_receipt_roles_match_deployment_receipt(
        &receipt.roles,
        &receipt.deployment_receipt,
    )?;
    ensure_unique_execution_receipt_roles(&receipt.roles)
}

const fn ensure_execution_receipt_provenance_ready(
    status: PromotionReadinessStatusV1,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    if matches!(status, PromotionReadinessStatusV1::Ready) {
        Ok(())
    } else {
        Err(ArtifactPromotionExecutionReceiptError::ProvenanceNotReady { status })
    }
}

fn ensure_execution_receipt_roles_match_deployment_receipt(
    roles: &[RolePromotionExecutionReceiptV1],
    deployment_receipt: &DeploymentReceiptV1,
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    let promotion_roles = roles
        .iter()
        .map(|role| role.role.as_str())
        .collect::<BTreeSet<_>>();
    let deployment_roles = deployment_receipt
        .role_phase_receipts
        .iter()
        .map(|receipt| receipt.role.as_str())
        .collect::<BTreeSet<_>>();
    for role in &promotion_roles {
        if !deployment_roles.contains(role) {
            return Err(
                ArtifactPromotionExecutionReceiptError::MissingDeploymentRole {
                    role: (*role).to_string(),
                },
            );
        }
    }
    for role in &deployment_roles {
        if !promotion_roles.contains(role) {
            return Err(
                ArtifactPromotionExecutionReceiptError::UnknownDeploymentRole {
                    role: (*role).to_string(),
                },
            );
        }
    }
    for role in roles {
        let role_receipt = deployment_receipt
            .role_phase_receipts
            .iter()
            .rev()
            .find(|receipt| receipt.role == role.role);
        if role.role_phase_result != role_receipt.map(|receipt| receipt.result) {
            return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
                field: "role_phase_result",
            });
        }
        if role.artifact_digest != role_receipt.and_then(|receipt| receipt.artifact_digest.clone())
        {
            return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
                field: "artifact_digest",
            });
        }
        if role.observed_module_hash_after
            != role_receipt.and_then(|receipt| receipt.observed_module_hash_after.clone())
        {
            return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
                field: "observed_module_hash_after",
            });
        }
        if role.canonical_embedded_config_sha256
            != role_receipt.and_then(|receipt| receipt.canonical_embedded_config_sha256.clone())
        {
            return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch {
                field: "canonical_embedded_config_sha256",
            });
        }
    }
    Ok(())
}

fn ensure_unique_execution_receipt_roles(
    roles: &[RolePromotionExecutionReceiptV1],
) -> Result<(), ArtifactPromotionExecutionReceiptError> {
    let mut seen = BTreeSet::new();
    for role in roles {
        ensure_execution_receipt_field("role", &role.role)?;
        if !seen.insert(role.role.as_str()) {
            return Err(ArtifactPromotionExecutionReceiptError::LinkageMismatch { field: "roles" });
        }
        if let Some(evidence_id) = &role.materialization_evidence_id {
            ensure_execution_receipt_field("materialization_evidence_id", evidence_id)?;
        }
        if let Some(digest) = &role.materialization_evidence_digest {
            ensure_execution_receipt_sha256("materialization_evidence_digest", digest)?;
        }
        if let Some(locator) = &role.wasm_store_locator {
            ensure_execution_receipt_field("wasm_store_locator", locator)?;
        }
        if let Some(digest) = &role.wasm_store_catalog_observation_digest {
            ensure_execution_receipt_sha256("wasm_store_catalog_observation_digest", digest)?;
        }
        if let Some(digest) = &role.artifact_digest {
            ensure_execution_receipt_field("artifact_digest", digest)?;
        }
        if let Some(hash) = &role.observed_module_hash_after {
            ensure_execution_receipt_field("observed_module_hash_after", hash)?;
        }
        if let Some(digest) = &role.canonical_embedded_config_sha256 {
            ensure_execution_receipt_field("canonical_embedded_config_sha256", digest)?;
        }
    }
    Ok(())
}
