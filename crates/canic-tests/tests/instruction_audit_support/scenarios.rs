use super::*;

// Build the fixed scenario manifest for the first 0.20 instruction baseline.
#[allow(clippy::too_many_lines)]
pub(super) fn scenarios() -> Vec<AuditScenario> {
    let mut scenarios = vec![
        AuditScenario {
            key: "app:canic_time:minimal-valid",
            canister: "leaf_probe",
            endpoint_or_flow: "audit_time_probe",
            transport_mode: "query",
            subject_kind: "endpoint",
            subject_label: "time_probe",
            arg_class: "minimal-valid",
            caller_class: "anonymous",
            auth_state: "public",
            replay_state: "n/a",
            cache_state: "n/a",
            topology_state: "standalone-audit-leaf-ready",
            freshness_model: "fresh-standalone-per-scenario",
            notes: "Audit-only raw time probe on one standalone internal leaf canister.",
        },
        AuditScenario {
            key: "app:canic_env:minimal-valid",
            canister: "leaf_probe",
            endpoint_or_flow: "audit_env_probe",
            transport_mode: "query",
            subject_kind: "endpoint",
            subject_label: "canic_env",
            arg_class: "minimal-valid",
            caller_class: "anonymous",
            auth_state: "public",
            replay_state: "n/a",
            cache_state: "n/a",
            topology_state: "standalone-audit-leaf-ready",
            freshness_model: "fresh-standalone-per-scenario",
            notes: "Audit-only env snapshot probe on one standalone internal leaf canister.",
        },
        AuditScenario {
            key: "app:canic_log:empty-page",
            canister: "leaf_probe",
            endpoint_or_flow: "audit_log_probe",
            transport_mode: "query",
            subject_kind: "endpoint",
            subject_label: "canic_log",
            arg_class: "empty-page",
            caller_class: "anonymous",
            auth_state: "public",
            replay_state: "n/a",
            cache_state: "cold",
            topology_state: "standalone-audit-leaf-ready",
            freshness_model: "fresh-standalone-per-scenario",
            notes: "Audit-only log pagination probe with the smallest page shape on one standalone internal leaf.",
        },
        AuditScenario {
            key: "root:canic_subnet_registry:full-registry",
            canister: "root_probe",
            endpoint_or_flow: "audit_subnet_registry_probe",
            transport_mode: "query",
            subject_kind: "endpoint",
            subject_label: "canic_subnet_registry",
            arg_class: "representative-valid",
            caller_class: "anonymous",
            auth_state: "public",
            replay_state: "n/a",
            cache_state: "n/a",
            topology_state: "standalone-audit-root-ready",
            freshness_model: "fresh-standalone-per-scenario",
            notes: "Audit-only root registry probe over a standalone internal root canister.",
        },
        AuditScenario {
            key: "root:canic_subnet_state:empty-struct",
            canister: "root_probe",
            endpoint_or_flow: "audit_subnet_state_probe",
            transport_mode: "query",
            subject_kind: "endpoint",
            subject_label: "canic_subnet_state",
            arg_class: "minimal-valid",
            caller_class: "anonymous",
            auth_state: "public",
            replay_state: "n/a",
            cache_state: "n/a",
            topology_state: "standalone-audit-root-ready",
            freshness_model: "fresh-standalone-per-scenario",
            notes: "Audit-only root state probe on a standalone internal root canister.",
        },
        AuditScenario {
            key: "scale_hub:plan_create_worker:empty-pool",
            canister: "scaling_probe",
            endpoint_or_flow: "audit_plan_create_worker_probe",
            transport_mode: "query",
            subject_kind: "endpoint",
            subject_label: "plan_create_worker",
            arg_class: "empty-pool",
            caller_class: "anonymous",
            auth_state: "local-test-only",
            replay_state: "n/a",
            cache_state: "n/a",
            topology_state: "standalone-audit-scaling-ready",
            freshness_model: "fresh-standalone-per-scenario",
            notes: "Audit-only scaling dry-run probe before any extra worker exists in one standalone internal scaling canister.",
        },
        AuditScenario {
            key: "root:canic_request_delegation:fresh-shard",
            canister: "root",
            endpoint_or_flow: "canic_request_delegation",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "canic_request_delegation",
            arg_class: "fresh-shard",
            caller_class: "registered-shard",
            auth_state: "registered-subnet-caller",
            replay_state: "fresh",
            cache_state: "cold",
            topology_state: "root_bootstrapped+fresh-user-shard",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Root delegation provisioning request from a freshly created shard to exercise delegated auth issuance and signer key checkpoints.",
        },
        AuditScenario {
            key: "test:test:minimal-valid",
            canister: "test",
            endpoint_or_flow: "test",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "test",
            arg_class: "minimal-valid",
            caller_class: "anonymous",
            auth_state: "local-test-only",
            replay_state: "n/a",
            cache_state: "n/a",
            topology_state: "standalone-test-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Minimal local/dev update on one standalone test helper canister with no chain-key dependency.",
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
            topology_state: "root_bootstrapped+local-test-helper-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Fresh root capability cycles request from a registered direct child to exercise replay and dispatcher checkpoints.",
        },
        AuditScenario {
            key: "root:canic_template_stage_manifest_admin:single-chunk",
            canister: "root",
            endpoint_or_flow: "canic_template_stage_manifest_admin",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "canic_template_stage_manifest_admin",
            arg_class: "single-chunk",
            caller_class: "anonymous-controller",
            auth_state: "controller-only",
            replay_state: "n/a",
            cache_state: "cold",
            topology_state: "root_bootstrapped+release-staging-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Stages one synthetic approved manifest into the root-local release buffer.",
        },
        AuditScenario {
            key: "root:canic_template_prepare_admin:single-chunk",
            canister: "root",
            endpoint_or_flow: "canic_template_prepare_admin",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "canic_template_prepare_admin",
            arg_class: "single-chunk",
            caller_class: "anonymous-controller",
            auth_state: "controller-only",
            replay_state: "n/a",
            cache_state: "warm-manifest",
            topology_state: "root_bootstrapped+release-staging-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Prepares one synthetic single-chunk release after its manifest is already staged.",
        },
        AuditScenario {
            key: "root:canic_template_publish_chunk_admin:single-chunk",
            canister: "root",
            endpoint_or_flow: "canic_template_publish_chunk_admin",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "canic_template_publish_chunk_admin",
            arg_class: "single-chunk",
            caller_class: "anonymous-controller",
            auth_state: "controller-only",
            replay_state: "n/a",
            cache_state: "warm-manifest+prepared",
            topology_state: "root_bootstrapped+release-staging-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Publishes the only chunk for one synthetic staged release after prepare has completed.",
        },
    ];

    if NetworkApi::build_network() == Some(BuildNetwork::Ic) {
        scenarios.push(AuditScenario {
            key: "test:test_verify_delegated_token:valid-delegated-token",
            canister: "test",
            endpoint_or_flow: "test_verify_delegated_token",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "test_verify_delegated_token",
            arg_class: "valid-delegated-token",
            caller_class: "delegated-subject",
            auth_state: "delegated-token",
            replay_state: "n/a",
            cache_state: "cold",
            topology_state: "root_bootstrapped+fresh-user-shard+verifier-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Verifier-side delegated token confirmation on the shared test canister using a freshly minted token from a newly created user shard.",
        });
    }

    scenarios
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
    }
}

// Return the current workspace minor line like `0.24`.
pub(super) fn current_minor_line() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let mut parts = version.split('.');
    let major = parts.next().unwrap_or("0");
    let minor = parts.next().unwrap_or("0");
    format!("{major}.{minor}")
}

// Require one environment variable and panic early when the runner forgot it.
fn required_env(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("missing required env var: {key}"))
}
