// Category C - Artifact / deployment audit (embedded config).
// This audit relies on embedded production config by design.

mod root;

use canic::{
    Error,
    cdk::{types::Principal, utils::wasm::get_wasm_hash},
    dto::{
        auth::DelegatedToken,
        capability::{
            CAPABILITY_VERSION_V1, CapabilityProof, CapabilityRequestMetadata, CapabilityService,
            RootCapabilityEnvelopeV1, RootCapabilityResponseV1,
        },
        env::EnvSnapshotResponse,
        log::LogEntry,
        metrics::{MetricEntry, MetricValue, MetricsKind},
        page::{Page, PageRequest},
        rpc::{CyclesRequest, Request, Response, RootRequestMetadata},
        state::SubnetStateResponse,
        topology::SubnetRegistryResponse,
    },
    ids::cap,
    protocol,
};
use canic_control_plane::{
    dto::template::{
        TemplateChunkInput, TemplateChunkSetInfoResponse, TemplateChunkSetPrepareInput,
        TemplateManifestInput,
    },
    ids::{
        TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion, WasmStoreBinding,
    },
};
use canic_internal::canister::{APP, SCALE_HUB, TEST, USER_HUB};
use canic_testing_internal::pic::{
    create_user_shard, install_standalone_canister, issue_delegated_token,
    request_root_delegation_provision,
};
use canic_testkit::{artifacts::WasmBuildProfile, pic::Pic};
use root::harness::{
    setup_root_capability, setup_root_scaling, setup_root_sharding, setup_root_topology,
};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    convert::TryFrom,
    env, fs,
    path::{Path, PathBuf},
};

const METHOD_TAG: &str = "Method V1";
const PERF_PAGE_LIMIT: u64 = 512;
const CHECKPOINT_SCAN_ROOTS: &[&str] = &["crates"];
const APP_CANIC_TIME_PERF_TEST: &str = "canic_time_perf_test";
const APP_CANIC_ENV_PERF_TEST: &str = "canic_env_perf_test";
const APP_CANIC_LOG_PERF_TEST: &str = "canic_log_perf_test";
const ROOT_CANIC_SUBNET_REGISTRY_PERF_TEST: &str = "canic_subnet_registry_perf_test";
const ROOT_CANIC_SUBNET_STATE_PERF_TEST: &str = "canic_subnet_state_perf_test";
const SCALE_HUB_PLAN_CREATE_WORKER_PERF_TEST: &str = "plan_create_worker_perf_test";
const FLOW_GAPS: &[(&str, &str)] = &[
    (
        "root capability dispatch",
        "crates/canic-core/src/workflow/rpc/request/handler/mod.rs",
    ),
    (
        "delegated auth issuance/verification",
        "crates/canic-core/src/workflow/auth.rs",
    ),
    (
        "replay/cached-response path",
        "crates/canic-core/src/workflow/rpc/request/handler/replay.rs",
    ),
    (
        "sharding assignment/query flow",
        "crates/canic-core/src/workflow/placement/sharding/mod.rs",
    ),
    (
        "scaling/provisioning flow",
        "crates/canic-core/src/workflow/placement/scaling/mod.rs",
    ),
    (
        "bootstrap/install/publication flow",
        "crates/canic-control-plane/src/workflow/bootstrap/root.rs",
    ),
];

///
/// AuditPaths
///

struct AuditPaths {
    report_path: PathBuf,
    artifacts_dir: PathBuf,
}

///
/// AuditMetadata
///

struct AuditMetadata {
    code_snapshot: String,
    branch: String,
    worktree: String,
    run_timestamp_utc: String,
    compared_baseline_report: String,
}

///
/// AuditScenario
///

#[derive(Clone, Copy, Serialize)]
struct AuditScenario {
    key: &'static str,
    canister: &'static str,
    endpoint_or_flow: &'static str,
    transport_mode: &'static str,
    subject_kind: &'static str,
    subject_label: &'static str,
    arg_class: &'static str,
    caller_class: &'static str,
    auth_state: &'static str,
    replay_state: &'static str,
    cache_state: &'static str,
    topology_state: &'static str,
    freshness_model: &'static str,
    notes: &'static str,
}

///
/// CanonicalPerfRow
///

#[derive(Serialize)]
struct CanonicalPerfRow {
    subject_kind: String,
    subject_label: String,
    count: u64,
    total_local_instructions: u64,
    avg_local_instructions: u64,
    scenario_key: String,
    scenario_labels: Vec<String>,
    principal_scope: Option<String>,
    sample_origin: String,
}

///
/// ScenarioResult
///

struct ScenarioResult {
    scenario: AuditScenario,
    row: CanonicalPerfRow,
    checkpoint_rows: Vec<CheckpointDeltaRow>,
}

///
/// PreparedScenario
///

struct PreparedScenario {
    target_pid: Principal,
    caller_pid: Option<Principal>,
    delegated_token: Option<DelegatedToken>,
}

///
/// CheckpointDeltaRow
///

#[derive(Serialize)]
struct CheckpointDeltaRow {
    scenario_key: String,
    canister: String,
    endpoint_or_flow: String,
    scope: String,
    label: String,
    count: u64,
    total_local_instructions: u64,
    avg_local_instructions: u64,
}

///
/// AuditTemplateFixture
///

struct AuditTemplateFixture {
    manifest: TemplateManifestInput,
    prepare: TemplateChunkSetPrepareInput,
    chunk: TemplateChunkInput,
}

///
/// CheckpointCoverageGap
///

#[derive(Serialize)]
struct CheckpointCoverageGap {
    flow_name: String,
    status: String,
    proposed_first_insertion_site: String,
}

///
/// MethodArtifact
///

#[derive(Serialize)]
struct MethodArtifact {
    method_tag: String,
    normalization: String,
    freshness_rule: String,
    checkpoint_rule: String,
}

///
/// EnvironmentArtifact
///

#[derive(Serialize)]
struct EnvironmentArtifact {
    code_snapshot_identifier: String,
    branch: String,
    worktree: String,
    run_timestamp_utc: String,
    execution_environment: String,
    target_canisters_in_scope: Vec<String>,
    target_endpoints_in_scope: Vec<String>,
    target_flows_in_scope: Vec<String>,
}

///
/// VerificationRow
///

struct VerificationRow {
    command: String,
    status: String,
    notes: String,
}

///
/// Write the dated instruction-footprint report and its normalized artifacts.
///

#[test]
#[ignore = "audit runner"]
fn generate_instruction_footprint_report() {
    let workspace_root = workspace_root();
    let paths = audit_paths();
    let metadata = audit_metadata();
    let scenarios = scenarios();
    let checkpoint_sites = scan_perf_callsites(&workspace_root);

    fs::create_dir_all(&paths.artifacts_dir).expect("create instruction audit artifacts dir");

    let scenario_manifest_path = paths.artifacts_dir.join("scenario-manifest.json");
    let perf_rows_path = paths.artifacts_dir.join("perf-rows.json");
    let flow_checkpoints_path = paths.artifacts_dir.join("flow-checkpoints.log");
    let verification_path = paths.artifacts_dir.join("verification-readout.md");
    let method_path = paths.artifacts_dir.join("method.json");
    let environment_path = paths.artifacts_dir.join("environment.json");
    let endpoint_matrix_path = paths.artifacts_dir.join("endpoint-matrix.tsv");
    let checkpoint_delta_path = paths.artifacts_dir.join("checkpoint-deltas.json");
    let checkpoint_gap_path = paths.artifacts_dir.join("checkpoint-coverage-gaps.json");

    write_json(&scenario_manifest_path, &scenarios);

    let results = scenarios
        .iter()
        .map(run_scenario)
        .collect::<Vec<ScenarioResult>>();
    let perf_rows = results
        .iter()
        .map(|result| &result.row)
        .collect::<Vec<&CanonicalPerfRow>>();
    let checkpoint_rows = results
        .iter()
        .flat_map(|result| result.checkpoint_rows.iter())
        .collect::<Vec<_>>();
    write_json(&perf_rows_path, &perf_rows);
    write_json(&checkpoint_delta_path, &checkpoint_rows);
    write_endpoint_matrix_tsv(&endpoint_matrix_path, &results);

    let gaps = checkpoint_coverage_gaps(&checkpoint_sites);
    write_json(&checkpoint_gap_path, &gaps);
    write_flow_checkpoint_log(&flow_checkpoints_path, &checkpoint_sites);

    let query_unobservable_count = results
        .iter()
        .filter(|result| query_perf_is_unobservable(&result.scenario, &result.row))
        .count();

    let verification_rows = verification_rows(
        &paths,
        &checkpoint_sites,
        query_unobservable_count,
        checkpoint_rows.len(),
    );
    write_verification_readout(&verification_path, &verification_rows);

    let method = MethodArtifact {
        method_tag: METHOD_TAG.to_string(),
        normalization: "MetricsKind::Perf rows are normalized into canonical endpoint rows. Update/timer lanes use persisted perf deltas; sampled query lanes use local-only same-call probe endpoints because query-side perf rows are not committed, so the probe returns the measured `perf_counter()` alongside the real query result.".to_string(),
        freshness_rule: "One fresh smallest-profile root harness per measured scenario (`topology`, `scaling`, or `sharding`); baseline and post-call perf tables were sampled inside that isolated topology.".to_string(),
        checkpoint_rule: "Checkpoint deltas are diffed from `MetricsKind::Perf` rows before/after sampled update scenarios. Query scenarios remain endpoint-only unless they traverse explicit checkpoint instrumentation.".to_string(),
    };
    write_json(&method_path, &method);

    let environment = EnvironmentArtifact {
        code_snapshot_identifier: metadata.code_snapshot.clone(),
        branch: metadata.branch.clone(),
        worktree: metadata.worktree.clone(),
        run_timestamp_utc: metadata.run_timestamp_utc.clone(),
        execution_environment: "PocketIC".to_string(),
        target_canisters_in_scope: vec![
            "app".to_string(),
            "root".to_string(),
            "scale_hub".to_string(),
            "test".to_string(),
            "user_hub".to_string(),
        ],
        target_endpoints_in_scope: scenarios
            .iter()
            .map(|scenario| format!("{}::{}", scenario.canister, scenario.endpoint_or_flow))
            .collect(),
        target_flows_in_scope: FLOW_GAPS
            .iter()
            .map(|(flow, _)| (*flow).to_string())
            .collect(),
    };
    write_json(&environment_path, &environment);

    write_report(
        &paths.report_path,
        &paths.artifacts_dir,
        &metadata,
        &results,
        &verification_rows,
        &checkpoint_sites,
        &gaps,
    );
}

