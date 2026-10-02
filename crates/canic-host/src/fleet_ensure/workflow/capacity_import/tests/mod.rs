//! Query failures preserve durable import allowances; paid effects are qualified in PocketIC.

mod server;

use super::*;
use crate::fleet_ensure::{
    generate::capacity_import::tests::inputs,
    model::capacity_import::{CapacityImportPlanRecord, CapacityImportReservationRecord},
    ops::capacity_import::{
        journal, prepare_review,
        transport::tests::{
            awaiting_handoff_status, completion, request, root_context, with_agent_and_icp,
        },
    },
    policy::capacity_import::tests::{destination, plan, sources},
};
use ic_agent::{Agent, Identity, identity::BasicIdentity};
use sha2_host::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[test]
fn failed_authority_queries_preserve_root_submission_allowances_across_reopen() {
    let signer = BasicIdentity::from_raw_key(&[7; 32]);
    let probe = Agent::builder()
        .with_url("http://127.0.0.1:1")
        .build()
        .unwrap();
    let initial = plan();
    let mut authority = initial.authority;
    let previous_operator = authority.operator;
    authority.operator = signer.sender().unwrap();
    authority.network_root_key_sha256 = Sha256::digest(probe.read_root_key()).into();
    let mut sources = initial.sources;
    for source in &mut sources {
        for controller in &mut source.binding.controllers {
            if *controller == previous_operator {
                *controller = authority.operator;
            }
        }
    }
    let plan = prepare_review(authority, sources, initial.root_budget).unwrap();
    let coordinator = plan.authority.coordinator;
    let server = server::QueryFailureServer::start(
        plan.authority.root,
        awaiting_handoff_status(&plan),
        root_context(),
    );
    // Only query failure routing is simulated here. Real authentication and
    // management effects remain covered by the signed-handoff PocketIC case.
    let agent = Agent::builder()
        .with_url(&server.url)
        .with_identity(signer)
        .with_verify_query_signatures(false)
        .build()
        .unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);
    let icp = IcpCli::new("unused", None)
        .with_timing_handler(move |event| sink.lock().unwrap().push(event));
    let transport = with_agent_and_icp(agent, icp);
    let (directory, paths, store) = approved_journal(plan);
    let original = store.read().unwrap().unwrap();
    drop(store);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    // More failures than the two submitted-update allowances, with an actual
    // journal reopen each time: no query failure may consume either allowance.
    for _ in 0..3 {
        let store = CapacityImportJournalStore::open(&paths).unwrap();
        let mut record = store.read().unwrap().unwrap();
        let result = runtime.block_on(finish_root(&store, &mut record, &transport));
        assert!(
            matches!(
                result,
                Err(CapacityImportJournalError::CoordinatorUnavailable { coordinator: observed })
                    if observed == coordinator
            ),
            "unexpected preflight result: {result:?}"
        );
        assert_eq!(record, original);
        assert_eq!(store.read().unwrap().unwrap(), original);
    }
    let events = events.lock().unwrap();
    let starts = events
        .iter()
        .filter(|event| event.succeeded.is_none())
        .collect::<Vec<_>>();
    assert_eq!(
        starts.len(),
        9,
        "each of three attempts observes Root status, context and Coordinator"
    );
    for start in starts {
        assert_eq!(start.kind, crate::icp::IcpRequestKind::AgentQuery);
        assert!(start.subject.is_none() && start.parent_request_id.is_none());
        let end = events
            .iter()
            .find(|event| event.request_id == start.request_id && event.succeeded.is_some())
            .unwrap();
        assert_eq!(end.target, start.target);
        assert_eq!(end.method, start.method);
        assert_eq!(
            end.succeeded,
            Some(start.target.as_deref() != Some(coordinator.to_text().as_str()))
        );
    }
    drop(events);
    server.finish();
    std::fs::remove_dir_all(directory).unwrap();
}

fn approved_journal(
    plan: CapacityImportPlanRecord,
) -> (PathBuf, EnsurePaths, CapacityImportJournalStore) {
    let (directory, paths, store) = reviewed_journal(plan.clone());
    let record = store.read().unwrap().unwrap();
    let mut record = journal::approve(
        &record,
        plan.plan_sha256,
        &destination(&plan),
        &sources(&plan),
    )
    .unwrap();
    store.save(&record).unwrap();
    record = journal::reserve(
        &record,
        CapacityImportReservationRecord {
            plan_sha256: plan.plan_sha256,
            authority: plan.authority.clone(),
            sources: plan
                .sources
                .iter()
                .map(|source| source.binding.canister_id)
                .collect(),
        },
    )
    .unwrap();
    store.save(&record).unwrap();
    for mut observed in sources(&plan) {
        let id = observed.binding.canister_id;
        let signed = request(&plan, id);
        record = journal::prepare_handoff(&record, &observed, signed.clone()).unwrap();
        store.save(&record).unwrap();
        record = journal::issue_handoff(&record, id).unwrap();
        store.save(&record).unwrap();
        observed
            .binding
            .controllers
            .clone_from(&plan.transitional_controllers);
        observed.binding.canister_version += 1;
        record =
            journal::observe_handoff(&record, &observed, &completion(&plan, id, &signed)).unwrap();
        store.save(&record).unwrap();
    }
    assert!(journal::all_custody_ready(&record));
    (directory, paths, store)
}

pub(super) fn reviewed_journal(
    plan: CapacityImportPlanRecord,
) -> (PathBuf, EnsurePaths, CapacityImportJournalStore) {
    let directory = crate::test_support::temp_dir("capacity-import-preflight");
    std::fs::create_dir_all(&directory).unwrap();
    let paths = EnsurePaths::under(&directory, "staging", "test-fleet");
    let (policy, seed) = inputs(&plan);
    std::fs::write(
        directory.join("policy.toml"),
        policy.replace("maximum_size = 3", "maximum_size = 4"),
    )
    .unwrap();
    std::fs::write(directory.join("seed.toml"), seed).unwrap();
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    let record = publication::bind(
        &paths,
        &journal::reviewed(plan).unwrap(),
        Path::new("policy.toml"),
        Path::new("seed.toml"),
    )
    .unwrap();
    store.stage_review(record).unwrap();
    (directory, paths, store)
}
