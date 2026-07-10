use super::super::*;
use super::deployment::phase_receipt;
use crate::deployment_truth::report::{ARTIFACT_MISSING_CODE, is_artifact_role_failure_code};

/// Build a lightweight receipt for the current-install artifact materialization
/// gate. The receipt is evidence only; live inventory/check data remains the
/// authority for any installer decision.
#[must_use]
pub fn artifact_gate_phase_receipt(
    check: &DeploymentCheckV1,
    started_at: impl Into<String>,
    finished_at: Option<String>,
) -> PhaseReceiptV1 {
    let missing = check
        .report
        .hard_failures
        .iter()
        .filter(|finding| finding.code == ARTIFACT_MISSING_CODE)
        .collect::<Vec<_>>();
    let mut evidence = check
        .inventory
        .observed_artifacts
        .iter()
        .filter_map(|artifact| {
            artifact
                .file_sha256
                .as_ref()
                .map(|hash| format!("artifact:{}:sha256:{hash}", artifact.role))
        })
        .collect::<Vec<_>>();
    evidence.extend(
        missing
            .iter()
            .filter_map(|finding| finding.subject.as_ref())
            .map(|role| format!("artifact:{role}:missing")),
    );
    let status = if missing.is_empty() {
        ObservationStatusV1::Observed
    } else {
        ObservationStatusV1::Missing
    };

    phase_receipt(
        "materialize_artifacts",
        started_at,
        finished_at,
        "verify configured role artifacts are materialized",
        status,
        evidence,
    )
}

/// Build role-scoped evidence for the current-install artifact materialization
/// gate.
///
/// These records do not decide safety; they preserve the per-role facts already
/// present in the check so later resume/reporting work can distinguish which
/// roles were verified and which failed materialization.
#[must_use]
pub fn artifact_gate_role_phase_receipts(check: &DeploymentCheckV1) -> Vec<RolePhaseReceiptV1> {
    check
        .plan
        .role_artifacts
        .iter()
        .map(|planned| {
            let observed = check
                .inventory
                .observed_artifacts
                .iter()
                .find(|artifact| artifact.role == planned.role);
            let failures = check
                .report
                .hard_failures
                .iter()
                .filter(|finding| finding.subject.as_deref() == Some(planned.role.as_str()))
                .filter(|finding| is_artifact_role_failure_code(&finding.code))
                .collect::<Vec<_>>();
            let error = if failures.is_empty() {
                None
            } else {
                Some(
                    failures
                        .iter()
                        .map(|finding| format!("{}: {}", finding.code, finding.message))
                        .collect::<Vec<_>>()
                        .join("; "),
                )
            };
            let artifact_digest = observed
                .and_then(|artifact| artifact.file_sha256.clone())
                .or_else(|| observed.and_then(|artifact| artifact.payload_sha256.clone()))
                .or_else(|| planned.observed_wasm_gz_file_sha256.clone())
                .or_else(|| planned.wasm_gz_sha256.clone());
            let result = if !failures.is_empty() {
                RolePhaseResultV1::Failed
            } else if observed
                .and_then(|artifact| artifact.file_sha256.as_ref())
                .is_some()
            {
                RolePhaseResultV1::VerifiedAlreadyApplied
            } else {
                RolePhaseResultV1::NotAttempted
            };

            RolePhaseReceiptV1 {
                role: planned.role.clone(),
                phase: "materialize_artifacts".to_string(),
                result,
                previous_module_hash: None,
                target_module_hash: planned.installed_module_hash.clone(),
                observed_module_hash_after: None,
                artifact_digest,
                canonical_embedded_config_sha256: planned.canonical_embedded_config_sha256.clone(),
                error,
            }
        })
        .collect()
}
