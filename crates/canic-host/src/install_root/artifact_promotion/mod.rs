use super::phase_receipts::{completed_phase_role_receipt, receipt_with_execution_context};
use super::receipt_io::write_artifact_promotion_execution_receipt;
use super::{clock::current_unix_timestamp_label, options::InstallRootOptions};
use crate::deployment_truth::{
    ArtifactPromotionExecutionReceiptRequest, ArtifactPromotionPlanV1,
    ArtifactPromotionProvenanceReportRequest, DeploymentCheckV1, DeploymentCommandResultV1,
    DeploymentExecutionContextV1, DeploymentExecutionStatusV1, DeploymentReceiptV1,
    ObservationStatusV1, RolePhaseResultV1, artifact_promotion_execution_receipt,
    artifact_promotion_provenance_report, deployment_receipt_from_check_with_status, phase_receipt,
};
use std::path::{Path, PathBuf};

pub(super) fn write_artifact_promotion_execution_receipt_for_install(
    options: &InstallRootOptions,
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
    check: &DeploymentCheckV1,
    execution_context: &DeploymentExecutionContextV1,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let Some(promotion_plan) = &options.artifact_promotion_plan_override else {
        return Ok(None);
    };
    let deployment_receipt =
        promotion_install_deployment_receipt(check, execution_context, promotion_plan)?;
    let provenance_report =
        artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
            report_id: format!("{}:execution-provenance", promotion_plan.plan_id),
            artifact_promotion_plan: promotion_plan.clone(),
            wasm_store_identity_report: None,
            wasm_store_catalog_verification: None,
            materialization_identity_report: None,
        })?;
    let receipt = artifact_promotion_execution_receipt(ArtifactPromotionExecutionReceiptRequest {
        receipt_id: format!("{}:execution-receipt", promotion_plan.plan_id),
        provenance_report,
        deployment_receipt,
    })?;
    let path =
        write_artifact_promotion_execution_receipt(icp_root, network, deployment_name, &receipt)?;
    println!(
        "Artifact promotion execution receipt JSON: {}",
        path.display()
    );
    Ok(Some(path))
}

fn promotion_install_deployment_receipt(
    check: &DeploymentCheckV1,
    execution_context: &DeploymentExecutionContextV1,
    promotion_plan: &ArtifactPromotionPlanV1,
) -> Result<DeploymentReceiptV1, Box<dyn std::error::Error>> {
    let started_at = current_unix_timestamp_label()?;
    let finished_at = current_unix_timestamp_label()?;
    let phase = phase_receipt(
        "promoted_plan_install",
        started_at.clone(),
        Some(finished_at.clone()),
        "execute promoted deployment plan through current install runner",
        ObservationStatusV1::Observed,
        vec![
            format!("artifact_promotion_plan:{}", promotion_plan.plan_id),
            format!(
                "artifact_promotion_plan_digest:{}",
                promotion_plan.artifact_promotion_plan_digest
            ),
            format!(
                "promotion_plan_lineage_digest:{}",
                promotion_plan.promotion_plan_lineage_digest
            ),
        ],
    );
    let role_phase_receipts = promotion_plan
        .transform
        .roles
        .iter()
        .map(|role| {
            completed_phase_role_receipt(
                check,
                "promoted_plan_install",
                &role.role,
                RolePhaseResultV1::Applied,
                None,
            )
            .ok_or_else(|| {
                format!(
                    "promoted role {} is missing from deployment plan artifacts",
                    role.role
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(receipt_with_execution_context(
        deployment_receipt_from_check_with_status(
            check,
            format!("{}:promoted_plan_install", check.check_id),
            DeploymentExecutionStatusV1::Complete,
            started_at,
            Some(finished_at),
            vec![phase],
            role_phase_receipts,
            DeploymentCommandResultV1::Succeeded,
        ),
        execution_context,
    ))
}
