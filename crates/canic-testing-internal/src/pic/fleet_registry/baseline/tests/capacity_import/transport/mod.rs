//! Qualify signed host handoff and the protected Root client on the real HTTP gateway.

mod admission;
mod expiry;
mod rejection;

use super::*;
use canic_core::{control_plane_support::policy::pool_import, protocol};
use canic_host::{
    fleet_ensure::{
        model::capacity_import::{
            CapacityImportAuthority, CapacityImportDisposition, CapacityImportPlanRecord,
            CapacityImportRootBudget, CapacityImportSourceBinding, CapacityImportSourceRecord,
        },
        ops::{
            EnsurePaths,
            capacity_import::{
                admission::observer::CapacityImportLiveObserver,
                journal::{self, CapacityImportJournalError, CapacityImportJournalStore},
                prepare_review, publication, reservation_evidence,
                transport::{CapacityImportTransport, HandoffOutcome},
            },
        },
        view::capacity_import::{
            CapacityImportDestinationView, CapacityImportOwnershipView, CapacityImportSourceView,
        },
    },
    icp::{IcpCli, IcpRequestKind, IcpRequestTiming},
};
use std::sync::{Arc, Mutex};

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one HTTP journey preserves signed intent across host restart before Root takes custody"
)]
pub(in crate::pic::fleet_registry::baseline::tests) fn host_import_transport_recovers_signed_handoff_and_root_progress()
 {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
    let directory = literal_zero_adapter_root(&workspace).join("capacity-import-transport");
    let _cleanup = TestDirectoryCleanup(directory.clone());
    let (wrapper, operator, _) = prepare_isolated_icp(&directory);
    let (mut pic, root) = setup_separate_coordinator();
    pic.set_controllers(root, None, vec![Principal::anonymous(), operator])
        .unwrap();
    let source = pic.create_canister_on_subnet(None, None, pic.get_subnet(root).unwrap());
    pic.install_canister(source, b"\0asm\x01\0\0\0".to_vec(), Vec::new(), None);
    pic.stop_canister(source, None).unwrap();
    pic.set_controllers(
        source,
        None,
        vec![operator, Principal::self_authenticating(b"retired owner")],
    )
    .unwrap();
    let stopped_source = pic.canister_status(source, Some(operator)).unwrap();
    let url = pic.make_live(None);
    let timings = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&timings);
    let icp = IcpCli::new(wrapper.to_str().unwrap(), Some("local".into()))
        .with_local_replica(Some(LocalReplicaTarget {
            environment: "local".into(),
            root_key: hex_bytes(pic.root_key().unwrap()),
            url: url.to_string(),
        }))
        .with_timing_handler(move |event| sink.lock().unwrap().push(event));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let transport = CapacityImportTransport::from_icp(&icp).unwrap();
    let context = runtime.block_on(transport.root_context(root)).unwrap();
    let plan = review(&pic, root, &[source], operator, &context);
    let source_version = pic.canister_status(source, Some(operator)).unwrap().version;
    assert!(matches!(
        runtime.block_on(transport.verify_destination(&plan)),
        Err(CapacityImportJournalError::CoordinatorRejected(_))
    ));
    assert_eq!(
        pic.canister_status(source, Some(operator)).unwrap().version,
        source_version
    );
    pic.set_controllers(
        context.binding.authority.binding.coordinator,
        None,
        vec![Principal::anonymous(), operator],
    )
    .unwrap();
    assert_eq!(
        runtime
            .block_on(transport.verify_destination(&plan))
            .unwrap(),
        context
    );
    let pool = root_pool_status(&pic, root);
    let destination = CapacityImportDestinationView {
        authority: plan.authority.clone(),
        ready: true,
        draining: false,
        competing_operation: false,
        occupied_capacity: pool.tracked - pool.store,
        maximum_capacity: pool.config.maximum_size,
        controlled_cycles: plan.root_budget.observed_cycles,
        reserved_cycles: plan.root_budget.observed_reserved_cycles,
        minimum_retained_cycles: plan.root_budget.minimum_retained_cycles,
        assigned_canisters: pool
            .entries
            .iter()
            .map(|entry| entry.canister_id)
            .chain([root, context.binding.authority.binding.coordinator])
            .collect(),
    };
    let paths = EnsurePaths::under(&directory, "local", "capacity-import");
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    retain_inputs(&workspace, &paths, &plan, &pool);
    let staged = publication::bind(
        &paths,
        &journal::reviewed(plan.clone()).unwrap(),
        Path::new("policy.toml"),
        Path::new("seed.toml"),
    )
    .unwrap();
    let staged = store.stage_review(staged).unwrap();
    let observed = observe(&pic, &plan, &stopped_source);
    let approved = journal::approve(
        &staged,
        plan.plan_sha256,
        &destination,
        std::slice::from_ref(&observed),
    )
    .unwrap();
    store.save(&approved).unwrap();
    let reserved = runtime
        .block_on(async {
            transport
                .prepare_reserve_root(&approved)
                .await?
                .submit()
                .await
        })
        .unwrap();
    let reserved =
        journal::reserve(&approved, reservation_evidence(&plan, &reserved).unwrap()).unwrap();
    store.save(&reserved).unwrap();
    assert!(matches!(
        runtime.block_on(transport.prepare_advance_root(&reserved, source)),
        Err(CapacityImportJournalError::Unresolved)
    ));
    let request = transport.prepare(&plan, source).unwrap();
    let intent = journal::prepare_handoff(&reserved, &observed, request).unwrap();
    store.save(&intent).unwrap();
    let issued = journal::issue_handoff(&intent, source).unwrap();
    store.save(&issued).unwrap();
    // Discard the submission response and reopen the durable owner with a new Agent.
    runtime
        .block_on(async {
            transport
                .prepare_submission(&issued, source)
                .await?
                .submit(&issued)
                .await
        })
        .unwrap();
    drop(transport);
    drop(store);
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    let issued = store.read().unwrap().unwrap();
    let transport = CapacityImportTransport::from_icp(&icp).unwrap();
    let completion = runtime.block_on(async {
        for _ in 0..40 {
            match transport.completion(&issued, source).await {
                Ok(HandoffOutcome::Completed(completion)) => return completion,
                Ok(HandoffOutcome::Retired(_)) => panic!("unexpected rejected handoff"),
                Err(CapacityImportJournalError::Unresolved) => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(error) => panic!("certified handoff completion: {error:?}"),
            }
        }
        panic!("original signed handoff must have a certified completion");
    });
    let handed_off =
        journal::observe_handoff(&issued, &observe(&pic, &plan, &stopped_source), &completion)
            .unwrap();
    store.save(&handed_off).unwrap();
    for _ in 0..2 {
        runtime
            .block_on(async {
                transport
                    .prepare_advance_root(&handed_off, source)
                    .await?
                    .submit()
                    .await
            })
            .unwrap();
        // Root progress stays readable after direct source authority has been removed.
        let reader = CapacityImportTransport::from_icp(&icp).unwrap();
        let retained = runtime.block_on(reader.root_status(&plan)).unwrap();
        assert_eq!(retained.reservation.plan_sha256, plan.plan_sha256);
    }
    // Drop the host after Root has normalized controllers. Completion resumes using Root
    // receipts, publishes generator inputs and releases allocation under the retained review.
    drop(store);
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    let digest = handed_off.operation.as_ref().unwrap().review.review_sha256;
    let completed = runtime
        .block_on(
            canic_host::fleet_ensure::workflow::capacity_import::complete(
                &store, &paths, digest, &icp,
            ),
        )
        .unwrap();
    assert!(publication::completed(&completed));
    let settled = runtime.block_on(transport.root_status(&plan)).unwrap();
    assert!(matches!(settled.phase, PoolImportPhase::Released { .. }));
    assert!(settled.root_receipt.is_some());
    let PoolImportSourceProgress::Ready(receipt) = &settled.progress[0] else {
        panic!("Root cleared the exact supplied source");
    };
    assert_eq!(receipt.canister_id, source);
    assert!(
        pic.canister_status(source, Some(root))
            .unwrap()
            .module_hash
            .is_none()
    );
    assert_eq!(
        runtime
            .block_on(async {
                transport
                    .prepare_advance_root(&handed_off, source)
                    .await?
                    .submit()
                    .await
            })
            .unwrap(),
        settled
    );
    let before = pic.canister_status(source, Some(root)).unwrap().version;
    let replay_start = timings.lock().unwrap().len();
    let sink = Arc::clone(&timings);
    let unavailable = IcpCli::new("/nonexistent/capacity-import-icp", Some("local".into()))
        .with_timing_handler(move |event| sink.lock().unwrap().push(event));
    assert_eq!(
        runtime
            .block_on(
                canic_host::fleet_ensure::workflow::capacity_import::complete(
                    &store,
                    &paths,
                    digest,
                    &unavailable,
                )
            )
            .unwrap(),
        completed
    );
    assert_eq!(
        timings.lock().unwrap().len(),
        replay_start,
        "terminal replay performs no transport work"
    );
    assert_eq!(
        pic.canister_status(source, Some(root)).unwrap().version,
        before
    );
    let policy: toml::Value =
        toml::from_str(&std::fs::read_to_string(directory.join("policy.toml")).unwrap()).unwrap();
    let seed: toml::Value =
        toml::from_str(&std::fs::read_to_string(directory.join("seed.toml")).unwrap()).unwrap();
    assert_eq!(
        policy["fleet_subnet_roots"][0]["canister_pool"]["imports"],
        seed["roots"][0]["pool_imports"]
    );
    assert!(
        seed["roots"][0]["pool_imports"]
            .as_array()
            .unwrap()
            .contains(&toml::Value::String(source.to_text()))
    );
    drop(store);
    fresh_handoff(&pic, &paths, &icp, &runtime, &transport, &completed);
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    rejection::uncertified_rejection_keeps_original_request(
        &pic, &paths, &store, &icp, &runtime, &transport,
    );
    assert_import_timings(&timings.lock().unwrap(), root, source);
    pic.stop_live();
}

