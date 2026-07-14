use canic::{
    Error,
    api::ic::network::NetworkApi,
    cdk::{types::Principal, utils::hash::wasm_hash},
    dto::{
        auth::{
            ActiveDelegationProofStatus, ActiveDelegationProofStatusResponse, DelegatedToken,
            DelegationAudience, RootIssuerPolicyResponse, RootIssuerPolicyUpsertRequest,
            RootIssuerRenewalTemplateResponse, RootIssuerRenewalTemplateUpsertRequest,
        },
        capability::{
            CAPABILITY_VERSION_V1, CapabilityProof, CapabilityRequestMetadata, CapabilityService,
            RootCapabilityEnvelopeV1, RootCapabilityResponseV1,
        },
        env::EnvSnapshotResponse,
        log::LogEntry,
        metrics::{MetricEntry, MetricValue, MetricsKind, QueryPerfSample},
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
use canic_testing_internal::canister::{APP, SCALE_HUB, TEST, USER_HUB};
use canic_testing_internal::pic::{
    CanicWasmBuildProfile, create_user_shard, install_audit_leaf_probe, install_audit_root_probe,
    install_audit_scaling_probe, install_standalone_canister,
    issue_delegated_token_from_active_proof, role_grant,
};
use canic_tests::root::{self, RootSetupProfile, harness::setup_root};
use ic_testkit::pic::Pic;
use serde::Serialize;
use std::{
    collections::BTreeSet,
    convert::TryFrom,
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};

mod estimates;
mod execution;
mod report;
mod scenarios;

use estimates::{
    ExecutionCycleEstimate, apply_execution_cycle_estimates, estimate_options_from_env,
};
use execution::run_scenario;
use report::{
    checkpoint_coverage_gaps, scan_perf_callsites, verification_rows, write_json, write_report,
    write_verification_readout,
};
use scenarios::{audit_metadata, audit_paths, scenarios, workspace_root};

const METHOD_TAG: &str = "CANIC-INSTRUCTION-001/v1";
const PERF_COUNTER_ID: u8 = 1;
const PERF_COUNTER_SOURCE: &str = "performance_counter(1)";
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
        "root proof provisioning and issuer delegated-token issuance/verification",
        "crates/canic-core/src/workflow/runtime/auth",
    ),
    (
        "replay/cached-response path",
        "crates/canic-core/src/workflow/rpc/request/handler/replay.rs",
    ),
    (
        "sharding assignment/query flow",
        "crates/canic-core/src/workflow/placement/sharding",
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
    method_id: String,
    method_version: String,
    method_fingerprint: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    execution_cycle_estimate: Option<ExecutionCycleEstimate>,
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
    method_id: String,
    method_version: String,
    method_fingerprint: String,
    counter_id: u8,
    counter_source: String,
    measured_unit: String,
    counter_semantics: String,
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
    let estimate_options = estimate_options_from_env(&workspace_root)
        .unwrap_or_else(|err| panic!("invalid estimate options: {err}"));
    let scenarios = scenarios();
    let checkpoint_sites = scan_perf_callsites(&workspace_root);

    fs::create_dir_all(&paths.artifacts_dir).expect("create instruction audit artifacts dir");

    let scenario_manifest_path = paths.artifacts_dir.join("scenario-manifest.json");
    let perf_rows_path = paths.artifacts_dir.join("perf-rows.json");
    let verification_path = paths.artifacts_dir.join("verification-readout.md");
    let method_path = paths.artifacts_dir.join("method.json");
    let environment_path = paths.artifacts_dir.join("environment.json");
    let checkpoint_delta_path = paths.artifacts_dir.join("checkpoint-deltas.json");
    let checkpoint_gap_path = paths.artifacts_dir.join("checkpoint-coverage-gaps.json");

    write_json(&scenario_manifest_path, &scenarios);

    let mut results = scenarios
        .iter()
        .map(run_scenario)
        .collect::<Vec<ScenarioResult>>();
    apply_execution_cycle_estimates(&mut results, estimate_options)
        .unwrap_or_else(|err| panic!("failed to estimate execution cycles: {err}"));
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

    let gaps = checkpoint_coverage_gaps(&checkpoint_sites);
    write_json(&checkpoint_gap_path, &gaps);

    let query_unobservable_count = results
        .iter()
        .filter(|result| execution::query_perf_is_unobservable(&result.scenario, &result.row))
        .count();

    let verification_rows = verification_rows(
        &paths,
        &metadata,
        &checkpoint_sites,
        query_unobservable_count,
        checkpoint_rows.len(),
    );
    write_verification_readout(&verification_path, &verification_rows);

    let method = method_artifact(&metadata);
    write_json(&method_path, &method);

    let environment = EnvironmentArtifact {
        code_snapshot_identifier: metadata.code_snapshot.clone(),
        branch: metadata.branch.clone(),
        worktree: metadata.worktree.clone(),
        run_timestamp_utc: metadata.run_timestamp_utc.clone(),
        execution_environment: "PocketIC".to_string(),
        target_canisters_in_scope: vec![
            "leaf_probe".to_string(),
            "root_probe".to_string(),
            "scaling_probe".to_string(),
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

fn method_artifact(metadata: &AuditMetadata) -> MethodArtifact {
    MethodArtifact {
        method_tag: METHOD_TAG.to_string(),
        method_id: metadata.method_id.clone(),
        method_version: metadata.method_version.clone(),
        method_fingerprint: metadata.method_fingerprint.clone(),
        counter_id: PERF_COUNTER_ID,
        counter_source: PERF_COUNTER_SOURCE.to_string(),
        measured_unit: "local_instructions".to_string(),
        counter_semantics: "Local WebAssembly instruction counter for the current call context; excludes other canisters and is not a cycle-charge measurement.".to_string(),
        normalization: "MetricsKind::Runtime perf rows are normalized into canonical endpoint rows. Update/timer lanes use persisted perf deltas; sampled query lanes use local-only QueryPerfSample probe endpoints because query-side perf rows are not committed, so the probe returns the measured local instruction counter alongside the real query result.".to_string(),
        freshness_rule: "One fresh smallest-profile root harness per measured scenario (`topology`, `scaling`, or `sharding`); baseline and post-call perf tables were sampled inside that isolated topology.".to_string(),
        checkpoint_rule: "Checkpoint deltas are diffed from `MetricsKind::Runtime` perf rows before/after sampled update scenarios. Query scenarios remain endpoint-only unless they traverse explicit checkpoint instrumentation.".to_string(),
    }
}

fn sample_origin_for_transport_mode(transport_mode: &str) -> &'static str {
    match transport_mode {
        "query" => "query",
        "composite_query" => "composite_query",
        "update" => "update",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn sample_row() -> CanonicalPerfRow {
        CanonicalPerfRow {
            subject_kind: "endpoint".to_string(),
            subject_label: "canic_env".to_string(),
            count: 1,
            total_local_instructions: 123,
            avg_local_instructions: 123,
            scenario_key: "app:canic_env:minimal-valid".to_string(),
            scenario_labels: vec!["transport_mode=query".to_string()],
            principal_scope: Some("anonymous".to_string()),
            sample_origin: sample_origin_for_transport_mode("query").to_string(),
            execution_cycle_estimate: None,
        }
    }

    #[test]
    fn instruction_row_json_keys_do_not_use_cycle_cost_words() {
        let value = serde_json::to_value(sample_row()).expect("serialize canonical row");
        let keys = json_key_paths(&value);

        assert!(
            keys.iter().any(|key| key == "total_local_instructions"),
            "instruction total field should remain explicit"
        );
        assert!(
            keys.iter().any(|key| key == "avg_local_instructions"),
            "instruction average field should remain explicit"
        );

        for key in keys {
            assert!(
                !measured_instruction_forbidden_key(&key),
                "measured instruction row key must not use cycle-cost wording: {key}"
            );
        }
    }

    #[test]
    fn method_artifact_records_counter_one_instruction_semantics() {
        let metadata = AuditMetadata {
            code_snapshot: "snapshot".to_string(),
            branch: "main".to_string(),
            worktree: "clean".to_string(),
            run_timestamp_utc: "2026-07-14T00:00:00Z".to_string(),
            compared_baseline_report: "N/A".to_string(),
            method_id: "CANIC-INSTRUCTION-001".to_string(),
            method_version: "1".to_string(),
            method_fingerprint: "test-fingerprint".to_string(),
        };
        let artifact =
            serde_json::to_value(method_artifact(&metadata)).expect("serialize method artifact");

        assert_eq!(artifact["method_id"], "CANIC-INSTRUCTION-001");
        assert_eq!(artifact["method_version"], "1");
        assert_eq!(artifact["method_fingerprint"], "test-fingerprint");
        assert_eq!(artifact["counter_id"], PERF_COUNTER_ID);
        assert_eq!(artifact["counter_source"], PERF_COUNTER_SOURCE);
        assert_eq!(artifact["measured_unit"], "local_instructions");
    }

    #[test]
    fn sample_origin_preserves_message_kind_scope() {
        assert_eq!(sample_origin_for_transport_mode("update"), "update");
        assert_eq!(sample_origin_for_transport_mode("query"), "query");
        assert_eq!(
            sample_origin_for_transport_mode("composite_query"),
            "composite_query"
        );
    }

    fn json_key_paths(value: &Value) -> Vec<String> {
        let mut keys = Vec::new();
        collect_json_key_paths(value, "", &mut keys);
        keys
    }

    fn collect_json_key_paths(value: &Value, prefix: &str, keys: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{prefix}.{key}")
                    };
                    keys.push(path.clone());
                    collect_json_key_paths(child, &path, keys);
                }
            }
            Value::Array(items) => {
                for item in items {
                    collect_json_key_paths(item, prefix, keys);
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }

    fn measured_instruction_forbidden_key(key: &str) -> bool {
        FORBIDDEN_MEASURED_KEY_PARTS
            .iter()
            .any(|part| key.contains(part))
    }

    const FORBIDDEN_MEASURED_KEY_PARTS: &[&str] = &[
        "cycle",
        "cycles",
        "burn",
        "charged",
        "cycle_cost",
        "cycle_delta",
    ];
}
