use super::super::{
    digest::artifact_promotion_execution_receipt_digest,
    ensure::{ensure_execution_receipt_field, ensure_execution_receipt_sha256},
    error::ArtifactPromotionExecutionReceiptError,
    request::ArtifactPromotionExecutionReceiptRequest,
};
use super::report::validate_artifact_promotion_provenance_report;
use crate::deployment_truth::{
    ArtifactPromotionExecutionReceiptV1, ArtifactPromotionProvenanceReportV1,
    DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentReceiptV1, PromotionReadinessStatusV1,
    RolePromotionExecutionReceiptV1, RolePromotionProvenanceV1,
};
use std::collections::BTreeSet;

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