fn assert_import_timings(events: &[IcpRequestTiming], root: Principal, source: Principal) {
    let direct = events
        .iter()
        .filter(|event| match event.kind {
            IcpRequestKind::AgentQuery => matches!(
                event.method.as_deref(),
                Some(protocol::CANIC_ROOT_STATUS | protocol::CANIC_COORDINATOR_REGISTRY)
            ),
            IcpRequestKind::AgentUpdate | IcpRequestKind::AgentRequestStatus => true,
            _ => false,
        })
        .collect::<Vec<_>>();
    assert!(!direct.is_empty());
    for start in direct.iter().filter(|event| event.succeeded.is_none()) {
        let end = direct
            .iter()
            .find(|event| event.request_id == start.request_id && event.succeeded.is_some())
            .unwrap();
        assert_eq!(end.kind, start.kind);
        assert_eq!(end.target, start.target);
        assert_eq!(end.subject, start.subject);
        assert_eq!(end.method, start.method);
        assert!(start.parent_request_id.is_none() && end.parent_request_id.is_none());
    }
    assert!(direct.iter().any(|event| event.kind == IcpRequestKind::AgentQuery && event.succeeded == Some(false)));
    assert!(
        direct
            .iter()
            .any(|event| event.kind == IcpRequestKind::AgentUpdate
                && event.target.as_deref() == Some(root.to_text().as_str())
                && event.subject == Some(source)
                && event.succeeded == Some(true))
    );
    assert!(
        direct
            .iter()
            .any(|event| event.kind == IcpRequestKind::AgentUpdate
                && event.target.as_deref()
                    == Some(Principal::management_canister().to_text().as_str())
                && event.method.as_deref() == Some("update_settings")
                && event.subject == Some(source)
                && event.succeeded == Some(true))
    );
    assert!(
        direct
            .iter()
            .any(|event| event.kind == IcpRequestKind::AgentRequestStatus
                && event.subject == Some(source)
                && event.succeeded == Some(true))
    );
}