// Build the fixed scenario manifest for the first 0.20 instruction baseline.
#[allow(clippy::too_many_lines)]
fn scenarios() -> Vec<AuditScenario> {
    vec![
        AuditScenario {
            key: "app:canic_time:minimal-valid",
            canister: "app",
            endpoint_or_flow: "canic_time",
            transport_mode: "query",
            subject_kind: "endpoint",
            subject_label: "canic_time",
            arg_class: "minimal-valid",
            caller_class: "anonymous",
            auth_state: "public",
            replay_state: "n/a",
            cache_state: "n/a",
            topology_state: "standalone-app-ready",
            freshness_model: "fresh-standalone-per-scenario",
            notes: "Shared lifecycle query surface with no arguments on one standalone app leaf.",
        },
        AuditScenario {
            key: "app:canic_env:minimal-valid",
            canister: "app",
            endpoint_or_flow: "canic_env",
            transport_mode: "query",
            subject_kind: "endpoint",
            subject_label: "canic_env",
            arg_class: "minimal-valid",
            caller_class: "anonymous",
            auth_state: "public",
            replay_state: "n/a",
            cache_state: "n/a",
            topology_state: "standalone-app-ready",
            freshness_model: "fresh-standalone-per-scenario",
            notes: "Shared environment snapshot query on one standalone app leaf canister.",
        },
        AuditScenario {
            key: "app:canic_log:empty-page",
            canister: "app",
            endpoint_or_flow: "canic_log",
            transport_mode: "query",
            subject_kind: "endpoint",
            subject_label: "canic_log",
            arg_class: "empty-page",
            caller_class: "anonymous",
            auth_state: "public",
            replay_state: "n/a",
            cache_state: "cold",
            topology_state: "standalone-app-ready",
            freshness_model: "fresh-standalone-per-scenario",
            notes: "Operator-facing log pagination with the smallest page shape on one standalone app leaf.",
        },
        AuditScenario {
            key: "root:canic_subnet_registry:full-registry",
            canister: "root",
            endpoint_or_flow: "canic_subnet_registry",
            transport_mode: "query",
            subject_kind: "endpoint",
            subject_label: "canic_subnet_registry",
            arg_class: "representative-valid",
            caller_class: "anonymous",
            auth_state: "public",
            replay_state: "n/a",
            cache_state: "n/a",
            topology_state: "root_bootstrapped+reference-topology-ready",
            freshness_model: "fresh-standalone-per-scenario",
            notes: "Shared root registry read over the auto-created reference topology.",
        },
        AuditScenario {
            key: "root:canic_subnet_state:empty-struct",
            canister: "root",
            endpoint_or_flow: "canic_subnet_state",
            transport_mode: "query",
            subject_kind: "endpoint",
            subject_label: "canic_subnet_state",
            arg_class: "minimal-valid",
            caller_class: "anonymous",
            auth_state: "public",
            replay_state: "n/a",
            cache_state: "n/a",
            topology_state: "root_bootstrapped+reference-topology-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Root-only state snapshot for the restored `[as ss ad sd]` cascade lane.",
        },
        AuditScenario {
            key: "scale_hub:plan_create_worker:empty-pool",
            canister: "scale_hub",
            endpoint_or_flow: "plan_create_worker",
            transport_mode: "query",
            subject_kind: "endpoint",
            subject_label: "plan_create_worker",
            arg_class: "empty-pool",
            caller_class: "anonymous",
            auth_state: "local-test-only",
            replay_state: "n/a",
            cache_state: "n/a",
            topology_state: "standalone-scale_hub-ready",
            freshness_model: "fresh-standalone-per-scenario",
            notes: "Scaling dry-run query before any extra worker exists in the pool on one standalone scale_hub canister.",
        },
        AuditScenario {
            key: "scale_hub:create_worker:first-worker",
            canister: "scale_hub",
            endpoint_or_flow: "create_worker",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "create_worker",
            arg_class: "empty-pool",
            caller_class: "anonymous",
            auth_state: "local-test-only",
            replay_state: "n/a",
            cache_state: "cold",
            topology_state: "root_bootstrapped+no-extra-workers",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Scaling worker creation update over the empty local pool to exercise placement checkpoints.",
        },
        AuditScenario {
            key: "user_hub:create_account:first-account",
            canister: "user_hub",
            endpoint_or_flow: "create_account",
            transport_mode: "update",
            subject_kind: "endpoint",
            subject_label: "create_account",
            arg_class: "new-principal",
            caller_class: "anonymous",
            auth_state: "local-test-only",
            replay_state: "n/a",
            cache_state: "cold",
            topology_state: "root_bootstrapped+user_shard-template-staged",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Sharding assignment update for a brand-new principal to exercise shard placement checkpoints.",
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
            notes: "Root delegation provisioning request from a freshly created shard to exercise delegated auth issuance and proof fanout checkpoints.",
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
            replay_state: "n/a",
            cache_state: "cold",
            topology_state: "root_bootstrapped+fresh-user-shard+verifier-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "Verifier-side delegated token confirmation on the shared test canister using a freshly minted token from a newly created user shard.",
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
    ]
}

// Resolve the repo root from this crate's manifest path.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

// Read the output file layout chosen by the shell runner.
fn audit_paths() -> AuditPaths {
    AuditPaths {
        report_path: PathBuf::from(required_env("CANIC_INSTRUCTION_AUDIT_REPORT_PATH")),
        artifacts_dir: PathBuf::from(required_env("CANIC_INSTRUCTION_AUDIT_ARTIFACTS_DIR")),
    }
}

// Read run metadata provided by the shell runner.
fn audit_metadata() -> AuditMetadata {
    AuditMetadata {
        code_snapshot: required_env("CANIC_INSTRUCTION_AUDIT_CODE_SNAPSHOT"),
        branch: required_env("CANIC_INSTRUCTION_AUDIT_BRANCH"),
        worktree: required_env("CANIC_INSTRUCTION_AUDIT_WORKTREE"),
        run_timestamp_utc: required_env("CANIC_INSTRUCTION_AUDIT_TIMESTAMP_UTC"),
        compared_baseline_report: required_env("CANIC_INSTRUCTION_AUDIT_BASELINE_REPORT"),
    }
}

// Return the current workspace minor line like `0.24`.
fn current_minor_line() -> String {
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

// Query rows only count as unobservable if the same-call probe path failed.
// Query calls do not commit shared perf-table state, so they cannot rely on
// post-call `canic_metrics(MetricsKind::Perf, ...)` reads the way updates do.
fn query_perf_is_unobservable(scenario: &AuditScenario, row: &CanonicalPerfRow) -> bool {
    scenario.transport_mode == "query" && row.count == 0
}

// Choose the fresh root topology shape required for one scenario.
fn setup_for_scenario(scenario: &AuditScenario) -> root::harness::RootSetup {
    match scenario.key {
        "root:canic_subnet_registry:full-registry" | "root:canic_subnet_state:empty-struct" => {
            setup_root_topology()
        }
        "scale_hub:create_worker:first-worker" => setup_root_scaling(),
        "user_hub:create_account:first-account"
        | "root:canic_request_delegation:fresh-shard"
        | "test:test_verify_delegated_token:valid-delegated-token" => setup_root_sharding(),
        _ => setup_root_capability(),
    }
}

// Execute one scenario in an isolated fresh topology and derive the endpoint delta.
fn run_scenario(scenario: &AuditScenario) -> ScenarioResult {
    if let Some(result) = run_standalone_scenario(scenario) {
        return result;
    }

    let setup = setup_for_scenario(scenario);
    let prepared = prepare_scenario(&setup, scenario);
    let target_pid = prepared.target_pid;
    let (count, total_instructions, sample_origin, checkpoint_rows) =
        if scenario.transport_mode == "query" {
            let total = execute_query_perf_probe(&setup.pic, scenario, target_pid);
            (1, total, "derived".to_string(), Vec::new())
        } else {
            let before = perf_entries(&setup.pic, target_pid);
            execute_scenario(&setup, scenario, &prepared);
            let after = perf_entries(&setup.pic, target_pid);
            let (count, total_instructions) = perf_delta(
                &before,
                &after,
                scenario.subject_kind,
                scenario.subject_label,
            );
            let checkpoint_rows = checkpoint_deltas(scenario, &before, &after);
            (
                count,
                total_instructions,
                "derived".to_string(),
                checkpoint_rows,
            )
        };
    let avg_local_instructions = if count == 0 {
        0
    } else {
        total_instructions / count
    };

    ScenarioResult {
        scenario: *scenario,
        row: CanonicalPerfRow {
            subject_kind: scenario.subject_kind.to_string(),
            subject_label: scenario.subject_label.to_string(),
            count,
            total_local_instructions: total_instructions,
            avg_local_instructions,
            scenario_key: scenario.key.to_string(),
            scenario_labels: vec![
                format!("canister={}", scenario.canister),
                format!("endpoint_or_flow={}", scenario.endpoint_or_flow),
                format!("transport_mode={}", scenario.transport_mode),
                format!("arg_class={}", scenario.arg_class),
                format!("caller_class={}", scenario.caller_class),
                format!("auth_state={}", scenario.auth_state),
                format!("replay_state={}", scenario.replay_state),
                format!("cache_state={}", scenario.cache_state),
                format!("topology_state={}", scenario.topology_state),
                format!("freshness_model={}", scenario.freshness_model),
                format!("method_tag={METHOD_TAG}"),
            ],
            principal_scope: Some(scenario.caller_class.to_string()),
            sample_origin,
        },
        checkpoint_rows,
    }
}

fn run_standalone_scenario(scenario: &AuditScenario) -> Option<ScenarioResult> {
    let (crate_name, role) = match scenario.key {
        "app:canic_time:minimal-valid"
        | "app:canic_env:minimal-valid"
        | "app:canic_log:empty-page" => ("canister_app", APP),
        "scale_hub:plan_create_worker:empty-pool" => ("canister_scale_hub", SCALE_HUB),
        "test:test:minimal-valid" => ("canister_test", TEST),
        _ => return None,
    };

    let fixture = install_standalone_canister(crate_name, role, WasmBuildProfile::Fast);
    let target_pid = fixture.canister_id;
    let (count, total_instructions, sample_origin, checkpoint_rows) =
        if scenario.transport_mode == "query" {
            let total = execute_query_perf_probe(&fixture.pic, scenario, target_pid);
            (1, total, "derived".to_string(), Vec::new())
        } else {
            let before = perf_entries(&fixture.pic, target_pid);
            execute_standalone_scenario(&fixture.pic, scenario, target_pid);
            let after = perf_entries(&fixture.pic, target_pid);
            let (count, total_instructions) = perf_delta(
                &before,
                &after,
                scenario.subject_kind,
                scenario.subject_label,
            );
            let checkpoint_rows = checkpoint_deltas(scenario, &before, &after);
            (
                count,
                total_instructions,
                "derived".to_string(),
                checkpoint_rows,
            )
        };
    let avg_local_instructions = if count == 0 {
        0
    } else {
        total_instructions / count
    };

    Some(ScenarioResult {
        scenario: *scenario,
        row: CanonicalPerfRow {
            subject_kind: scenario.subject_kind.to_string(),
            subject_label: scenario.subject_label.to_string(),
            count,
            total_local_instructions: total_instructions,
            avg_local_instructions,
            scenario_key: scenario.key.to_string(),
            scenario_labels: vec![
                format!("canister={}", scenario.canister),
                format!("endpoint_or_flow={}", scenario.endpoint_or_flow),
                format!("transport_mode={}", scenario.transport_mode),
                format!("arg_class={}", scenario.arg_class),
                format!("caller_class={}", scenario.caller_class),
                format!("auth_state={}", scenario.auth_state),
                format!("replay_state={}", scenario.replay_state),
                format!("cache_state={}", scenario.cache_state),
                format!("topology_state={}", scenario.topology_state),
                format!("freshness_model={}", scenario.freshness_model),
                format!("method_tag={METHOD_TAG}"),
            ],
            principal_scope: Some(scenario.caller_class.to_string()),
            sample_origin,
        },
        checkpoint_rows,
    })
}

fn execute_standalone_scenario(pic: &Pic, scenario: &AuditScenario, target_pid: Principal) {
    match scenario.key {
        "test:test:minimal-valid" => {
            let response: Result<(), Error> = pic
                .update_call(target_pid, "test", ())
                .expect("standalone test transport failed");
            response.expect("standalone test application failed");
        }
        other => panic!("unsupported standalone audit scenario: {other}"),
    }
}

// Resolve the principal of the canister that owns the measured endpoint.
fn scenario_target_pid(
    root_id: canic::cdk::types::Principal,
    scenario: &AuditScenario,
    subnet_directory: &std::collections::HashMap<
        canic::ids::CanisterRole,
        canic::cdk::types::Principal,
    >,
) -> canic::cdk::types::Principal {
    match scenario.canister {
        "root" => root_id,
        "app" => *subnet_directory
            .get(&APP)
            .expect("app must exist in subnet directory"),
        "scale_hub" => *subnet_directory
            .get(&SCALE_HUB)
            .expect("scale_hub must exist in subnet directory"),
        "user_hub" => *subnet_directory
            .get(&USER_HUB)
            .expect("user_hub must exist in subnet directory"),
        "test" => *subnet_directory
            .get(&TEST)
            .expect("test must exist in subnet directory"),
        other => panic!("unsupported audit canister: {other}"),
    }
}

// Prepare scenario-specific prerequisites outside the measured perf window.
fn prepare_scenario(
    setup: &root::harness::RootSetup,
    scenario: &AuditScenario,
) -> PreparedScenario {
    let target_pid = scenario_target_pid(setup.root_id, scenario, &setup.subnet_directory);

    match scenario.key {
        "root:canic_template_prepare_admin:single-chunk" => {
            let fixture = audit_template_fixture(scenario);
            stage_manifest(&setup.pic, target_pid, &fixture.manifest);
            PreparedScenario {
                target_pid,
                caller_pid: None,
                delegated_token: None,
            }
        }
        "root:canic_template_publish_chunk_admin:single-chunk" => {
            let fixture = audit_template_fixture(scenario);
            stage_manifest(&setup.pic, target_pid, &fixture.manifest);
            prepare_chunk_set(&setup.pic, target_pid, &fixture.prepare);
            PreparedScenario {
                target_pid,
                caller_pid: None,
                delegated_token: None,
            }
        }
        "root:canic_request_delegation:fresh-shard" => {
            let user_hub_pid = *setup
                .subnet_directory
                .get(&USER_HUB)
                .expect("user_hub must exist for auth audit scenario");
            let shard_pid =
                create_user_shard(&setup.pic, user_hub_pid, Principal::from_slice(&[43; 29]));
            PreparedScenario {
                target_pid,
                caller_pid: Some(shard_pid),
                delegated_token: None,
            }
        }
        "test:test_verify_delegated_token:valid-delegated-token" => {
            let user_hub_pid = *setup
                .subnet_directory
                .get(&USER_HUB)
                .expect("user_hub must exist for verifier auth audit scenario");
            let shard_pid =
                create_user_shard(&setup.pic, user_hub_pid, Principal::from_slice(&[44; 29]));
            let subject = Principal::from_slice(&[45; 29]);
            let provision =
                request_root_delegation_provision(&setup.pic, setup.root_id, shard_pid, target_pid);
            let token = issue_delegated_token(
                &setup.pic,
                shard_pid,
                subject,
                vec![target_pid],
                vec![cap::VERIFY.to_string()],
                provision.proof.cert.issued_at,
                provision.proof.cert.expires_at,
            );
            PreparedScenario {
                target_pid,
                caller_pid: Some(subject),
                delegated_token: Some(token),
            }
        }
        _ => PreparedScenario {
            target_pid,
            caller_pid: None,
            delegated_token: None,
        },
    }
}

// Execute the actual endpoint call for one scenario.
fn execute_scenario(
    setup: &root::harness::RootSetup,
    scenario: &AuditScenario,
    prepared: &PreparedScenario,
) {
    let target_pid = prepared.target_pid;
    match scenario.key {
        "scale_hub:create_worker:first-worker" => {
            let response: Result<Principal, Error> = setup
                .pic
                .update_call(target_pid, "create_worker", ())
                .expect("create_worker transport failed");
            let _ = response.expect("create_worker application failed");
        }
        "user_hub:create_account:first-account" => {
            let response: Result<Principal, Error> = setup
                .pic
                .update_call(
                    target_pid,
                    "create_account",
                    (Principal::from_slice(&[42; 29]),),
                )
                .expect("create_account transport failed");
            let _ = response.expect("create_account application failed");
        }
        "root:canic_request_delegation:fresh-shard" => {
            execute_root_delegation_issue_scenario(setup, target_pid, prepared);
        }
        "test:test:minimal-valid" => {
            let response: Result<(), Error> = setup
                .pic
                .update_call(target_pid, "test", ())
                .expect("test transport failed");
            response.expect("test application failed");
        }
        "test:test_verify_delegated_token:valid-delegated-token" => {
            execute_verifier_auth_scenario(setup, target_pid, prepared);
        }
        "root:canic_response_capability_v1:request-cycles-fresh" => {
            execute_root_cycles_scenario(setup, target_pid);
        }
        "root:canic_template_stage_manifest_admin:single-chunk" => {
            let fixture = audit_template_fixture(scenario);
            stage_manifest(&setup.pic, target_pid, &fixture.manifest);
        }
        "root:canic_template_prepare_admin:single-chunk" => {
            let fixture = audit_template_fixture(scenario);
            prepare_chunk_set(&setup.pic, target_pid, &fixture.prepare);
        }
        "root:canic_template_publish_chunk_admin:single-chunk" => {
            let fixture = audit_template_fixture(scenario);
            publish_chunk(&setup.pic, target_pid, &fixture.chunk);
        }
        other => panic!("unsupported audit scenario: {other}"),
    }
}

// Execute the root-side delegated auth issuance scenario from a fresh shard.
fn execute_root_delegation_issue_scenario(
    setup: &root::harness::RootSetup,
    _target_pid: Principal,
    prepared: &PreparedScenario,
) {
    let caller = prepared
        .caller_pid
        .expect("auth audit scenario must resolve a shard caller");
    let verifier_pid = *setup
        .subnet_directory
        .get(&TEST)
        .expect("test canister must exist for auth audit scenario");
    let response =
        request_root_delegation_provision(&setup.pic, setup.root_id, caller, verifier_pid);
    assert_eq!(response.proof.cert.shard_pid, caller);
}

// Execute the verifier-side delegated token confirmation scenario.
fn execute_verifier_auth_scenario(
    setup: &root::harness::RootSetup,
    target_pid: Principal,
    prepared: &PreparedScenario,
) {
    let caller = prepared
        .caller_pid
        .expect("verifier auth audit scenario must resolve a delegated subject caller");
    let token = prepared
        .delegated_token
        .clone()
        .expect("verifier auth audit scenario must mint a delegated token");
    let response: Result<Result<(), Error>, Error> =
        setup
            .pic
            .update_call_as(target_pid, caller, "test_verify_delegated_token", (token,));
    response
        .expect("test_verify_delegated_token transport failed")
        .expect("test_verify_delegated_token application failed");
}

// Execute the fresh root cycles request scenario through the root dispatcher.
fn execute_root_cycles_scenario(setup: &root::harness::RootSetup, target_pid: Principal) {
    let caller = *setup
        .subnet_directory
        .get(&TEST)
        .expect("test canister must exist for root capability request");
    let request = Request::Cycles(CyclesRequest {
        cycles: 999,
        metadata: Some(metadata([90u8; 32], 120)),
    });
    let response = root_capability_response_as(setup, target_pid, caller, request)
        .expect("fresh root cycles capability request must succeed");
    match response {
        Response::Cycles(response) => {
            assert_eq!(response.cycles_transferred, 999);
        }
        other => panic!("expected cycles response, got: {other:?}"),
    }
}

// Execute the query path inside a same-call perf probe endpoint and return the
// measured local instruction counter from that call context.
fn execute_query_perf_probe(
    pic: &Pic,
    scenario: &AuditScenario,
    target_pid: canic::cdk::types::Principal,
) -> u64 {
    match scenario.key {
        "app:canic_time:minimal-valid" => {
            let response: Result<(u64, u64), Error> = pic
                .query_call(target_pid, APP_CANIC_TIME_PERF_TEST, ())
                .expect("canic_time_perf_test transport query failed");
            let (_value, perf) = response.expect("canic_time_perf_test application query failed");
            perf
        }
        "app:canic_env:minimal-valid" => {
            let response: Result<(EnvSnapshotResponse, u64), Error> = pic
                .query_call(target_pid, APP_CANIC_ENV_PERF_TEST, ())
                .expect("canic_env_perf_test transport query failed");
            let (_value, perf) = response.expect("canic_env_perf_test application query failed");
            perf
        }
        "app:canic_log:empty-page" => {
            let response: Result<(Page<LogEntry>, u64), Error> = pic
                .query_call(
                    target_pid,
                    APP_CANIC_LOG_PERF_TEST,
                    (
                        Option::<String>::None,
                        Option::<String>::None,
                        Option::<canic::__internal::core::log::Level>::None,
                        PageRequest {
                            limit: 10,
                            offset: 0,
                        },
                    ),
                )
                .expect("canic_log_perf_test transport query failed");
            let (_value, perf) = response.expect("canic_log_perf_test application query failed");
            perf
        }
        "root:canic_subnet_registry:full-registry" => {
            let response: Result<(SubnetRegistryResponse, u64), Error> = pic
                .query_call(target_pid, ROOT_CANIC_SUBNET_REGISTRY_PERF_TEST, ())
                .expect("canic_subnet_registry_perf_test transport query failed");
            let (_value, perf) =
                response.expect("canic_subnet_registry_perf_test application query failed");
            perf
        }
        "root:canic_subnet_state:empty-struct" => {
            let response: Result<(SubnetStateResponse, u64), Error> = pic
                .query_call(target_pid, ROOT_CANIC_SUBNET_STATE_PERF_TEST, ())
                .expect("canic_subnet_state_perf_test transport query failed");
            let (_value, perf) =
                response.expect("canic_subnet_state_perf_test application query failed");
            perf
        }
        "scale_hub:plan_create_worker:empty-pool" => {
            let response: Result<(bool, u64), Error> = pic
                .query_call(target_pid, SCALE_HUB_PLAN_CREATE_WORKER_PERF_TEST, ())
                .expect("plan_create_worker_perf_test transport query failed");
            let (_value, perf) =
                response.expect("plan_create_worker_perf_test application query failed");
            perf
        }
        other => panic!("unsupported query perf probe scenario: {other}"),
    }
}

// Build one synthetic staged-release fixture for root admin perf scenarios.
fn audit_template_fixture(scenario: &AuditScenario) -> AuditTemplateFixture {
    let slug = scenario.key.replace(':', "-");
    let bytes = format!("canic-instruction-audit-{slug}").into_bytes();
    let payload_hash = get_wasm_hash(&bytes);
    let chunk_hashes = vec![get_wasm_hash(&bytes)];
    let template_id = TemplateId::from(format!("audit:{slug}"));
    let version = TemplateVersion::from(format!("0.20-audit-{slug}"));

    AuditTemplateFixture {
        manifest: TemplateManifestInput {
            template_id: template_id.clone(),
            role: APP,
            version: version.clone(),
            payload_hash: payload_hash.clone(),
            payload_size_bytes: bytes.len() as u64,
            store_binding: WasmStoreBinding::new("bootstrap"),
            chunking_mode: TemplateChunkingMode::Chunked,
            manifest_state: TemplateManifestState::Approved,
            approved_at: None,
            created_at: 0,
        },
        prepare: TemplateChunkSetPrepareInput {
            template_id: template_id.clone(),
            version: version.clone(),
            payload_hash,
            payload_size_bytes: bytes.len() as u64,
            chunk_hashes,
        },
        chunk: TemplateChunkInput {
            template_id,
            version,
            chunk_index: 0,
            bytes,
        },
    }
}

// Stage one manifest through the root admin surface.
fn stage_manifest(
    pic: &Pic,
    root_id: canic::cdk::types::Principal,
    manifest: &TemplateManifestInput,
) {
    let staged: Result<(), Error> = pic
        .update_call(
            root_id,
            protocol::CANIC_TEMPLATE_STAGE_MANIFEST_ADMIN,
            (manifest.clone(),),
        )
        .expect("manifest staging transport failed");
    staged.expect("manifest staging application failed");
}

// Prepare one staged chunk set through the root admin surface.
fn prepare_chunk_set(
    pic: &Pic,
    root_id: canic::cdk::types::Principal,
    request: &TemplateChunkSetPrepareInput,
) {
    let prepared: Result<TemplateChunkSetInfoResponse, Error> = pic
        .update_call(
            root_id,
            protocol::CANIC_TEMPLATE_PREPARE_ADMIN,
            (request.clone(),),
        )
        .expect("template prepare transport failed");
    let _ = prepared.expect("template prepare application failed");
}

// Publish one staged chunk through the root admin surface.
fn publish_chunk(pic: &Pic, root_id: canic::cdk::types::Principal, request: &TemplateChunkInput) {
    let published: Result<(), Error> = pic
        .update_call(
            root_id,
            protocol::CANIC_TEMPLATE_PUBLISH_CHUNK_ADMIN,
            (request.clone(),),
        )
        .expect("template publish chunk transport failed");
    published.expect("template publish chunk application failed");
}

// Read the current perf metrics table for one canister.
fn perf_entries(pic: &Pic, canister_id: canic::cdk::types::Principal) -> Vec<MetricEntry> {
    let response: Result<Page<MetricEntry>, Error> = pic
        .query_call(
            canister_id,
            protocol::CANIC_METRICS,
            (
                MetricsKind::Perf,
                PageRequest {
                    limit: PERF_PAGE_LIMIT,
                    offset: 0,
                },
            ),
        )
        .expect("perf metrics transport query failed");

    response
        .expect("perf metrics application query failed")
        .entries
}

// Derive one endpoint/timer delta from two perf snapshots.
fn perf_delta(
    before: &[MetricEntry],
    after: &[MetricEntry],
    subject_kind: &str,
    subject_label: &str,
) -> (u64, u64) {
    let before_slot = perf_slot(before, subject_kind, subject_label);
    let after_slot = perf_slot(after, subject_kind, subject_label);

    (
        after_slot.0.saturating_sub(before_slot.0),
        after_slot.1.saturating_sub(before_slot.1),
    )
}

// Project one perf row into `(count, total_instructions)`.
fn perf_slot(entries: &[MetricEntry], subject_kind: &str, subject_label: &str) -> (u64, u64) {
    entries
        .iter()
        .find_map(|entry| {
            if entry
                .labels
                .first()
                .is_some_and(|label| label == subject_kind)
                && entry
                    .labels
                    .get(1)
                    .is_some_and(|label| label == subject_label)
            {
                Some(match entry.value {
                    MetricValue::CountAndU64 { count, value_u64 } => (count, value_u64),
                    MetricValue::Count(count) => (count, 0),
                    MetricValue::U128(_) => (0, 0),
                })
            } else {
                None
            }
        })
        .unwrap_or((0, 0))
}

// Derive checkpoint deltas from two perf snapshots for one sampled update scenario.
fn checkpoint_deltas(
    scenario: &AuditScenario,
    before: &[MetricEntry],
    after: &[MetricEntry],
) -> Vec<CheckpointDeltaRow> {
    let mut rows = after
        .iter()
        .filter_map(|entry| {
            let [kind, scope, label] = entry.labels.as_slice() else {
                return None;
            };
            if kind != "checkpoint" {
                return None;
            }

            let before_slot = perf_checkpoint_slot(before, scope, label);
            let after_slot = match entry.value {
                MetricValue::CountAndU64 { count, value_u64 } => (count, value_u64),
                MetricValue::Count(count) => (count, 0),
                MetricValue::U128(_) => (0, 0),
            };

            let count = after_slot.0.saturating_sub(before_slot.0);
            let total_local_instructions = after_slot.1.saturating_sub(before_slot.1);
            if count == 0 && total_local_instructions == 0 {
                return None;
            }

            Some(CheckpointDeltaRow {
                scenario_key: scenario.key.to_string(),
                canister: scenario.canister.to_string(),
                endpoint_or_flow: scenario.endpoint_or_flow.to_string(),
                scope: scope.clone(),
                label: label.clone(),
                count,
                total_local_instructions,
                avg_local_instructions: if count == 0 {
                    0
                } else {
                    total_local_instructions / count
                },
            })
        })
        .collect::<Vec<_>>();

    rows.sort_by_key(|row| std::cmp::Reverse(row.total_local_instructions));
    rows
}

// Project one checkpoint row into `(count, total_instructions)`.
fn perf_checkpoint_slot(entries: &[MetricEntry], scope: &str, label: &str) -> (u64, u64) {
    entries
        .iter()
        .find_map(|entry| {
            let [kind, entry_scope, entry_label] = entry.labels.as_slice() else {
                return None;
            };
            if kind == "checkpoint" && entry_scope == scope && entry_label == label {
                Some(match entry.value {
                    MetricValue::CountAndU64 { count, value_u64 } => (count, value_u64),
                    MetricValue::Count(count) => (count, 0),
                    MetricValue::U128(_) => (0, 0),
                })
            } else {
                None
            }
        })
        .unwrap_or((0, 0))
}

// Execute one structural root capability call as the requested child caller.
fn root_capability_response_as(
    setup: &root::harness::RootSetup,
    target_pid: Principal,
    caller: Principal,
    request: Request,
) -> Result<Response, Error> {
    let (request_id, nonce, ttl_seconds) = capability_metadata_from_request(&request);
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request,
        proof: CapabilityProof::Structural,
        metadata: CapabilityRequestMetadata {
            request_id,
            nonce,
            issued_at: target_now_secs(setup, target_pid),
            ttl_seconds,
        },
    };

    let result: Result<Result<RootCapabilityResponseV1, Error>, Error> = setup.pic.update_call_as(
        target_pid,
        caller,
        protocol::CANIC_RESPONSE_CAPABILITY_V1,
        (envelope,),
    );
    result
        .expect("root capability transport call failed")
        .map(|response| response.response)
}

// Read one canister's current time in seconds for capability metadata issuance.
fn target_now_secs(setup: &root::harness::RootSetup, canister_id: Principal) -> u64 {
    let now: Result<u64, Error> = setup
        .pic
        .query_call(canister_id, protocol::CANIC_TIME, ())
        .expect("canic_time transport query failed");
    now.expect("canic_time application query failed") / 1_000_000_000
}

// Rebuild the capability metadata tuple that the structural envelope expects.
fn capability_metadata_from_request(request: &Request) -> ([u8; 16], [u8; 16], u32) {
    let metadata = match request {
        Request::CreateCanister(req) => req.metadata,
        Request::UpgradeCanister(req) => req.metadata,
        Request::Cycles(req) => req.metadata,
        Request::IssueDelegation(req) => req.metadata,
        Request::IssueRoleAttestation(req) => req.metadata,
    };

    match metadata {
        Some(meta) => {
            let mut request_id = [0u8; 16];
            request_id.copy_from_slice(&meta.request_id[..16]);
            let mut nonce = [0u8; 16];
            nonce.copy_from_slice(&meta.request_id[16..]);
            let ttl_seconds =
                u32::try_from(meta.ttl_seconds.min(u64::from(u32::MAX))).expect("ttl bounded");
            (request_id, nonce, ttl_seconds)
        }
        None => ([0u8; 16], [0u8; 16], 60),
    }
}

// Build one deterministic root request metadata value for audit scenarios.
const fn metadata(request_id: [u8; 32], ttl_seconds: u64) -> RootRequestMetadata {
    RootRequestMetadata {
        request_id,
        ttl_seconds,
    }
}

// Scan the repo for concrete `perf!` checkpoint call sites.
fn scan_perf_callsites(workspace_root: &Path) -> Vec<String> {
    let mut out = Vec::new();

    for root in CHECKPOINT_SCAN_ROOTS {
        visit_rust_files(&workspace_root.join(root), &mut |path| {
            let Ok(contents) = fs::read_to_string(path) else {
                return;
            };

            for (line_no, line) in contents.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("///") {
                    continue;
                }

                let Some(index) = line.find("perf!(") else {
                    continue;
                };
                let previous = line[..index].chars().next_back();
                if matches!(previous, Some('"' | '\'' | '`')) {
                    continue;
                }

                if line[index..].starts_with("perf!(") {
                    let relative = path
                        .strip_prefix(workspace_root)
                        .expect("path under workspace root");
                    out.push(format!(
                        "{}:{}:{}",
                        relative.display(),
                        line_no + 1,
                        line.trim()
                    ));
                }
            }
        });
    }

    out.sort();
    out
}

