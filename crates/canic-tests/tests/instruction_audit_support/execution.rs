use super::*;

fn setup_for_scenario(scenario: &AuditScenario) -> root::harness::RootSetup {
    match scenario.key {
        "root:bootstrap:init-checkpoints" => setup_root(RootSetupProfile::Topology),
        "scale:request_cycles_from_parent:fresh" | "scale_hub:create_worker:empty-pool" => {
            setup_root(RootSetupProfile::Scaling)
        }
        "user_hub:create_account:new-principal"
        | "root:test_provision_chain_key_delegation_proof_for_issuer:new-issuer"
        | "issuer:canic_prepare_delegated_token:active-proof"
        | "test:test_verify_delegated_token:valid-delegated-token" => {
            setup_root(RootSetupProfile::Sharding)
        }
        _ => setup_root(RootSetupProfile::Capability),
    }
}

// Execute one v2 scenario in a fresh authoritative root-harness topology.
pub(super) fn run_scenario(scenario: &AuditScenario) -> ScenarioResult {
    let setup = setup_for_scenario(scenario);
    if scenario.transport_mode == "install" {
        return observe_bootstrap_scenario(setup, scenario);
    }

    let prepared = prepare_scenario(&setup, scenario);
    let target_pid = prepared.target_pid;
    let before = perf_entries(&setup.pic, target_pid);
    execute_scenario(&setup, scenario, &prepared);
    let after = perf_entries(&setup.pic, target_pid);
    let (count, total_instructions) = perf_delta(
        &before,
        &after,
        scenario.subject_kind,
        scenario.transport_mode,
        scenario.subject_label,
    );
    let checkpoint_rows = checkpoint_deltas(scenario, &before, &after);
    drop(setup);

    scenario_result(scenario, count, total_instructions, checkpoint_rows)
}

fn scenario_result(
    scenario: &AuditScenario,
    count: u64,
    total_instructions: u64,
    checkpoint_rows: Vec<CheckpointDeltaRow>,
) -> ScenarioResult {
    ScenarioResult {
        scenario: *scenario,
        row: CanonicalPerfRow {
            subject_kind: scenario.subject_kind.to_string(),
            subject_label: scenario.subject_label.to_string(),
            count,
            total_local_instructions: total_instructions,
            avg_local_instructions: total_instructions.checked_div(count).unwrap_or(0),
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
            sample_origin: sample_origin_for_transport_mode(scenario.transport_mode).to_string(),
            execution_cycle_estimate: None,
        },
        checkpoint_rows,
    }
}

fn observe_bootstrap_scenario(
    setup: root::harness::RootSetup,
    scenario: &AuditScenario,
) -> ScenarioResult {
    let entries = perf_entries(&setup.pic, setup.root_id);
    let checkpoint_rows = checkpoint_deltas(scenario, &[], &entries)
        .into_iter()
        .filter(|row| row.label.starts_with("bootstrap_"))
        .collect::<Vec<_>>();
    assert!(
        !checkpoint_rows.is_empty(),
        "root bootstrap scenario produced no bootstrap checkpoints"
    );
    let total_instructions = checkpoint_rows
        .iter()
        .map(|row| row.total_local_instructions)
        .sum();
    assert!(
        total_instructions > 0,
        "root bootstrap scenario produced a zero checkpoint total"
    );
    drop(setup);

    scenario_result(scenario, 1, total_instructions, checkpoint_rows)
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
            .expect("app must exist in subnet index"),
        "scale_hub" => *subnet_index
            .get(&SCALE_HUB)
            .expect("scale_hub must exist in subnet index"),
        "user_hub" => *subnet_index
            .get(&USER_HUB)
            .expect("user_hub must exist in subnet index"),
        "test" => *subnet_index
            .get(&TEST)
            .expect("test must exist in subnet index"),
        other => panic!("unsupported audit canister: {other}"),
    }
}