fn fresh_handoff(
    pic: &PocketIc,
    paths: &EnsurePaths,
    icp: &IcpCli,
    runtime: &tokio::runtime::Runtime,
    transport: &CapacityImportTransport,
    previous: &canic_host::fleet_ensure::model::capacity_import::CapacityImportJournalRecord,
) {
    let store = CapacityImportJournalStore::open(paths).unwrap();
    let root = previous.plan.authority.root;
    let operator = previous.plan.authority.operator;
    let source = pic.create_canister_on_subnet(None, None, pic.get_subnet(root).unwrap());
    pic.install_canister(source, b"\0asm\x01\0\0\0".to_vec(), Vec::new(), None);
    pic.stop_canister(source, None).unwrap();
    pic.set_controllers(source, None, vec![operator]).unwrap();
    let stopped = pic.canister_status(source, Some(operator)).unwrap();
    let context = runtime.block_on(transport.root_context(root)).unwrap();
    let plan = admission::review(pic, review(pic, root, &[source], operator, &context));
    let staged = publication::bind(
        paths,
        &journal::reviewed(plan.clone()).unwrap(),
        Path::new("policy.toml"),
        Path::new("seed.toml"),
    )
    .unwrap();
    let digest = staged.operation.as_ref().unwrap().review.review_sha256;
    store.stage_review(staged).unwrap();
    let expired_request = expiry::retain_expired_intent(&store, icp, runtime, transport);
    // Consume the second submission before a simulated process death. The exact
    // unsent signed request stays uncertain until certified expiry reconciliation.
    let spent =
        publication::reserve_submission(&store.read().unwrap().unwrap(), "0:handoff").unwrap();
    store.save(&spent).unwrap();
    let mut reader = CapacityImportLiveObserver::from_icp(icp).unwrap();
    assert!(matches!(
        runtime.block_on(canic_host::fleet_ensure::workflow::capacity_import::apply(
            &store,
            paths,
            digest,
            icp,
            &mut reader,
        )),
        Err(CapacityImportJournalError::BudgetExhausted { .. })
    ));
    let store = approve_attempt_continuation(paths, store);
    let completed = runtime
        .block_on(canic_host::fleet_ensure::workflow::capacity_import::apply(
            &store,
            paths,
            digest,
            icp,
            &mut reader,
        ))
        .unwrap();
    assert!(publication::completed(&completed));
    let handoff = &completed.handoffs[0];
    assert_eq!(handoff.retirements.len(), 1);
    assert_eq!(handoff.retirements[0].request.request_id, expired_request);
    assert_ne!(
        handoff.request.as_ref().unwrap().request_id,
        expired_request
    );
    assert_eq!(
        completed.operation.as_ref().unwrap().submissions["0:handoff"],
        3
    );
    let status = pic.canister_status(source, Some(root)).unwrap();
    assert!(status.module_hash.is_none());
    assert_eq!(status.version, stopped.version + 3);
    let mut controllers = status.settings.controllers;
    controllers.sort_unstable();
    assert_eq!(controllers, plan.final_controllers);
    let unavailable = IcpCli::new("/nonexistent/capacity-import-icp", Some("local".into()));
    // The original review is now archived and Root retains the newer operation.
    // Its local replay must not query Root or use the observation owner.
    let original_digest = previous.operation.as_ref().unwrap().review.review_sha256;
    assert_eq!(
        runtime
            .block_on(canic_host::fleet_ensure::workflow::capacity_import::apply(
                &store,
                paths,
                original_digest,
                &unavailable,
                &mut reader
            ))
            .unwrap(),
        *previous
    );
    assert_eq!(store.read().unwrap().unwrap(), completed);
}