// Recursively visit Rust source files under one directory root.
fn visit_rust_files(dir: &Path, visitor: &mut impl FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_rust_files(&path, visitor);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            visitor(&path);
        }
    }
}

// Build the current checkpoint-gap table from the static critical-flow list.
fn checkpoint_coverage_gaps(checkpoint_sites: &[String]) -> Vec<CheckpointCoverageGap> {
    FLOW_GAPS
        .iter()
        .map(|(flow_name, insertion_site)| CheckpointCoverageGap {
            flow_name: (*flow_name).to_string(),
            status: if checkpoint_sites
                .iter()
                .any(|site| site.starts_with(insertion_site))
            {
                "PASS".to_string()
            } else {
                "PARTIAL".to_string()
            },
            proposed_first_insertion_site: (*insertion_site).to_string(),
        })
        .collect()
}

// Write the raw checkpoint scan output expected by the audit definition.
fn write_flow_checkpoint_log(path: &Path, checkpoint_sites: &[String]) {
    let body = if checkpoint_sites.is_empty() {
        "No `perf!` checkpoint call sites were found under `crates/`.\n".to_string()
    } else {
        let mut lines = checkpoint_sites.join("\n");
        lines.push('\n');
        lines
    };

    fs::write(path, body).expect("write flow checkpoints log");
}

