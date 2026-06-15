use super::*;

pub(in crate::deployment_truth::tests) fn sample_receipt_with_phase(
    plan_id: &str,
    root_principal: Option<&str>,
    postcondition: ObservationStatusV1,
    role_result: RolePhaseResultV1,
) -> DeploymentReceiptV1 {
    DeploymentReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        operation_id: "operation-1".to_string(),
        plan_id: plan_id.to_string(),
        execution_context: None,
        operation_status: DeploymentExecutionStatusV1::Complete,
        started_at: "2026-05-22T00:00:00Z".to_string(),
        finished_at: Some("2026-05-22T00:00:01Z".to_string()),
        operator_principal: None,
        root_principal: root_principal.map(str::to_string),
        previous_observed_deployment_epoch: None,
        phase_receipts: vec![PhaseReceiptV1 {
            phase: "materialize_artifacts".to_string(),
            started_at: "2026-05-22T00:00:00Z".to_string(),
            finished_at: Some("2026-05-22T00:00:01Z".to_string()),
            attempted_action: "verify configured role artifacts are materialized".to_string(),
            verified_postcondition: VerifiedPostconditionV1 {
                status: postcondition,
                evidence: vec!["artifact:root:sha256:file".to_string()],
            },
        }],
        role_phase_receipts: vec![RolePhaseReceiptV1 {
            role: "root".to_string(),
            phase: "materialize_artifacts".to_string(),
            result: role_result,
            previous_module_hash: None,
            target_module_hash: Some("module".to_string()),
            observed_module_hash_after: None,
            artifact_digest: Some("file".to_string()),
            canonical_embedded_config_sha256: Some("canonical".to_string()),
            error: (role_result == RolePhaseResultV1::Failed)
                .then(|| "artifact_missing: missing observed artifact for role root".to_string()),
        }],
        final_inventory_id: Some("inventory-1".to_string()),
        command_result: DeploymentCommandResultV1::Succeeded,
    }
}

pub(in crate::deployment_truth::tests) fn sample_wasm_store_staging_receipt() -> StagingReceiptV1 {
    StagingReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        role: "root".to_string(),
        artifact_identity: "embedded:root:0.44.0:abc123".to_string(),
        transport: ArtifactTransportV1::WasmStore,
        wasm_store_locator: Some("root:aaaaa-aa:bootstrap".to_string()),
        prepared_chunk_hashes: vec!["chunk-a".to_string(), "chunk-b".to_string()],
        published_chunk_count: 2,
        verified_postcondition: VerifiedPostconditionV1 {
            status: ObservationStatusV1::Observed,
            evidence: vec!["payload_sha256:abc123".to_string()],
        },
    }
}

pub(in crate::deployment_truth::tests) fn sample_role_phase_receipt(
    result: RolePhaseResultV1,
) -> RolePhaseReceiptV1 {
    RolePhaseReceiptV1 {
        role: "root".to_string(),
        phase: "install_root".to_string(),
        result,
        previous_module_hash: None,
        target_module_hash: Some("module".to_string()),
        observed_module_hash_after: (result == RolePhaseResultV1::Applied)
            .then(|| "module".to_string()),
        artifact_digest: Some("file".to_string()),
        canonical_embedded_config_sha256: Some("canonical".to_string()),
        error: (result == RolePhaseResultV1::Failed).then(|| "install failed".to_string()),
    }
}
