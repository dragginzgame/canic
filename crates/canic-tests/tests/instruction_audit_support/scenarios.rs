use super::*;

// Build the fixed v2 scenario manifest. Every row uses an authoritative
// root-harness artifact and a fresh PocketIC topology.
#[expect(
    clippy::too_many_lines,
    reason = "the fixed ordered scenario table is clearer as one authoritative roster"
)]
pub(super) fn scenarios() -> Vec<AuditScenario> {
    vec![
        AuditScenario {
            key: "scale:request_cycles_from_parent:fresh",
            canister: "scale",
            endpoint_or_flow: "request_cycles_from_parent",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "request_cycles_from_parent",
            arg_class: "cycles-999",
            caller_class: "anonymous",
            auth_state: "public-child-endpoint-and-parent-structural-proof",
            replay_state: "fresh",
            cache_state: "cold",
            topology_state: "scaling-profile-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Scale child update performs a fresh structural capability round trip to its parent.",
        },
        AuditScenario {
            key: "scale_hub:create_worker:empty-pool",
            canister: "scale_hub",
            endpoint_or_flow: "create_worker",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "create_worker",
            arg_class: "empty-pool",
            caller_class: "local-test",
            auth_state: "require-local",
            replay_state: "n/a",
            cache_state: "empty-pool",
            topology_state: "scaling-profile-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Scaling worker creation through observe, plan, create, and registration stages.",
        },
        AuditScenario {
            key: "user_hub:create_account:new-principal",
            canister: "user_hub",
            endpoint_or_flow: "create_account",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "create_account",
            arg_class: "new-principal",
            caller_class: "local-test",
            auth_state: "require-local",
            replay_state: "n/a",
            cache_state: "empty-assignment",
            topology_state: "sharding-profile-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "User-shard assignment and allocation through the maintained user_hub endpoint.",
        },
        AuditScenario {
            key: "root:test_provision_chain_key_delegation_proof_for_issuer:new-issuer",
            canister: "root",
            endpoint_or_flow: "test_provision_chain_key_delegation_proof_for_issuer",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "test_provision_chain_key_delegation_proof_for_issuer",
            arg_class: "registered-new-issuer",
            caller_class: "controller",
            auth_state: "issuer-policy-and-template",
            replay_state: "n/a",
            cache_state: "proof-missing",
            topology_state: "sharding-profile-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Root facade creates and installs the first chain-key delegation proof for an issuer.",
        },
        AuditScenario {
            key: "issuer:canic_prepare_delegated_token:active-proof",
            canister: "issuer",
            endpoint_or_flow: "canic_prepare_delegated_token",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "canic_prepare_delegated_token",
            arg_class: "minimal-valid",
            caller_class: "delegated-subject",
            auth_state: "active-proof",
            replay_state: "fresh",
            cache_state: "proof-warm",
            topology_state: "sharding-profile-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Issuer-local delegated-token preparation from an explicitly provisioned active proof.",
        },
        AuditScenario {
            key: "test:test_verify_delegated_token:valid-delegated-token",
            canister: "test",
            endpoint_or_flow: "test_verify_delegated_token",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "test_verify_delegated_token",
            arg_class: "valid-delegated-token",
            caller_class: "delegated-subject",
            auth_state: "delegated-token",
            replay_state: "fresh",
            cache_state: "proof-warm",
            topology_state: "sharding-profile-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Verifier-side delegated-token confirmation for a freshly issued token.",
        },
        AuditScenario {
            key: "root:canic_response_capability_v1:request-cycles-fresh",
            canister: "root",
            endpoint_or_flow: "canic_response_capability_v1",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "canic_response_capability_v1",
            arg_class: "cycles-request",
            caller_class: "registered-direct-child",
            auth_state: "structural-proof",
            replay_state: "fresh",
            cache_state: "cold",
            topology_state: "capability-profile-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Fresh root capability cycles request through auth, replay, policy, and execution.",
        },
        AuditScenario {
            key: "root:canic_response_capability_v1:request-cycles-replay",
            canister: "root",
            endpoint_or_flow: "canic_response_capability_v1",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "canic_response_capability_v1",
            arg_class: "cycles-request",
            caller_class: "registered-direct-child",
            auth_state: "structural-proof",
            replay_state: "duplicate-same",
            cache_state: "warm-response",
            topology_state: "capability-profile-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Second identical cycles request returns the cached replay response.",
        },
        AuditScenario {
            key: "root:canic_template_stage_manifest_admin:single-chunk",
            canister: "root",
            endpoint_or_flow: "canic_template_stage_manifest_admin",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "canic_template_stage_manifest_admin",
            arg_class: "single-chunk",
            caller_class: "controller",
            auth_state: "controller-only",
            replay_state: "n/a",
            cache_state: "cold",
            topology_state: "capability-profile-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Stage one synthetic approved manifest in the root-local release buffer.",
        },
        AuditScenario {
            key: "root:canic_template_prepare_admin:single-chunk",
            canister: "root",
            endpoint_or_flow: "canic_template_prepare_admin",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "canic_template_prepare_admin",
            arg_class: "single-chunk",
            caller_class: "controller",
            auth_state: "controller-only",
            replay_state: "n/a",
            cache_state: "warm-manifest",
            topology_state: "capability-profile-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Prepare one staged single-chunk release.",
        },
        AuditScenario {
            key: "root:canic_template_publish_chunk_admin:single-chunk",
            canister: "root",
            endpoint_or_flow: "canic_template_publish_chunk_admin",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "canic_template_publish_chunk_admin",
            arg_class: "single-chunk",
            caller_class: "controller",
            auth_state: "controller-only",
            replay_state: "n/a",
            cache_state: "warm-manifest-and-prepare",
            topology_state: "capability-profile-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Publish the only chunk for one prepared release.",
        },
        AuditScenario {
            key: "root:bootstrap:init-checkpoints",
            canister: "root",
            endpoint_or_flow: "root_bootstrap_init",
            transport_mode: "install",
            subject_kind: "flow",
            subject_label: "root_bootstrap_init",
            arg_class: "topology-profile",
            caller_class: "system",
            auth_state: "lifecycle",
            replay_state: "n/a",
            cache_state: "fresh-install",
            topology_state: "topology-profile-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Checkpoint-group observation retained from the completed root init bootstrap flow.",
        },
    ]
}