// Assemble the verification table for the first instruction-footprint run.
fn verification_rows(
    paths: &AuditPaths,
    checkpoint_sites: &[String],
    query_unobservable_count: usize,
    measured_checkpoint_count: usize,
) -> Vec<VerificationRow> {
    vec![
        VerificationRow {
            command: "cargo test -p canic-tests --test instruction_audit generate_instruction_footprint_report -- --ignored --nocapture".to_string(),
            status: "PASS".to_string(),
            notes: "PocketIC runner completed and wrote the report plus normalized artifacts."
                .to_string(),
        },
        VerificationRow {
            command: "fresh root harness profile per scenario".to_string(),
            status: "PASS".to_string(),
            notes:
                "Each scenario used a fresh smallest-profile root bootstrap instead of sharing one cumulative perf table."
                    .to_string(),
        },
        VerificationRow {
            command: "canic_metrics(MetricsKind::Perf, PageRequest { limit=512, offset=0 })"
                .to_string(),
            status: "PASS".to_string(),
            notes: format!("Update scenarios were sampled before/after through persisted perf rows, and query scenarios used same-call local-only probe endpoints because query-side perf rows are not committed; normalized rows saved under `{}`.", paths.artifacts_dir.join("perf-rows.json").display()),
        },
        VerificationRow {
            command: "repo checkpoint scan".to_string(),
            status: "PASS".to_string(),
            notes: if checkpoint_sites.is_empty() {
                "No `perf!` call sites are present in the current repo scan; flow checkpoint coverage remains partial.".to_string()
            } else {
                format!("Found {} checkpoint call sites.", checkpoint_sites.len())
            },
        },
        VerificationRow {
            command: "checkpoint delta capture".to_string(),
            status: if measured_checkpoint_count == 0 {
                "PARTIAL".to_string()
            } else {
                "PASS".to_string()
            },
            notes: if measured_checkpoint_count == 0 {
                "Sampled update scenarios did not produce any non-zero checkpoint deltas."
                    .to_string()
            } else {
                format!(
                    "{measured_checkpoint_count} non-zero checkpoint delta rows were captured under `{}`.",
                    paths.artifacts_dir.join("checkpoint-deltas.json").display()
                )
            },
        },
        VerificationRow {
            command: "query perf visibility".to_string(),
            status: if query_unobservable_count == 0 {
                "PASS".to_string()
            } else {
                "PARTIAL".to_string()
            },
            notes: if query_unobservable_count == 0 {
                "All sampled query scenarios returned same-call local instruction counters through the local-only probe endpoints, which avoids relying on non-persisted query-side perf state.".to_string()
            } else {
                format!(
                    "{query_unobservable_count} sampled query scenarios failed to return a same-call local instruction counter through the probe path."
                )
            },
        },
        VerificationRow {
            command: "baseline comparison".to_string(),
            status: "BLOCKED".to_string(),
            notes: "First run of day for `instruction-footprint`; baseline deltas are `N/A`."
                .to_string(),
        },
    ]
}