// Prepare scenario-specific prerequisites outside the measured perf window.
fn prepare_scenario(
    setup: &root::harness::RootSetup,
    scenario: &AuditScenario,
) -> PreparedScenario {
    let target_pid = match scenario.key {
        "scale:request_cycles_from_parent:fresh" => {
            let scale_hub_pid = *setup
                .subnet_index
                .get(&SCALE_HUB)
                .expect("scale_hub must exist for scale child scenario");
            let worker_pid = root::workers::create_worker(&setup.pic, scale_hub_pid)
                .expect("scale_hub must create a scale child for instruction audit");
            root::workers::prepare_worker_for_explicit_parent_funding(&setup.pic, worker_pid);
            worker_pid
        }
        _ if scenario.canister == "issuer" => setup.root_id,
        _ => scenario_target_pid(setup.root_id, scenario, &setup.subnet_index),
    };

    match scenario.key {
        "root:canic_response_capability_v1:request-cycles-replay" => {
            execute_root_cycles_scenario(setup, target_pid);
            PreparedScenario {
                target_pid,
                caller_pid: None,
                issuer_pid: None,
                delegated_token: None,
            }
        }
        "root:canic_template_prepare_admin:single-chunk" => {
            let fixture = audit_template_fixture(scenario);
            stage_manifest(&setup.pic, target_pid, &fixture.manifest);
            PreparedScenario {
                target_pid,
                caller_pid: None,
                issuer_pid: None,
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
                issuer_pid: None,
                delegated_token: None,
            }
        }
        "root:test_provision_chain_key_delegation_proof_for_issuer:new-issuer" => {
            let (issuer_pid, _) = prepare_issuer(setup, 44);
            PreparedScenario {
                target_pid,
                caller_pid: None,
                issuer_pid: Some(issuer_pid),
                delegated_token: None,
            }
        }
        "issuer:canic_prepare_delegated_token:active-proof" => {
            let (issuer_pid, subject) = prepare_issuer(setup, 46);
            provision_delegation_proof(setup, issuer_pid);
            PreparedScenario {
                target_pid: issuer_pid,
                caller_pid: Some(subject),
                issuer_pid: Some(issuer_pid),
                delegated_token: None,
            }
        }
        "test:test_verify_delegated_token:valid-delegated-token" => {
            let (issuer_pid, subject) = prepare_issuer(setup, 45);
            provision_delegation_proof(setup, issuer_pid);
            let token = issue_delegated_token_from_active_proof(
                &setup.pic,
                issuer_pid,
                subject,
                DelegationAudience::Project("test".to_string()),
                vec![role_grant(TEST, vec![cap::VERIFY.to_string()])],
                10_000_000_000,
            );
            PreparedScenario {
                target_pid,
                caller_pid: Some(subject),
                issuer_pid: Some(issuer_pid),
                delegated_token: Some(token),
            }
        }
        _ => PreparedScenario {
            target_pid,
            caller_pid: None,
            issuer_pid: None,
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
        "scale:request_cycles_from_parent:fresh" => {
            let response: Result<u128, Error> = setup
                .pic
                .update_call(target_pid, "request_cycles_from_parent", (999u128,))
                .expect("scale request_cycles_from_parent transport failed");
            assert_eq!(
                response.expect("scale request_cycles_from_parent application failed"),
                999
            );
        }
        "test:test_verify_delegated_token:valid-delegated-token" => {
            execute_verifier_auth_scenario(setup, target_pid, prepared);
        }
        "root:canic_response_capability_v1:request-cycles-fresh"
        | "root:canic_response_capability_v1:request-cycles-replay" => {
            execute_root_cycles_scenario(setup, target_pid);
        }
        "user_hub:create_account:new-principal" => {
            let created: Result<Principal, Error> = setup
                .pic
                .update_call(
                    target_pid,
                    "create_account",
                    (Principal::from_slice(&[51; 29]),),
                )
                .expect("create_account transport failed");
            created.expect("create_account application failed");
        }
        "scale_hub:create_worker:empty-pool" => {
            let created: Result<Principal, Error> = setup
                .pic
                .update_call(target_pid, "create_worker", ())
                .expect("create_worker transport failed");
            created.expect("create_worker application failed");
        }
        "root:test_provision_chain_key_delegation_proof_for_issuer:new-issuer" => {
            provision_delegation_proof(
                setup,
                prepared
                    .issuer_pid
                    .expect("root proof scenario must prepare an issuer"),
            );
        }
        "issuer:canic_prepare_delegated_token:active-proof" => {
            execute_delegated_token_prepare(setup, prepared);
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

fn prepare_issuer(setup: &root::harness::RootSetup, subject_byte: u8) -> (Principal, Principal) {
    let user_hub_pid = *setup
        .subnet_index
        .get(&USER_HUB)
        .expect("user_hub must exist for delegated auth audit scenarios");
    let subject = Principal::from_slice(&[subject_byte; 29]);
    let issuer_pid = create_user_shard(&setup.pic, user_hub_pid, subject);
    upsert_delegation_issuer(setup, issuer_pid);
    upsert_delegation_renewal_template(setup, issuer_pid);
    (issuer_pid, subject)
}

fn provision_delegation_proof(setup: &root::harness::RootSetup, issuer_pid: Principal) {
    let provisioned: Result<(), Error> = setup
        .pic
        .update_call(
            setup.root_id,
            "test_provision_chain_key_delegation_proof_for_issuer",
            (issuer_pid,),
        )
        .expect("root proof provisioning transport failed");
    provisioned.expect("root proof provisioning application failed");
}

fn execute_delegated_token_prepare(setup: &root::harness::RootSetup, prepared: &PreparedScenario) {
    let subject = prepared
        .caller_pid
        .expect("delegated prepare scenario must have a subject");
    let _issuer_pid = prepared
        .issuer_pid
        .expect("delegated prepare scenario must have an issuer");
    let response: Result<DelegatedTokenPrepareResponse, Error> = setup
        .pic
        .update_call_as(
            prepared.target_pid,
            subject,
            protocol::CANIC_PREPARE_DELEGATED_TOKEN,
            (DelegatedTokenPrepareRequest {
                metadata: Some(AuthRequestMetadata {
                    request_id: [92; 32],
                    ttl_ns: 60_000_000_000,
                }),
                subject,
                aud: DelegationAudience::Project("test".to_string()),
                grants: vec![role_grant(TEST, vec![cap::VERIFY.to_string()])],
                ttl_ns: 10_000_000_000,
                ext: None,
            },),
        )
        .expect("delegated token prepare transport failed");
    response.expect("delegated token prepare application failed");
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
        .expect("verifier auth audit scenario must issue a delegated token");
    let response: Result<Result<(), Error>, _> =
        setup
            .pic
            .update_call_as(target_pid, caller, "test_verify_delegated_token", (token,));
    response
        .expect("test_verify_delegated_token transport failed")
        .expect("test_verify_delegated_token application failed");
}

fn upsert_delegation_issuer(setup: &root::harness::RootSetup, issuer_pid: Principal) {
    let registered: Result<RootIssuerPolicyResponse, Error> = setup
        .pic
        .update_call(
            setup.root_id,
            protocol::CANIC_UPSERT_ROOT_ISSUER_POLICY,
            (RootIssuerPolicyUpsertRequest {
                issuer_pid,
                enabled: true,
                allowed_audiences: vec![DelegationAudience::Project("test".to_string())],
                allowed_grants: vec![role_grant(TEST, vec![cap::VERIFY.to_string()])],
                max_cert_ttl_ns: 60_000_000_000,
                refresh_after_ratio_bps: 8_000,
            },),
        )
        .expect("root issuer registration transport failed");
    let registered = registered.expect("root issuer registration application failed");
    assert_eq!(registered.issuer.issuer_pid, issuer_pid);
}

fn upsert_delegation_renewal_template(setup: &root::harness::RootSetup, issuer_pid: Principal) {
    let response: Result<RootIssuerRenewalTemplateResponse, Error> = setup
        .pic
        .update_call(
            setup.root_id,
            protocol::CANIC_UPSERT_ROOT_ISSUER_RENEWAL_TEMPLATE,
            (RootIssuerRenewalTemplateUpsertRequest {
                issuer_pid,
                enabled: true,
                aud: DelegationAudience::Project("test".to_string()),
                grants: vec![role_grant(TEST, vec![cap::VERIFY.to_string()])],
                cert_ttl_ns: 60_000_000_000,
            },),
        )
        .expect("root issuer renewal template transport failed");
    let response = response.expect("root issuer renewal template application failed");
    assert_eq!(response.template.issuer_pid, issuer_pid);
}

// Execute the fresh root cycles request scenario through the root dispatcher.
fn execute_root_cycles_scenario(setup: &root::harness::RootSetup, target_pid: Principal) {
    let caller = *setup
        .subnet_index
        .get(&TEST)
        .expect("test canister must exist for root capability request");
    let request = Request::Cycles(CyclesRequest {
        cycles: 999,
        metadata: Some(metadata([90u8; 32], 120_000_000_000)),
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

// Build one synthetic staged-release fixture for root admin perf scenarios.
fn audit_template_fixture(scenario: &AuditScenario) -> AuditTemplateFixture {
    let slug = scenario.key.replace(':', "-");
    let bytes = format!("canic-instruction-audit-{slug}").into_bytes();
    let payload_hash = wasm_hash(&bytes);
    let chunk_hashes = vec![wasm_hash(&bytes)];
    let template_id = TemplateId::from(format!("audit:{slug}"));
    let version = TemplateVersion::from(format!("instruction-audit-{slug}"));

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
                MetricsKind::Runtime,
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
    transport_mode: &str,
    subject_label: &str,
) -> (u64, u64) {
    let before_slot = perf_slot(before, subject_kind, transport_mode, subject_label);
    let after_slot = perf_slot(after, subject_kind, transport_mode, subject_label);

    (
        after_slot.0.saturating_sub(before_slot.0),
        after_slot.1.saturating_sub(before_slot.1),
    )
}

// Project one perf row into `(count, total_instructions)`.
fn perf_slot(
    entries: &[MetricEntry],
    subject_kind: &str,
    transport_mode: &str,
    subject_label: &str,
) -> (u64, u64) {
    entries
        .iter()
        .find_map(|entry| {
            let [family, kind, origin, label] = entry.labels.as_slice() else {
                return None;
            };
            if family == "perf"
                && kind == subject_kind
                && origin == transport_mode
                && label == subject_label
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
            let [family, kind, scope, label] = entry.labels.as_slice() else {
                return None;
            };
            if family != "perf" || kind != "checkpoint" {
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
            let [family, kind, entry_scope, entry_label] = entry.labels.as_slice() else {
                return None;
            };
            if family == "perf"
                && kind == "checkpoint"
                && entry_scope == scope
                && entry_label == label
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

// Execute one structural root capability call as the requested child caller.
fn root_capability_response_as(
    setup: &root::harness::RootSetup,
    target_pid: Principal,
    caller: Principal,
    request: Request,
) -> Result<Response, Error> {
    let (request_id, ttl_ns) = capability_metadata_from_request(&request);
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request,
        proof: CapabilityProof::Structural,
        metadata: CapabilityRequestMetadata {
            request_id,
            issued_at_ns: target_now_ns(setup, target_pid),
            ttl_ns,
        },
    };

    let result: Result<Result<RootCapabilityResponseV1, Error>, _> = setup.pic.update_call_as(
        target_pid,
        caller,
        protocol::CANIC_RESPONSE_CAPABILITY_V1,
        (envelope,),
    );
    result
        .expect("root capability transport call failed")
        .map(|response| response.response)
}

// Read one canister's current time in nanoseconds for capability metadata issuance.
fn target_now_ns(setup: &root::harness::RootSetup, canister_id: Principal) -> u64 {
    let _ = canister_id;
    setup.pic.current_time_nanos()
}

// Rebuild the capability metadata tuple that the structural envelope expects.
const fn capability_metadata_from_request(request: &Request) -> ([u8; 32], u64) {
    let metadata = match request {
        Request::AcknowledgePlacementReceipt(req) => req.metadata,
        Request::AllocatePlacementChild(req) | Request::CreateCanister(req) => req.metadata,
        Request::UpgradeCanister(req) => req.metadata,
        Request::RecycleCanister(req) => req.metadata,
        Request::Cycles(req) => req.metadata,
    };

    match metadata {
        Some(meta) => (meta.request_id, meta.ttl_ns),
        None => ([0u8; 32], 60_000_000_000),
    }
}

// Build one deterministic root request metadata value for audit scenarios.
const fn metadata(request_id: [u8; 32], ttl_ns: u64) -> RootRequestMetadata {
    RootRequestMetadata { request_id, ttl_ns }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_perf_slot_binds_call_kind_before_endpoint_name() {
        let entries = vec![MetricEntry {
            labels: vec![
                "perf".to_string(),
                "endpoint".to_string(),
                "update".to_string(),
                "request_cycles_from_parent".to_string(),
            ],
            principal: None,
            value: MetricValue::CountAndU64 {
                count: 1,
                value_u64: 123,
            },
        }];

        assert_eq!(
            perf_slot(&entries, "endpoint", "update", "request_cycles_from_parent"),
            (1, 123)
        );
        assert_eq!(
            perf_slot(&entries, "endpoint", "query", "request_cycles_from_parent"),
            (0, 0)
        );
    }
}
