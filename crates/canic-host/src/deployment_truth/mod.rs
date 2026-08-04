//! Passive deployment-truth model types for host-side planning and safety checks.

use crate::release_set::AppConfigSnapshot;
use canic_core::cdk::utils::hash::{hex_bytes, sha256_hex};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

mod authority;
mod executor;
mod model;
mod multi;
mod observe;
mod plan;
mod receipt;
mod report;
#[cfg(test)]
mod tests;
mod text;

pub use authority::build_authority_reconciliation_plan;
pub use executor::{
    CURRENT_CLI_EXECUTOR_CAPABILITIES, CurrentCliDeploymentExecutor,
    DeploymentExecutionPreflightError, DeploymentExecutor, TESTKIT_PREFLIGHT_CAPABILITIES,
    TestkitPreflightContext, current_cli_execution_context, deployment_execution_preflight,
    deployment_execution_preflight_from_check, has_executor_capabilities,
    missing_executor_capabilities, testkit_execution_context,
    validate_deployment_execution_preflight, validate_deployment_execution_preflight_for_check,
};
pub use model::{
    ArtifactDigestSourceV1, ArtifactSourceV1, AuthorityActionV1, AuthorityAutomaticActionV1,
    AuthorityControllerDeltaV1, AuthorityExternalActionV1, AuthorityProfileV1,
    AuthorityReconciliationPlanV1, AuthorityReconciliationStateV1, CanisterAuthorityActionV1,
    CanisterControlClassV1, DeploymentAssumptionKindV1, DeploymentAssumptionV1, DeploymentCheckV1,
    DeploymentCommandResultV1, DeploymentComparisonCategoryV1, DeploymentComparisonDiffV1,
    DeploymentComparisonReportV1, DeploymentComparisonTargetV1, DeploymentDiffV1,
    DeploymentExecutionContextV1, DeploymentExecutionPreflightStatusV1,
    DeploymentExecutionPreflightV1, DeploymentExecutionStatusV1, DeploymentExecutorBackendV1,
    DeploymentExecutorCapabilityV1, DeploymentIdentityV1, DeploymentInventoryV1,
    DeploymentObservationGapV1, DeploymentPlanV1, DeploymentReceiptV1,
    DeploymentRootObservationSourceV1, DeploymentRootObservationV1, DiffItemV1, ExpectedCanisterV1,
    ExpectedPoolCanisterV1, LocalDeploymentConfigV1, ObservationStatusV1, ObservedArtifactV1,
    ObservedCanisterV1, ObservedPoolCanisterV1, PhaseReceiptV1, ResumeSafetyV1,
    RoleArtifactManifestV1, RoleArtifactV1, RoleAssignmentSourceV1, RoleEpochExpectationV1,
    RoleEpochObservationV1, RolePhaseReceiptV1, RolePhaseResultV1, SafetyFindingV1, SafetyReportV1,
    SafetySeverityV1, SafetyStatusV1, TrustDomainV1, VerifiedPostconditionV1,
    VerifierReadinessExpectationV1, VerifierReadinessObservationV1,
};
pub use multi::{
    DeploymentComparisonReportError, deployment_comparison_report_from_checks,
    validate_deployment_comparison_report,
};
pub use observe::{
    DeploymentTruthError, LocalArtifactManifestRequest, LocalInventoryRequest,
    collect_local_deployment_inventory, collect_local_role_artifact_manifest,
};
pub(crate) use observe::{
    collect_local_deployment_inventory_at_root, collect_local_role_artifact_manifest_at_root,
};
pub(crate) use plan::build_local_deployment_plan_at_root;
pub use plan::{LocalDeploymentPlanRequest, build_local_deployment_plan};
pub use receipt::{
    artifact_gate_phase_receipt, artifact_gate_role_phase_receipts,
    deployment_execution_status_for_receipt_parts, deployment_receipt_from_check,
    deployment_receipt_from_check_with_status, phase_receipt,
};
pub(crate) use report::check_local_deployment_at_root;
pub use report::{
    LocalDeploymentCheckRequest, check_local_deployment, compare_plan_inventory_and_receipt,
    compare_plan_to_inventory, is_evidence_conflict_finding_code, safety_report_from_diff,
};
pub use text::deployment_comparison_report_text;

pub const DEPLOYMENT_TRUTH_SCHEMA_VERSION: u32 = 1;
const ROOT_ROLE: &str = "root";
const IMPLICIT_WASM_STORE_ROLE: &str = "wasm_store";

fn deployment_truth_roles_with_implicit_wasm_store(mut roles: Vec<String>) -> Vec<String> {
    if !roles.iter().any(|role| role == IMPLICIT_WASM_STORE_ROLE) {
        roles.push(IMPLICIT_WASM_STORE_ROLE.to_string());
    }
    roles.sort_by(|left, right| {
        deployment_truth_role_rank(left)
            .cmp(&deployment_truth_role_rank(right))
            .then_with(|| left.cmp(right))
    });
    roles.dedup();
    roles
}

fn deployment_truth_role_rank(role: &str) -> u8 {
    match role {
        ROOT_ROLE => 0,
        IMPLICIT_WASM_STORE_ROLE => 1,
        _ => 2,
    }
}

fn deployment_truth_artifact_source(role: &str) -> ArtifactSourceV1 {
    match role {
        IMPLICIT_WASM_STORE_ROLE => ArtifactSourceV1::WasmStore,
        _ => ArtifactSourceV1::LocalBuild,
    }
}

fn deployment_config_path(workspace_root: &Path, config_path: Option<&Path>) -> PathBuf {
    config_path.map_or_else(
        || crate::release_set::config_path(workspace_root),
        |path| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                workspace_root.join(path)
            }
        },
    )
}

fn file_sha256_hex(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_bytes(hasher.finalize()))
}

fn canonical_runtime_config_sha256_hex(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let config = AppConfigSnapshot::load(path)?;
    Ok(sha256_hex(&serde_json::to_vec(config.model())?))
}

fn stable_json_sha256_hex<T: Serialize>(value: &T) -> String {
    sha256_hex(
        &serde_json::to_vec(value)
            .expect("deployment truth identity inputs must JSON-encode deterministically"),
    )
}