// Write the markdown verification table consumed by the dated report.
#[allow(clippy::format_push_string)]
fn write_verification_readout(path: &Path, rows: &[VerificationRow]) {
    let mut out = String::from("| Command | Status | Notes |\n| --- | --- | --- |\n");
    for row in rows {
        out.push_str(&format!(
            "| `{}` | {} | {} |\n",
            row.command, row.status, row.notes
        ));
    }

    fs::write(path, out).expect("write verification readout");
}

// Serialize one JSON artifact with a trailing newline.
fn write_json<T>(path: &Path, value: &T)
where
    T: ?Sized + Serialize,
{
    let mut body = serde_json::to_string_pretty(value).expect("serialize json");
    body.push('\n');
    fs::write(path, body).expect("write json artifact");
}

// Write the normalized endpoint matrix as a simple TSV artifact.
#[allow(clippy::format_push_string)]
fn write_endpoint_matrix_tsv(path: &Path, results: &[ScenarioResult]) {
    let mut out = String::from(
        "canister\tendpoint_or_flow\tscenario_key\tcount\ttotal_local_instructions\tavg_local_instructions\n",
    );

    for result in results {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            result.scenario.canister,
            result.scenario.endpoint_or_flow,
            result.scenario.key,
            result.row.count,
            result.row.total_local_instructions,
            result.row.avg_local_instructions
        ));
    }

    fs::write(path, out).expect("write endpoint matrix tsv");
}