fn approve_attempt_continuation(
    paths: &EnsurePaths,
    store: CapacityImportJournalStore,
) -> CapacityImportJournalStore {
    let interrupted = store.read().unwrap().unwrap();
    drop(store);
    let review = canic_host::fleet_ensure::workflow::attempt_recovery::review(
        &paths.workspace,
        "local",
        "capacity-import",
    )
    .unwrap();
    canic_host::fleet_ensure::workflow::attempt_recovery::apply(
        &paths.workspace,
        "local",
        "capacity-import",
        review.review_sha256,
    )
    .unwrap();
    let store = CapacityImportJournalStore::open(paths).unwrap();
    let continued = store.read().unwrap().unwrap();
    assert_eq!(continued.plan, interrupted.plan);
    assert_eq!(continued.handoffs, interrupted.handoffs);
    assert_eq!(continued.reservation, interrupted.reservation);
    assert_eq!(
        continued.operation.as_ref().unwrap().submissions["0:handoff"],
        2
    );
    store
}

fn retain_inputs(
    workspace: &Path,
    paths: &EnsurePaths,
    plan: &CapacityImportPlanRecord,
    pool: &CanisterPoolResponse,
) {
    let mut policy: toml::Value = toml::from_str(&generated_journey_policy(
        plan.authority.operator,
        plan.authority.subnet.into_principal(),
        1,
        pool.config.maximum_size as usize - 1,
        &workspace.join("canisters/audit/root_probe/canic.toml"),
    ))
    .unwrap();
    policy.as_table_mut().unwrap().insert(
        "recovery_controllers".into(),
        toml::Value::Array(
            plan.authority
                .recovery_controllers
                .iter()
                .map(|id| toml::Value::String(id.to_text()))
                .collect(),
        ),
    );
    let store = pool
        .entries
        .iter()
        .find(|entry| entry.status == canic::dto::pool::CanisterPoolAssetStatus::Store)
        .unwrap()
        .canister_id;
    let imports = pool
        .entries
        .iter()
        .filter(|entry| entry.canister_id != store)
        .map(|entry| entry.canister_id.to_text())
        .collect::<Vec<_>>();
    policy["fleet_subnet_roots"][0]["canister_pool"]
        .as_table_mut()
        .unwrap()
        .insert(
            "imports".into(),
            toml::Value::Array(imports.iter().cloned().map(toml::Value::String).collect()),
        );
    let seed = format!(
        r#"
schema_version = 1
fleet_id = "{}"
fresh_estate = false
coordinator = "{}"
management_creation_fee_cycles = "500B"
[[roots]]
placement_subnet = "{}"
root = "{}"
store = "{}"
pool_imports = [{}]
"#,
        plan.authority.fleet.fleet.fleet_id,
        plan.authority.coordinator,
        plan.authority.subnet.into_principal(),
        plan.authority.root,
        store,
        imports
            .iter()
            .map(|id| format!("\"{id}\""))
            .collect::<Vec<_>>()
            .join(",")
    );
    std::fs::write(
        paths.workspace.join("policy.toml"),
        toml::to_string_pretty(&policy).unwrap(),
    )
    .unwrap();
    std::fs::write(paths.workspace.join("seed.toml"), seed).unwrap();
}

