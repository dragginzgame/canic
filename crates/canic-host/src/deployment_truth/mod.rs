//! Passive deployment-truth model types for host-side planning and safety checks.

use sha2::{Digest, Sha256};
use std::{
    fmt::Write as _,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

mod model;
mod observe;
mod plan;
mod receipt;
mod report;
#[cfg(test)]
mod tests;

pub use model::{
    ArtifactDigestSourceV1, ArtifactSourceV1, AuthorityProfileV1, CanisterControlClassV1,
    DeploymentAssumptionV1, DeploymentCheckV1, DeploymentCommandResultV1, DeploymentDiffV1,
    DeploymentExecutionStatusV1, DeploymentIdentityV1, DeploymentInventoryV1,
    DeploymentObservationGapV1, DeploymentPlanV1, DeploymentReceiptV1, DiffItemV1,
    ExpectedCanisterV1, ExpectedPoolCanisterV1, LocalDeploymentConfigV1, ObservationStatusV1,
    ObservedArtifactV1, ObservedCanisterV1, ObservedPoolCanisterV1, PhaseReceiptV1, ResumeSafetyV1,
    RoleArtifactManifestV1, RoleArtifactV1, RoleEpochExpectationV1, RoleEpochObservationV1,
    RolePhaseReceiptV1, RolePhaseResultV1, SafetyFindingV1, SafetyReportV1, SafetySeverityV1,
    SafetyStatusV1, TrustDomainV1, VerifiedPostconditionV1, VerifierReadinessExpectationV1,
    VerifierReadinessObservationV1,
};
pub use observe::{
    DeploymentTruthError, LocalArtifactManifestRequest, LocalInventoryRequest,
    collect_local_deployment_inventory, collect_local_role_artifact_manifest,
};
pub use plan::{LocalDeploymentPlanRequest, build_local_deployment_plan};
pub use receipt::{
    artifact_gate_phase_receipt, artifact_gate_role_phase_receipts, deployment_receipt_from_check,
    deployment_receipt_from_check_with_status, phase_receipt,
};
pub use report::{
    LocalDeploymentCheckRequest, check_local_deployment, compare_plan_inventory_and_receipt,
    compare_plan_to_inventory, safety_report_from_diff,
};

pub const DEPLOYMENT_TRUTH_SCHEMA_VERSION: u32 = 1;

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
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut hex, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Ok(hex)
}
