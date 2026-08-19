use super::*;

#[derive(CandidType)]
enum RootCommand {
    RespondCapability(RootCapabilityEnvelopeV1),
    UpsertIssuerPolicy(RootIssuerPolicyUpsertRequest),
    UpsertIssuerRenewalTemplate(RootIssuerRenewalTemplateUpsertRequest),
}

#[derive(CandidType, Deserialize)]
enum RootCommandResponse {
    RespondCapability(RootCapabilityResponseV1),
    UpsertIssuerPolicy(RootIssuerPolicyResponse),
    UpsertIssuerRenewalTemplate(RootIssuerRenewalTemplateResponse),
}

#[derive(CandidType)]
enum CanisterCommand {
    PrepareDelegatedToken(DelegatedTokenPrepareRequest),
}

#[derive(CandidType, Deserialize)]
enum CanisterCommandResponse {
    PrepareDelegatedToken(DelegatedTokenPrepareResponse),
}

#[derive(CandidType)]
enum RoleStatusRequest {
    Metrics(MetricsStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum RoleStatusResponse {
    Metrics(Page<MetricEntry>),
}

fn setup_for_scenario(scenario: &AuditScenario) -> root::harness::RootSetup {
    match scenario.key {
        "root:bootstrap:init-checkpoints" => setup_root(RootSetupProfile::Topology),
        "scale:request_cycles_from_parent:fresh" | "scale_hub:create_worker:empty-pool" => {
            setup_root(RootSetupProfile::Scaling)
        }
        "user_hub:create_account:new-principal" => setup_root(RootSetupProfile::Sharding),
        _ => setup_root(RootSetupProfile::Capability),
    }
}

// Execute one v3 scenario in an exactly restored authoritative baseline.
pub(super) fn run_scenario(scenario: &AuditScenario) -> ScenarioResult {
    if is_registry_auth_scenario(scenario) {
        return run_registry_auth_scenario(scenario);
    }

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

fn is_registry_auth_scenario(scenario: &AuditScenario) -> bool {
    matches!(
        scenario.key,
        "root:test_provision_chain_key_delegation_proof_for_issuer:new-issuer"
            | "issuer:canic_command.PrepareDelegatedToken:active-proof"
            | "issuer_verifier:issuer_verify_token:valid-delegated-token"
    )
}

fn run_registry_auth_scenario(scenario: &AuditScenario) -> ScenarioResult {
    let setup = setup_active_component_registry();
    let prepared = prepare_registry_auth_scenario(&setup, scenario);
    let before = perf_entries(setup.pic(), prepared.target_pid);
    execute_registry_auth_scenario(&setup, scenario, &prepared);
    let after = perf_entries(setup.pic(), prepared.target_pid);
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

fn prepare_registry_auth_scenario(
    setup: &ActiveComponentRegistryFixture,
    scenario: &AuditScenario,
) -> PreparedScenario {
    let subject = Principal::from_slice(&[scenario.key.as_bytes()[0]; 29]);
    upsert_delegation_issuer(
        setup.pic(),
        setup.root,
        setup.issuer.canister_id,
        &setup.verifier.role,
    );
    upsert_delegation_renewal_template(
        setup.pic(),
        setup.root,
        setup.issuer.canister_id,
        &setup.verifier.role,
    );

    match scenario.key {
        "root:test_provision_chain_key_delegation_proof_for_issuer:new-issuer" => {
            PreparedScenario {
                target_pid: setup.root,
                caller_pid: None,
                issuer_pid: Some(setup.issuer.canister_id),
                delegated_token: None,
            }
        }
        "issuer:canic_command.PrepareDelegatedToken:active-proof" => {
            provision_delegation_proof(setup.pic(), setup.root, setup.issuer.canister_id);
            PreparedScenario {
                target_pid: setup.issuer.canister_id,
                caller_pid: Some(subject),
                issuer_pid: Some(setup.issuer.canister_id),
                delegated_token: None,
            }
        }
        "issuer_verifier:issuer_verify_token:valid-delegated-token" => {
            provision_delegation_proof(setup.pic(), setup.root, setup.issuer.canister_id);
            let token = issue_delegated_token_from_active_proof(
                setup.pic(),
                setup.issuer.canister_id,
                subject,
                DelegationAudience::Fleet(test_fleet()),
                vec![role_grant(
                    setup.verifier.role.clone(),
                    vec![cap::VERIFY.to_string()],
                )],
                10_000_000_000,
            );
            PreparedScenario {
                target_pid: setup.verifier.canister_id,
                caller_pid: Some(subject),
                issuer_pid: Some(setup.issuer.canister_id),
                delegated_token: Some(token),
            }
        }
        other => panic!("unsupported Registry-bound auth audit scenario: {other}"),
    }
}

fn execute_registry_auth_scenario(
    setup: &ActiveComponentRegistryFixture,
    scenario: &AuditScenario,
    prepared: &PreparedScenario,
) {
    match scenario.key {
        "root:test_provision_chain_key_delegation_proof_for_issuer:new-issuer" => {
            provision_delegation_proof(
                setup.pic(),
                setup.root,
                prepared
                    .issuer_pid
                    .expect("root proof scenario must prepare an issuer"),
            );
        }
        "issuer:canic_command.PrepareDelegatedToken:active-proof" => {
            execute_delegated_token_prepare(setup.pic(), prepared, &setup.verifier.role);
        }
        "issuer_verifier:issuer_verify_token:valid-delegated-token" => {
            execute_verifier_auth_scenario(setup.pic(), prepared.target_pid, prepared);
        }
        other => panic!("unsupported Registry-bound auth audit scenario: {other}"),
    }
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
    component_canisters: &std::collections::HashMap<canic::ids::CanisterRole, Principal>,
) -> Principal {
    match scenario.canister {
        "root" => root_id,
        "app" => *component_canisters
            .get(&APP)
            .expect("app Component must exist"),
        "scale_hub" => *component_canisters
            .get(&SCALE_HUB)
            .expect("scale_hub Component must exist"),
        "user_hub" => *component_canisters
            .get(&USER_HUB)
            .expect("user_hub Component must exist"),
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
                .component_canisters
                .get(&SCALE_HUB)
                .expect("scale_hub must exist for scale child scenario");
            let worker_pid = root::workers::create_worker(&setup.pic, setup.root_id, scale_hub_pid)
                .expect("scale_hub must create a scale child for instruction audit");
            root::workers::prepare_worker_for_explicit_parent_funding(&setup.pic, worker_pid);
            worker_pid
        }
        _ if scenario.canister == "issuer" => setup.root_id,
        _ => scenario_target_pid(setup.root_id, scenario, &setup.component_canisters),
    };

    match scenario.key {
        "root:canic_command:respond-capability-request-cycles-replay" => {
            execute_root_cycles_scenario(setup, target_pid);
            PreparedScenario {
                target_pid,
                caller_pid: None,
                issuer_pid: None,
                delegated_token: None,
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
                .update_candid(target_pid, "request_cycles_from_parent", (999u128,))
                .expect("scale request_cycles_from_parent transport failed");
            assert_eq!(
                response.expect("scale request_cycles_from_parent application failed"),
                999
            );
        }
        "root:canic_command:respond-capability-request-cycles-fresh"
        | "root:canic_command:respond-capability-request-cycles-replay" => {
            execute_root_cycles_scenario(setup, target_pid);
        }
        "user_hub:create_account:new-principal" => {
            let created: Result<Principal, Error> = setup
                .pic
                .update_candid(
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
                .update_candid(target_pid, "create_worker", ())
                .expect("create_worker transport failed");
            created.expect("create_worker application failed");
        }
        other => panic!("unsupported audit scenario: {other}"),
    }
}

fn provision_delegation_proof(pic: &PocketIc, root: Principal, issuer_pid: Principal) {
    let provisioned: Result<(), Error> = pic
        .update_candid(
            root,
            "test_provision_chain_key_delegation_proof_for_issuer",
            (issuer_pid,),
        )
        .expect("root proof provisioning transport failed");
    provisioned.expect("root proof provisioning application failed");
}

fn execute_delegated_token_prepare(
    pic: &PocketIc,
    prepared: &PreparedScenario,
    verifier_role: &canic::ids::CanisterRole,
) {
    let subject = prepared
        .caller_pid
        .expect("delegated prepare scenario must have a subject");
    let _issuer_pid = prepared
        .issuer_pid
        .expect("delegated prepare scenario must have an issuer");
    let response: Result<CanisterCommandResponse, Error> = pic
        .update_candid_as(
            prepared.target_pid,
            subject,
            protocol::CANIC_COMMAND,
            (CanisterCommand::PrepareDelegatedToken(
                DelegatedTokenPrepareRequest {
                    metadata: Some(AuthRequestMetadata {
                        request_id: [92; 32],
                        ttl_ns: 60_000_000_000,
                    }),
                    subject,
                    aud: DelegationAudience::Fleet(test_fleet()),
                    grants: vec![role_grant(
                        verifier_role.clone(),
                        vec![cap::VERIFY.to_string()],
                    )],
                    ttl_ns: 10_000_000_000,
                    ext: None,
                },
            ),),
        )
        .expect("delegated token prepare transport failed");
    let CanisterCommandResponse::PrepareDelegatedToken(_) =
        response.expect("delegated token prepare application failed");
}

// Execute delegated-token confirmation through the second issuer Component.
fn execute_verifier_auth_scenario(
    pic: &PocketIc,
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
        pic.update_candid_as(target_pid, caller, "issuer_verify_token", (token,));
    response
        .expect("issuer_verify_token transport failed")
        .expect("issuer_verify_token application failed");
}

fn upsert_delegation_issuer(
    pic: &PocketIc,
    root: Principal,
    issuer_pid: Principal,
    verifier_role: &canic::ids::CanisterRole,
) {
    let registered: Result<RootCommandResponse, Error> = pic
        .update_candid(
            root,
            protocol::CANIC_COMMAND,
            (RootCommand::UpsertIssuerPolicy(
                RootIssuerPolicyUpsertRequest {
                    issuer_pid,
                    enabled: true,
                    allowed_audiences: vec![DelegationAudience::Fleet(test_fleet())],
                    allowed_grants: vec![role_grant(
                        verifier_role.clone(),
                        vec![cap::VERIFY.to_string()],
                    )],
                    max_cert_ttl_ns: 60_000_000_000,
                    refresh_after_ratio_bps: 8_000,
                },
            ),),
        )
        .expect("root issuer registration transport failed");
    let RootCommandResponse::UpsertIssuerPolicy(registered) =
        registered.expect("root issuer registration application failed")
    else {
        panic!("unexpected Root command response");
    };
    assert_eq!(registered.issuer.issuer_pid, issuer_pid);
}

fn upsert_delegation_renewal_template(
    pic: &PocketIc,
    root: Principal,
    issuer_pid: Principal,
    verifier_role: &canic::ids::CanisterRole,
) {
    let response: Result<RootCommandResponse, Error> = pic
        .update_candid(
            root,
            protocol::CANIC_COMMAND,
            (RootCommand::UpsertIssuerRenewalTemplate(
                RootIssuerRenewalTemplateUpsertRequest {
                    issuer_pid,
                    enabled: true,
                    aud: DelegationAudience::Fleet(test_fleet()),
                    grants: vec![role_grant(
                        verifier_role.clone(),
                        vec![cap::VERIFY.to_string()],
                    )],
                    cert_ttl_ns: 60_000_000_000,
                },
            ),),
        )
        .expect("root issuer renewal template transport failed");
    let RootCommandResponse::UpsertIssuerRenewalTemplate(response) =
        response.expect("root issuer renewal template application failed")
    else {
        panic!("unexpected Root command response");
    };
    assert_eq!(response.template.issuer_pid, issuer_pid);
}

// Execute the fresh root cycles request scenario through the root dispatcher.
fn execute_root_cycles_scenario(setup: &root::harness::RootSetup, target_pid: Principal) {
    let caller = *setup
        .component_canisters
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
            assert_eq!(response.cycles_transferred(), Some(999));
        }
        other => panic!("expected cycles response, got: {other:?}"),
    }
}

// Read the current perf metrics table for one canister.
fn perf_entries(pic: &PocketIc, canister_id: Principal) -> Vec<MetricEntry> {
    let response: Result<RoleStatusResponse, Error> = pic
        .query_candid(
            canister_id,
            protocol::CANIC_STATUS,
            (RoleStatusRequest::Metrics(MetricsStatusRequest {
                kind: MetricsKind::Runtime,
                page: PageRequest {
                    limit: PERF_PAGE_LIMIT,
                    offset: 0,
                },
            }),),
        )
        .expect("perf metrics transport query failed");

    let RoleStatusResponse::Metrics(page) =
        response.expect("perf metrics application query failed");
    page.entries
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

    let result: Result<Result<RootCommandResponse, Error>, _> = setup.pic.update_candid_as(
        target_pid,
        caller,
        protocol::CANIC_COMMAND,
        (RootCommand::RespondCapability(envelope),),
    );
    let response = result.expect("root capability transport call failed")?;
    let RootCommandResponse::RespondCapability(response) = response else {
        panic!("root capability command returned an uncorrelated response")
    };
    Ok(response.response)
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