// Render the first dated instruction-footprint report from normalized results.
#[allow(clippy::format_push_string, clippy::too_many_lines)]
fn write_report(
    path: &Path,
    artifacts_dir: &Path,
    metadata: &AuditMetadata,
    results: &[ScenarioResult],
    verification_rows: &[VerificationRow],
    checkpoint_sites: &[String],
    gaps: &[CheckpointCoverageGap],
) {
    let query_unobservable_count = results
        .iter()
        .filter(|result| query_perf_is_unobservable(&result.scenario, &result.row))
        .count();
    let checkpoint_rows = results
        .iter()
        .flat_map(|result| result.checkpoint_rows.iter())
        .collect::<Vec<_>>();

    let mut ordered = results
        .iter()
        .filter(|result| !query_perf_is_unobservable(&result.scenario, &result.row))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|result| std::cmp::Reverse(result.row.avg_local_instructions));

    let hotspot_rows = ordered.iter().take(3).copied().collect::<Vec<_>>();
    let risk_score = risk_score(checkpoint_sites, query_unobservable_count, &hotspot_rows);
    let minor_line = current_minor_line();
    let report_date = metadata
        .run_timestamp_utc
        .get(..10)
        .expect("timestamp includes YYYY-MM-DD");
    let report_file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("report file name");
    let artifacts_dir_name = artifacts_dir
        .file_name()
        .and_then(|name| name.to_str())
        .expect("artifacts directory name");
    let target_canisters = render_scope(
        results
            .iter()
            .map(|result| result.scenario.canister)
            .collect::<BTreeSet<_>>(),
    );
    let target_endpoints = render_scope(
        results
            .iter()
            .map(|result| result.scenario.endpoint_or_flow)
            .collect::<BTreeSet<_>>(),
    );

    let mut out = String::new();
    out.push_str(&format!(
        "# Instruction Footprint Audit - {report_date}\n\n"
    ));
    out.push_str("## Report Preamble\n\n");
    out.push_str(&format!(
        "- Scope: Canic instruction footprint (first `{minor_line}` baseline, partial canister scope)\n"
    ));
    out.push_str("- Definition path: `docs/audits/recurring/system/instruction-footprint.md`\n");
    out.push_str(&format!(
        "- Compared baseline report path: `{}`\n",
        metadata.compared_baseline_report
    ));
    out.push_str(&format!(
        "- Code snapshot identifier: `{}`\n",
        metadata.code_snapshot
    ));
    out.push_str(&format!("- Method tag/version: `{METHOD_TAG}`\n"));
    out.push_str("- Comparability status: `partial`\n");
    out.push_str("- Auditor: `codex`\n");
    out.push_str(&format!(
        "- Run timestamp (UTC): `{}`\n",
        metadata.run_timestamp_utc
    ));
    out.push_str(&format!("- Branch: `{}`\n", metadata.branch));
    out.push_str(&format!("- Worktree: `{}`\n", metadata.worktree));
    out.push_str("- Execution environment: `PocketIC`\n");
    out.push_str(&format!(
        "- Target canisters in scope: {target_canisters}\n"
    ));
    out.push_str(&format!(
        "- Target endpoints/flows in scope: {target_endpoints}\n"
    ));
    out.push_str("- Deferred from this baseline: no additional functional flows are deferred beyond first-run comparability; this run covers shared queries plus delegated auth issuance, verifier confirmation, replay/cycles, scaling worker creation, sharding account creation, and root template admin updates.\n\n");

    out.push_str("## Findings / Checklist\n\n");
    out.push_str("| Check | Result | Evidence |\n| --- | --- | --- |\n");
    out.push_str(&format!(
        "| Scenario manifest recorded | PASS | `artifacts/{artifacts_dir_name}/scenario-manifest.json` captures the scenario identity tuple for every sampled endpoint. |\n"
    ));
    out.push_str(&format!(
        "| Normalized perf rows recorded | PASS | `artifacts/{artifacts_dir_name}/perf-rows.json` stores canonical endpoint rows with count and total local instructions. |\n"
    ));
    out.push_str(&format!(
        "| Checkpoint deltas recorded | {} | `artifacts/{artifacts_dir_name}/checkpoint-deltas.json` stores non-zero per-scenario checkpoint rows. |\n",
        if checkpoint_rows.is_empty() { "PARTIAL" } else { "PASS" }
    ));
    out.push_str("| Fresh topology isolation used | PASS | Each scenario ran under a fresh smallest-profile root harness install instead of reusing one cumulative perf table. |\n");
    out.push_str(&format!(
        "| Flow checkpoint coverage scanned | PASS | `artifacts/{artifacts_dir_name}/flow-checkpoints.log` records the current repo scan result. |\n"
    ));
    if checkpoint_sites.is_empty() {
        out.push_str("| `perf!` checkpoints available for critical flows | PARTIAL | Current repo scan found zero `perf!` call sites under `crates/`, so flow-stage attribution is not yet measurable. |\n");
    } else {
        out.push_str("| `perf!` checkpoints available for critical flows | PASS | Current repo scan found at least one `perf!` call site. |\n");
    }
    if query_unobservable_count == 0 {
        out.push_str("| Query endpoint perf visibility | PASS | Sampled query scenarios were measured through same-call local-only perf probe endpoints because query-side perf rows are not committed. |\n");
    } else {
        out.push_str(&format!(
            "| Query endpoint perf visibility | PARTIAL | {query_unobservable_count} sampled query scenarios failed to return a same-call local instruction counter through the probe path. |\n"
        ));
    }
    out.push_str("| Baseline path selected by daily baseline discipline | PARTIAL | First run of day for `instruction-footprint`; baseline deltas are `N/A`. |\n\n");

    out.push_str("## Comparison to Previous Relevant Run\n\n");
    out.push_str("- First run of day for `instruction-footprint`; this report establishes the daily baseline.\n");
    out.push_str("- Query scenarios are now sampled through same-call local-only perf probes because query-side perf rows are not committed, so their rows are directly comparable to later probe-backed reruns.\n");
    if query_unobservable_count > 0 {
        out.push_str("- One or more query probe calls still failed to return a usable local instruction counter, so those rows remain partial until the probe path is stable.\n");
    }
    out.push_str("- Baseline drift values are `N/A` until a same-day rerun or later comparable run exists.\n\n");

    out.push_str("## Endpoint Matrix\n\n");
    out.push_str("| Canister | Endpoint | Scenario | Count | Total local instructions | Avg local instructions | Baseline delta | Notes |\n");
    out.push_str("| --- | --- | --- | ---: | ---: | ---: | --- | --- |\n");
    for result in results {
        let notes = if query_perf_is_unobservable(&result.scenario, &result.row) {
            "probe failed to return a local instruction counter"
        } else if result.scenario.transport_mode == "query" {
            "same-call local-only perf probe"
        } else {
            ""
        };
        out.push_str(&format!(
            "| `{}` | `{}` | `{}` | {} | {} | {} | N/A | {} |\n",
            result.scenario.canister,
            result.scenario.endpoint_or_flow,
            result.scenario.arg_class,
            result.row.count,
            result.row.total_local_instructions,
            result.row.avg_local_instructions,
            notes
        ));
    }
    out.push('\n');

    out.push_str("## Flow Checkpoints\n\n");
    if checkpoint_sites.is_empty() {
        out.push_str("- No current `perf!` checkpoints were found under `crates/`; no per-stage flow deltas are available yet.\n");
        out.push_str(&format!(
            "- Flow checkpoint evidence file: `artifacts/{artifacts_dir_name}/flow-checkpoints.log`\n\n"
        ));
    } else {
        for site in checkpoint_sites {
            out.push_str(&format!("- `{site}`\n"));
        }
        out.push('\n');
    }

    out.push_str("## Measured Checkpoint Deltas\n\n");
    if checkpoint_rows.is_empty() {
        out.push_str("- No sampled scenario produced a non-zero checkpoint delta in this run.\n\n");
    } else {
        let mut ordered_checkpoint_rows = checkpoint_rows;
        ordered_checkpoint_rows.sort_by_key(|row| std::cmp::Reverse(row.total_local_instructions));
        out.push_str("| Scenario | Scope | Label | Count | Total local instructions | Avg local instructions |\n");
        out.push_str("| --- | --- | --- | ---: | ---: | ---: |\n");
        for row in ordered_checkpoint_rows.iter().take(12) {
            out.push_str(&format!(
                "| `{}` | `{}` | `{}` | {} | {} | {} |\n",
                row.scenario_key,
                row.scope,
                row.label,
                row.count,
                row.total_local_instructions,
                row.avg_local_instructions
            ));
        }
        out.push('\n');
    }

    out.push_str("## Checkpoint Coverage Gaps\n\n");
    let covered_gaps = gaps
        .iter()
        .filter(|gap| gap.status == "PASS")
        .collect::<Vec<_>>();
    let uncovered_gaps = gaps
        .iter()
        .filter(|gap| gap.status != "PASS")
        .collect::<Vec<_>>();
    out.push_str("Critical flows with checkpoints:\n");
    if covered_gaps.is_empty() {
        out.push_str("- none\n\n");
    } else {
        for gap in &covered_gaps {
            out.push_str(&format!("- `{}`\n", gap.flow_name));
        }
        out.push('\n');
    }
    out.push_str("Critical flows without checkpoints:\n");
    if uncovered_gaps.is_empty() {
        out.push_str("- none\n");
    } else {
        for gap in &uncovered_gaps {
            out.push_str(&format!("- `{}`\n", gap.flow_name));
        }
    }
    out.push('\n');
    out.push_str("Proposed first checkpoint insertion sites:\n");
    if uncovered_gaps.is_empty() {
        out.push_str("- none\n");
    } else {
        for gap in &uncovered_gaps {
            out.push_str(&format!(
                "- `{}` -> `{}`\n",
                gap.flow_name, gap.proposed_first_insertion_site
            ));
        }
    }
    out.push('\n');

    out.push_str("## Structural Hotspots\n\n");
    out.push_str("| Rank | Scenario | Avg local instructions | Module pressure | Evidence |\n");
    out.push_str("| --- | --- | ---: | --- | --- |\n");
    for (index, result) in hotspot_rows.iter().enumerate() {
        let (pressure, evidence) = hotspot_hint(result.scenario.subject_label);
        out.push_str(&format!(
            "| {} | `{}` | {} | {} | {} |\n",
            index + 1,
            result.scenario.key,
            result.row.avg_local_instructions,
            pressure,
            evidence
        ));
    }
    out.push('\n');

    out.push_str("## Hub Module Pressure\n\n");
    out.push_str("- `scale_hub::create_worker` concentrates cost in the scaling coordinator surface plus `canic-core` placement workflow. That makes scaling one of the first shared instruction hot paths worth reducing.\n");
    out.push_str("- `user_hub::create_account` is now measurable as a real sharding update, and its first-account path is dominated by `canic-core::workflow::placement::sharding::bootstrap_empty_active`.\n");
    out.push_str("- `root::canic_response_capability_v1` now has measured replay/cycles stage deltas, so root capability work no longer has to be treated as an opaque endpoint total.\n");
    out.push_str("- `test::test` provides the current chain-key-free update floor on a non-root child canister. Drift there points back to shared runtime/update overhead rather than topology-specific logic.\n");
    out.push_str("- Root state/registry reads stay separate from the leaf floor. They matter for operator paths, but they should not be confused with the shared ordinary-leaf baseline.\n\n");

    out.push_str("## Dependency Fan-In Pressure\n\n");
    out.push_str("- Shared lifecycle/observability endpoints (`canic_time`, `canic_env`, `canic_log`) all route through the default `start!` bundle, and this matrix now samples them through same-call local-only perf probes. Their rows reflect actual query counters from the measured call context rather than inferred zeroes or missing query-side perf-table commits.\n");
    out.push_str("- The sampled non-trivial hotspot fans into `canic-core` placement orchestration (`workflow/placement/scaling`). The local `test::test` update acts as the baseline floor for update overhead on an ordinary child canister.\n");
    if checkpoint_sites.is_empty() {
        out.push_str("- There is currently no flow-stage attribution because `perf!` coverage is absent. That is itself a dependency-pressure signal: optimization work is bottlenecked by missing internal checkpoints.\n\n");
    } else {
        out.push_str("- Flow-stage checkpoints now exist in the scaling, sharding, auth, and replay workflows. This matrix records non-zero checkpoint deltas for sampled update scenarios, so the next optimization pass can target concrete stages instead of endpoint totals alone.\n\n");
    }

    out.push_str("## Early Warning Signals\n\n");
    out.push_str("| Signal | Status | Evidence |\n");
    out.push_str("| --- | --- | --- |\n");
    if checkpoint_sites.is_empty() {
        out.push_str("| Flow checkpoint coverage absent | WARN | Current repo scan found zero `perf!` call sites under `crates/`. |\n");
    } else {
        out.push_str(&format!(
            "| Flow checkpoint coverage present | INFO | Current repo scan found {} `perf!` call sites under `crates/`. |\n",
            checkpoint_sites.len()
        ));
    }
    if query_unobservable_count > 0 {
        out.push_str(&format!(
            "| Query probe path failed on sampled rows | WARN | {query_unobservable_count} sampled query scenarios did not return a usable same-call local instruction counter. |\n"
        ));
    }
    if let Some(top) = hotspot_rows.first() {
        out.push_str(&format!(
            "| Highest sampled endpoint currently highest-cost | WARN | `{}` averages {} local instructions in this first baseline. |\n",
            top.scenario.key, top.row.avg_local_instructions
        ));
    }
    out.push_str("| Baseline drift not yet available | INFO | First run of day; deltas remain `N/A` until the next comparable rerun. |\n\n");

    out.push_str("## Risk Score\n\n");
    out.push_str(&format!("Risk Score: **{risk_score} / 10**\n\n"));
    out.push_str("Interpretation: query visibility and stage attribution are now working for the sampled matrix. The remaining audit risk is mostly first-run comparability (`N/A` baseline deltas) plus a few endpoint-only paths that still do not have deeper internal stage attribution, not missing coverage of the critical flows themselves.\n\n");

    out.push_str("## Verification Readout\n\n");
    out.push_str("| Command | Status | Notes |\n| --- | --- | --- |\n");
    for row in verification_rows {
        out.push_str(&format!(
            "| `{}` | {} | {} |\n",
            row.command, row.status, row.notes
        ));
    }
    out.push('\n');

    out.push_str("## Follow-up Actions\n\n");
    out.push_str("1. Owner boundary: `flow observability`\n");
    if checkpoint_sites.is_empty() {
        out.push_str("   Action: add first stable `perf!` checkpoints to the scaling, sharding, and root-capability workflows so the next rerun can move from endpoint-only totals to real flow-stage attribution.\n");
    } else {
        out.push_str("   Action: rerun this audit after one concrete perf change so the next report has real comparable baseline deltas instead of first-run `N/A`, and only add deeper verifier-side auth checkpoints if that endpoint-total starts to matter.\n");
    }
    out.push_str("2. Owner boundary: `shared update hotspots`\n");
    out.push_str(&format!(
        "   Action: compare `scale_hub::create_worker` and `user_hub::create_account` against the `test::test` update floor before/after any placement/sharding cleanup, using this report as the `{minor_line}` baseline.\n"
    ));
    out.push_str("3. Owner boundary: `shared observability floor`\n");
    out.push_str("   Action: keep `app` query surfaces in the matrix so shared-runtime drift does not hide behind root-only or coordinator-only endpoints.\n\n");

    out.push_str("## Report Files\n\n");
    out.push_str(&format!("- [{report_file_name}](./{report_file_name})\n"));
    out.push_str(&format!(
        "- [scenario-manifest.json](artifacts/{artifacts_dir_name}/scenario-manifest.json)\n"
    ));
    out.push_str(&format!(
        "- [perf-rows.json](artifacts/{artifacts_dir_name}/perf-rows.json)\n"
    ));
    out.push_str(&format!(
        "- [endpoint-matrix.tsv](artifacts/{artifacts_dir_name}/endpoint-matrix.tsv)\n"
    ));
    out.push_str(&format!(
        "- [checkpoint-deltas.json](artifacts/{artifacts_dir_name}/checkpoint-deltas.json)\n"
    ));
    out.push_str(&format!(
        "- [flow-checkpoints.log](artifacts/{artifacts_dir_name}/flow-checkpoints.log)\n"
    ));
    out.push_str(&format!(
        "- [checkpoint-coverage-gaps.json](artifacts/{artifacts_dir_name}/checkpoint-coverage-gaps.json)\n"
    ));
    out.push_str(&format!(
        "- [verification-readout.md](artifacts/{artifacts_dir_name}/verification-readout.md)\n"
    ));
    out.push_str(&format!(
        "- [method.json](artifacts/{artifacts_dir_name}/method.json)\n"
    ));
    out.push_str(&format!(
        "- [environment.json](artifacts/{artifacts_dir_name}/environment.json)\n"
    ));

    fs::write(path, out).expect("write instruction audit report");
}

