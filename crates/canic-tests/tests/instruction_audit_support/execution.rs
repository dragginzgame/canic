use super::*;

// Query rows only count as unobservable if the same-call probe path failed.
// Query calls do not commit shared perf-table state, so they cannot rely on
// post-call `canic_metrics(MetricsKind::Perf, ...)` reads the way updates do.
pub(super) fn query_perf_is_unobservable(scenario: &AuditScenario, row: &CanonicalPerfRow) -> bool {
    scenario.transport_mode == "query" && row.count == 0
}

// Choose the fresh root topology shape required for one scenario.
fn setup_for_scenario(scenario: &AuditScenario) -> root_harness::RootSetup {
    match scenario.key {
        "root:canic_subnet_registry:full-registry" | "root:canic_subnet_state:empty-struct" => {
            setup_root(RootSetupProfile::Topology)
        }
        "root:canic_request_delegation:fresh-shard"
        | "test:test_verify_delegated_token:valid-delegated-token" => {
            setup_root(RootSetupProfile::Sharding)
        }
        _ => setup_root(RootSetupProfile::Capability),
    }
}

// Execute one scenario in an isolated fresh topology and derive the endpoint delta.
pub(super) fn run_scenario(scenario: &AuditScenario) -> ScenarioResult {
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
    let avg_local_instructions = total_instructions.checked_div(count).unwrap_or(0);

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
        | "app:canic_log:empty-page" => return Some(run_audit_leaf_probe_scenario(scenario)),
        "root:canic_subnet_registry:full-registry" | "root:canic_subnet_state:empty-struct" => {
            return Some(run_audit_root_probe_scenario(scenario));
        }
        "scale_hub:plan_create_worker:empty-pool" => {
            return Some(run_audit_scaling_probe_scenario(scenario));
        }
        "test:test:minimal-valid" => ("canister_test", TEST),
        _ => return None,
    };

    let fixture = install_standalone_canister(crate_name, role, WasmBuildProfile::Fast);
    let target_pid = fixture.canister_id();
    let (count, total_instructions, sample_origin, checkpoint_rows) =
        if scenario.transport_mode == "query" {
            let total = execute_query_perf_probe(fixture.pic(), scenario, target_pid);
            (1, total, "derived".to_string(), Vec::new())
        } else {
            let before = perf_entries(fixture.pic(), target_pid);
            execute_standalone_scenario(fixture.pic(), scenario, target_pid);
            let after = perf_entries(fixture.pic(), target_pid);
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
    let avg_local_instructions = total_instructions.checked_div(count).unwrap_or(0);

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

fn run_audit_leaf_probe_scenario(scenario: &AuditScenario) -> ScenarioResult {
    let fixture = install_audit_leaf_probe(WasmBuildProfile::Fast);
    run_query_only_standalone_result(scenario, fixture.pic(), fixture.canister_id())
}

fn run_audit_root_probe_scenario(scenario: &AuditScenario) -> ScenarioResult {
    let fixture = install_audit_root_probe(WasmBuildProfile::Fast);
    run_query_only_standalone_result(scenario, &fixture.pic, fixture.canister_id)
}

fn run_audit_scaling_probe_scenario(scenario: &AuditScenario) -> ScenarioResult {
    let fixture = install_audit_scaling_probe(WasmBuildProfile::Fast);
    run_query_only_standalone_result(scenario, fixture.pic(), fixture.canister_id())
}

fn run_query_only_standalone_result(
    scenario: &AuditScenario,
    pic: &Pic,
    target_pid: Principal,
) -> ScenarioResult {
    let total = execute_query_perf_probe(pic, scenario, target_pid);

    ScenarioResult {
        scenario: *scenario,
        row: CanonicalPerfRow {
            subject_kind: scenario.subject_kind.to_string(),
            subject_label: scenario.subject_label.to_string(),
            count: 1,
            total_local_instructions: total,
            avg_local_instructions: total,
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
            sample_origin: "derived".to_string(),
        },
        checkpoint_rows: Vec::new(),
    }
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
    root_id: Principal,
    scenario: &AuditScenario,
    subnet_index: &std::collections::HashMap<canic::ids::CanisterRole, Principal>,
) -> Principal {
    match scenario.canister {
        "root" => root_id,
        "app" => *subnet_index
            .get(&APP)
            .expect("app must exist in subnet directory"),
        "scale_hub" => *subnet_index
            .get(&SCALE_HUB)
            .expect("scale_hub must exist in subnet directory"),
        "user_hub" => *subnet_index
            .get(&USER_HUB)
            .expect("user_hub must exist in subnet directory"),
        "test" => *subnet_index
            .get(&TEST)
            .expect("test must exist in subnet directory"),
        other => panic!("unsupported audit canister: {other}"),
    }
}

// Prepare scenario-specific prerequisites outside the measured perf window.
fn prepare_scenario(setup: &root_harness::RootSetup, scenario: &AuditScenario) -> PreparedScenario {
    let target_pid = scenario_target_pid(setup.root_id, scenario, &setup.subnet_index);

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
                .subnet_index
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
                .subnet_index
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
                DelegationAudience::Any,
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
    setup: &root_harness::RootSetup,
    scenario: &AuditScenario,
    prepared: &PreparedScenario,
) {
    let target_pid = prepared.target_pid;
    match scenario.key {
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
    setup: &root_harness::RootSetup,
    _target_pid: Principal,
    prepared: &PreparedScenario,
) {
    let caller = prepared
        .caller_pid
        .expect("auth audit scenario must resolve a shard caller");
    let verifier_pid = *setup
        .subnet_index
        .get(&TEST)
        .expect("test canister must exist for auth audit scenario");
    let response =
        request_root_delegation_provision(&setup.pic, setup.root_id, caller, verifier_pid);
    assert_eq!(response.proof.cert.shard_pid, caller);
}

// Execute the verifier-side delegated token confirmation scenario.
fn execute_verifier_auth_scenario(
    setup: &root_harness::RootSetup,
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
fn execute_root_cycles_scenario(setup: &root_harness::RootSetup, target_pid: Principal) {
    let caller = *setup
        .subnet_index
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
fn execute_query_perf_probe(pic: &Pic, scenario: &AuditScenario, target_pid: Principal) -> u64 {
    match scenario.key {
        "app:canic_time:minimal-valid" => {
            let response: Result<(u64, u64), Error> = pic
                .query_call(target_pid, AUDIT_TIME_PROBE, ())
                .expect("audit_time_probe transport query failed");
            let (_value, perf) = response.expect("audit_time_probe application query failed");
            perf
        }
        "app:canic_env:minimal-valid" => {
            let response: Result<(EnvSnapshotResponse, u64), Error> = pic
                .query_call(target_pid, AUDIT_ENV_PROBE, ())
                .expect("audit_env_probe transport query failed");
            let (_value, perf) = response.expect("audit_env_probe application query failed");
            perf
        }
        "app:canic_log:empty-page" => {
            let response: Result<(Page<LogEntry>, u64), Error> = pic
                .query_call(
                    target_pid,
                    AUDIT_LOG_PROBE,
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
                .expect("audit_log_probe transport query failed");
            let (_value, perf) = response.expect("audit_log_probe application query failed");
            perf
        }
        "root:canic_subnet_registry:full-registry" => {
            let response: Result<(SubnetRegistryResponse, u64), Error> = pic
                .query_call(target_pid, AUDIT_SUBNET_REGISTRY_PROBE, ())
                .expect("audit_subnet_registry_probe transport query failed");
            let (_value, perf) =
                response.expect("audit_subnet_registry_probe application query failed");
            perf
        }
        "root:canic_subnet_state:empty-struct" => {
            let response: Result<(SubnetStateResponse, u64), Error> = pic
                .query_call(target_pid, AUDIT_SUBNET_STATE_PROBE, ())
                .expect("audit_subnet_state_probe transport query failed");
            let (_value, perf) =
                response.expect("audit_subnet_state_probe application query failed");
            perf
        }
        "scale_hub:plan_create_worker:empty-pool" => {
            let response: Result<(bool, u64), Error> = pic
                .query_call(target_pid, AUDIT_PLAN_CREATE_WORKER_PROBE, ())
                .expect("audit_plan_create_worker_probe transport query failed");
            let (_value, perf) =
                response.expect("audit_plan_create_worker_probe application query failed");
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
fn stage_manifest(pic: &Pic, root_id: Principal, manifest: &TemplateManifestInput) {
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
fn prepare_chunk_set(pic: &Pic, root_id: Principal, request: &TemplateChunkSetPrepareInput) {
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
fn publish_chunk(pic: &Pic, root_id: Principal, request: &TemplateChunkInput) {
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
fn perf_entries(pic: &Pic, canister_id: Principal) -> Vec<MetricEntry> {
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
                avg_local_instructions: total_local_instructions.checked_div(count).unwrap_or(0),
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
    setup: &root_harness::RootSetup,
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
fn target_now_secs(setup: &root_harness::RootSetup, canister_id: Principal) -> u64 {
    let _ = canister_id;
    setup.pic.current_time_nanos() / 1_000_000_000
}

// Rebuild the capability metadata tuple that the structural envelope expects.
fn capability_metadata_from_request(request: &Request) -> ([u8; 16], [u8; 16], u32) {
    let metadata = match request {
        Request::CreateCanister(req) => req.metadata,
        Request::UpgradeCanister(req) => req.metadata,
        Request::RecycleCanister(req) => req.metadata,
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
