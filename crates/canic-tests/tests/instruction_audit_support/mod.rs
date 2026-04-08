use crate::root_harness::{self, RootSetupProfile, setup_root};
use canic::{
    Error,
    api::ic::network::NetworkApi,
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
    ids::{BuildNetwork, cap},
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
    create_user_shard, install_audit_leaf_probe, install_audit_root_probe,
    install_audit_scaling_probe, install_standalone_canister, issue_delegated_token,
    request_root_delegation_provision,
};
use canic_testkit::{artifacts::WasmBuildProfile, pic::Pic};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    convert::TryFrom,
    env, fs,
    path::{Path, PathBuf},
};

mod execution;
mod report;
mod scenarios;

use execution::run_scenario;
use report::{
    checkpoint_coverage_gaps, scan_perf_callsites, verification_rows, write_endpoint_matrix_tsv,
    write_flow_checkpoint_log, write_json, write_report, write_verification_readout,
};
use scenarios::{audit_metadata, audit_paths, scenarios, workspace_root};

const METHOD_TAG: &str = "Method V1";
const PERF_PAGE_LIMIT: u64 = 512;
const CHECKPOINT_SCAN_ROOTS: &[&str] = &["crates"];
const AUDIT_TIME_PROBE: &str = "audit_time_probe";
const AUDIT_ENV_PROBE: &str = "audit_env_probe";
const AUDIT_LOG_PROBE: &str = "audit_log_probe";
const AUDIT_SUBNET_REGISTRY_PROBE: &str = "audit_subnet_registry_probe";
const AUDIT_SUBNET_STATE_PROBE: &str = "audit_subnet_state_probe";
const AUDIT_PLAN_CREATE_WORKER_PROBE: &str = "audit_plan_create_worker_probe";
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

/// Write the dated instruction-footprint report and its normalized artifacts.
pub fn generate_instruction_footprint_report() {
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
        .filter(|result| execution::query_perf_is_unobservable(&result.scenario, &result.row))
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
            "audit_leaf_probe".to_string(),
            "audit_root_probe".to_string(),
            "audit_scaling_probe".to_string(),
            "test".to_string(),
            "root".to_string(),
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