pub(super) fn review(
    pic: &PocketIc,
    root: Principal,
    sources: &[Principal],
    operator: Principal,
    context: &PoolImportContext,
) -> CapacityImportPlanRecord {
    let maximum_paid_calls = pool_import::recommended_calls(sources.len()).unwrap();
    let root_status = pic.canister_status(root, Some(operator)).unwrap();
    prepare_review(
        CapacityImportAuthority {
            fleet: context.binding.authority.binding.fleet.clone(),
            network_root_key_sha256: wasm_hash(&pic.root_key().unwrap()).try_into().unwrap(),
            operator,
            coordinator: context.binding.authority.binding.coordinator,
            root,
            subnet: context.binding.placement_subnet,
            root_authority_sha256: context.root_authority_sha256,
            import_sequence: context.next_sequence,
            recovery_controllers: context
                .binding
                .authority
                .binding
                .recovery_controllers
                .clone(),
        },
        sources
            .iter()
            .map(|&source| {
                let source_status = pic.canister_status(source, Some(operator)).unwrap();
                CapacityImportSourceRecord {
                    binding: CapacityImportSourceBinding {
                        canister_id: source,
                        subnet: context.binding.placement_subnet,
                        controllers: source_status.settings.controllers,
                        module_sha256: source_status
                            .module_hash
                            .map(|hash| hash.try_into().unwrap()),
                        canister_version: source_status.version,
                        stopped: true,
                        snapshots_size_bytes: source_status
                            .memory_metrics
                            .snapshots_size
                            .0
                            .try_into()
                            .unwrap(),
                    },
                    disposition: CapacityImportDisposition::AbsenceEvidence {
                        evidence_sha256: [0x43; 32],
                    },
                    observed_cycles: source_status.cycles.0.try_into().unwrap(),
                    observed_reserved_cycles: source_status.reserved_cycles.0.try_into().unwrap(),
                    minimum_ready_cycles: context
                        .binding
                        .limits
                        .canister_pool
                        .canister_cycles
                        .to_u128(),
                    maximum_debit_cycles: 1_000_000_000_000,
                }
            })
            .collect(),
        CapacityImportRootBudget {
            observed_cycles: root_status.cycles.0.try_into().unwrap(),
            observed_reserved_cycles: root_status.reserved_cycles.0.try_into().unwrap(),
            minimum_retained_cycles: context
                .binding
                .funding
                .root_funding
                .request_threshold
                .to_u128(),
            maximum_debit_cycles: pool_import::required_debit(
                context.maximum_call_debit_cycles,
                maximum_paid_calls,
            )
            .unwrap(),
            maximum_paid_calls,
        },
    )
    .unwrap()
}

fn observe(
    pic: &PocketIc,
    plan: &CapacityImportPlanRecord,
    stopped: &ic_testkit::pic::CanisterStatusResult,
) -> CapacityImportSourceView {
    let source = &plan.sources[0];
    let observed = pic
        .canister_status(source.binding.canister_id, Some(plan.authority.operator))
        .unwrap();
    let mut controllers = observed.settings.controllers;
    controllers.sort_unstable();
    CapacityImportSourceView {
        binding: CapacityImportSourceBinding {
            canister_id: source.binding.canister_id,
            subnet: canic::ids::SubnetId::from_principal(
                pic.get_subnet(source.binding.canister_id).unwrap(),
            ),
            controllers,
            module_sha256: observed.module_hash.map(|hash| hash.try_into().unwrap()),
            canister_version: observed.version,
            stopped: observed.status == stopped.status,
            snapshots_size_bytes: observed.memory_metrics.snapshots_size.0.try_into().unwrap(),
        },
        cycles: observed.cycles.0.try_into().unwrap(),
        reserved_cycles: observed.reserved_cycles.0.try_into().unwrap(),
        ownership: CapacityImportOwnershipView::Unassigned,
        disposition_evidence_sha256: Some([0x43; 32]),
    }
}