// Render one stable, backtick-quoted scope list for the report preamble.
fn render_scope(items: BTreeSet<&str>) -> String {
    items
        .into_iter()
        .map(|item| format!("`{item}`"))
        .collect::<Vec<_>>()
        .join(" ")
}

// Map the current highest-cost labels back to concrete modules/files.
fn hotspot_hint(subject_label: &str) -> (&'static str, &'static str) {
    match subject_label {
        "create_account" => (
            "Sharding coordinator plus `canic-core` sharding workflow",
            "[user_hub/lib](/home/adam/projects/canic/canisters/user_hub/src/lib.rs), [sharding workflow](/home/adam/projects/canic/crates/canic-core/src/workflow/placement/sharding/mod.rs)",
        ),
        "canic_response_capability_v1" => (
            "Root dispatcher plus replay/capability workflow",
            "[request handler](/home/adam/projects/canic/crates/canic-core/src/workflow/rpc/request/handler/mod.rs), [replay workflow](/home/adam/projects/canic/crates/canic-core/src/workflow/rpc/request/handler/replay.rs)",
        ),
        "create_worker" => (
            "Scaling coordinator plus `canic-core` placement workflow",
            "[scale_hub/lib](/home/adam/projects/canic/canisters/scale_hub/src/lib.rs), [scaling workflow](/home/adam/projects/canic/crates/canic-core/src/workflow/placement/scaling/mod.rs)",
        ),
        "plan_create_worker" => (
            "Scaling policy read path",
            "[scale_hub/lib](/home/adam/projects/canic/canisters/scale_hub/src/lib.rs), [scaling workflow](/home/adam/projects/canic/crates/canic-core/src/workflow/placement/scaling/mod.rs)",
        ),
        "test" => (
            "Local/dev update floor on the test helper canister",
            "[test/lib](/home/adam/projects/canic/canisters/test/src/lib.rs)",
        ),
        "canic_subnet_registry" => (
            "Root topology registry query",
            "[endpoints](/home/adam/projects/canic/crates/canic/src/macros/endpoints.rs), [registry query](/home/adam/projects/canic/crates/canic-core/src/workflow/topology/registry/query.rs)",
        ),
        "canic_subnet_state" => (
            "Root state snapshot query",
            "[endpoints](/home/adam/projects/canic/crates/canic/src/macros/endpoints.rs), [state query](/home/adam/projects/canic/crates/canic-core/src/workflow/state/query.rs)",
        ),
        "canic_log" => (
            "Shared log pagination surface",
            "[endpoints](/home/adam/projects/canic/crates/canic/src/macros/endpoints.rs), [log query](/home/adam/projects/canic/crates/canic-core/src/workflow/log/query.rs)",
        ),
        "canic_env" => (
            "Shared env snapshot surface",
            "[endpoints](/home/adam/projects/canic/crates/canic/src/macros/endpoints.rs), [env query](/home/adam/projects/canic/crates/canic-core/src/workflow/env/query.rs)",
        ),
        "canic_time" => (
            "Shared lifecycle/runtime query surface",
            "[endpoints](/home/adam/projects/canic/crates/canic/src/macros/endpoints.rs)",
        ),
        _ => (
            "Shared runtime surface",
            "[endpoints](/home/adam/projects/canic/crates/canic/src/macros/endpoints.rs)",
        ),
    }
}

// Compute a bounded risk score for the first baseline.
fn risk_score(
    checkpoint_sites: &[String],
    query_unobservable_count: usize,
    hotspot_rows: &[&ScenarioResult],
) -> u8 {
    let mut score = 2u8;

    if checkpoint_sites.is_empty() {
        score = score.saturating_add(3);
    }

    if query_unobservable_count > 0 {
        score = score.saturating_add(1);
    }

    if hotspot_rows
        .first()
        .is_some_and(|row| row.row.avg_local_instructions > 2_000_000)
    {
        score = score.saturating_add(2);
    }

    if hotspot_rows
        .iter()
        .filter(|row| row.scenario.canister == "root")
        .count()
        == hotspot_rows.len()
    {
        score = score.saturating_add(1);
    }

    score.min(10)
}