// Resolve the repo root from this crate's manifest path.
pub(super) fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

// Read the output file layout chosen by the shell runner.
pub(super) fn audit_paths() -> AuditPaths {
    AuditPaths {
        report_path: PathBuf::from(required_env("CANIC_INSTRUCTION_AUDIT_REPORT_PATH")),
        artifacts_dir: PathBuf::from(required_env("CANIC_INSTRUCTION_AUDIT_ARTIFACTS_DIR")),
    }
}

// Read run metadata provided by the shell runner.
pub(super) fn audit_metadata() -> AuditMetadata {
    AuditMetadata {
        code_snapshot: required_env("CANIC_INSTRUCTION_AUDIT_CODE_SNAPSHOT"),
        branch: required_env("CANIC_INSTRUCTION_AUDIT_BRANCH"),
        worktree: required_env("CANIC_INSTRUCTION_AUDIT_WORKTREE"),
        run_timestamp_utc: required_env("CANIC_INSTRUCTION_AUDIT_TIMESTAMP_UTC"),
        compared_baseline_report: required_env("CANIC_INSTRUCTION_AUDIT_BASELINE_REPORT"),
        method_id: required_env("CANIC_INSTRUCTION_AUDIT_METHOD_ID"),
        method_version: required_env("CANIC_INSTRUCTION_AUDIT_METHOD_VERSION"),
        method_fingerprint: required_env("CANIC_INSTRUCTION_AUDIT_METHOD_FINGERPRINT"),
    }
}

// Return the current workspace minor line as `<major>.<minor>`.
pub(super) fn current_minor_line() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let mut parts = version.split('.');
    let major = parts.next().unwrap_or("0");
    let minor = parts.next().unwrap_or("0");
    format!("{major}.{minor}")
}

fn required_env(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("missing required env var: {key}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v2_roster_is_fixed_complete_and_query_free() {
        let actual = scenarios();
        let expected_keys = [
            "scale:request_cycles_from_parent:fresh",
            "scale_hub:create_worker:empty-pool",
            "user_hub:create_account:new-principal",
            "root:test_provision_chain_key_delegation_proof_for_issuer:new-issuer",
            "issuer:canic_prepare_delegated_token:active-proof",
            "test:test_verify_delegated_token:valid-delegated-token",
            "root:canic_response_capability_v1:request-cycles-fresh",
            "root:canic_response_capability_v1:request-cycles-replay",
            "root:canic_template_stage_manifest_admin:single-chunk",
            "root:canic_template_prepare_admin:single-chunk",
            "root:canic_template_publish_chunk_admin:single-chunk",
            "root:bootstrap:init-checkpoints",
        ];

        assert_eq!(actual.len(), expected_keys.len());
        assert_eq!(
            actual
                .iter()
                .map(|scenario| scenario.key)
                .collect::<Vec<_>>(),
            expected_keys
        );
        assert!(actual.iter().all(|scenario| {
            matches!(scenario.transport_mode, "update" | "install")
                && scenario.freshness_model == "fresh-topology-per-scenario"
        }));
    }
}
