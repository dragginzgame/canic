//! Recover partial paired publication without repeating remote effects or overwriting edits.

use super::*;
use crate::fleet_ensure::{
    generate::capacity_import::tests::inputs,
    model::capacity_import::CapacityImportReservationRecord,
    ops::capacity_import::{
        evidence::tests::settled,
        journal,
        transport::tests::{completion, request},
    },
    policy::capacity_import::tests::{destination, plan, sources},
};

fn fixture() -> (PathBuf, EnsurePaths, CapacityImportJournalStore) {
    let directory = crate::test_support::temp_dir("capacity-import-publication");
    std::fs::create_dir_all(&directory).unwrap();
    let paths = EnsurePaths::under(directory.as_path(), "staging", "test-fleet");
    let plan = plan();
    let (policy, seed) = inputs(&plan);
    let policy = policy.replace("maximum_size = 3", "maximum_size = 4");
    std::fs::write(directory.as_path().join("policy.toml"), policy).unwrap();
    std::fs::write(directory.as_path().join("seed.toml"), seed).unwrap();
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    let journal = bind(
        &paths,
        &journal::reviewed(plan).unwrap(),
        Path::new("policy.toml"),
        Path::new("seed.toml"),
    )
    .unwrap();
    store.stage_review(journal).unwrap();
    (directory, paths, store)
}

fn ready(store: &CapacityImportJournalStore) -> CapacityImportJournalRecord {
    let reserved = reserved(store);
    let plan = reserved.plan.clone();
    let mut journal = reserved;
    for mut observed in sources(&plan) {
        let id = observed.binding.canister_id;
        let request = request(&plan, id);
        journal = journal::prepare_handoff(&journal, &observed, request.clone()).unwrap();
        store.save(&journal).unwrap();
        journal = journal::issue_handoff(&journal, id).unwrap();
        store.save(&journal).unwrap();
        observed
            .binding
            .controllers
            .clone_from(&plan.transitional_controllers);
        observed.binding.canister_version += 1;
        journal = journal::observe_handoff(&journal, &observed, &completion(&plan, id, &request))
            .unwrap();
        store.save(&journal).unwrap();
    }
    journal = retain_settled(&journal, &settled(&plan)).unwrap();
    store.save(&journal).unwrap();
    journal
}

fn reserved(store: &CapacityImportJournalStore) -> CapacityImportJournalRecord {
    let journal = store.read().unwrap().unwrap();
    let plan = &journal.plan;
    let approved = journal::approve(
        &journal,
        plan.plan_sha256,
        &destination(plan),
        &sources(plan),
    )
    .unwrap();
    store.save(&approved).unwrap();
    let reserved = journal::reserve(
        &approved,
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
    store.save(&reserved).unwrap();
    reserved
}

#[test]
fn partial_publication_recovers_exact_pair_after_reopening() {
    let (directory, paths, store) = fixture();
    let mut journal = ready(&store);
    let operation = journal.operation.as_mut().unwrap();
    operation.publication_started = true;
    std::fs::write(
        paths.workspace.join("policy.toml"),
        &operation.review.policy.replacement,
    )
    .unwrap();
    store.save(&journal).unwrap();
    drop(store);
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    let published = publish(&store, &paths).unwrap();
    let operation = published.operation.as_ref().unwrap();
    assert!(operation.publication_complete);
    for document in [&operation.review.policy, &operation.review.seed] {
        assert_eq!(
            std::fs::read_to_string(paths.workspace.join(&document.relative_path)).unwrap(),
            document.replacement
        );
    }
    assert_eq!(publish(&store, &paths).unwrap(), published);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn unrelated_seed_edit_prevents_either_publication_write() {
    let (directory, paths, store) = fixture();
    let journal = ready(&store);
    std::fs::write(paths.workspace.join("seed.toml"), "# unrelated edit\n").unwrap();
    assert!(matches!(
        publish(&store, &paths),
        Err(CapacityImportJournalError::PublicationConflict)
    ));
    assert_eq!(
        std::fs::read_to_string(paths.workspace.join("policy.toml")).unwrap(),
        journal.operation.unwrap().review.policy.original
    );
    assert!(
        !store
            .read()
            .unwrap()
            .unwrap()
            .operation
            .unwrap()
            .publication_started
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn terminal_release_binds_publication_and_replay_preserves_later_edits() {
    let (directory, paths, store) = fixture();
    let ready = ready(&store);
    let mut status = settled(&ready.plan);
    status.phase = PoolImportPhase::Released {
        publication_sha256: publication_digest(&ready).unwrap(),
    };
    assert!(matches!(
        retain_released(&ready, &status),
        Err(CapacityImportJournalError::PublicationConflict)
    ));
    let published = publish(&store, &paths).unwrap();
    let released = retain_released(&published, &status).unwrap();
    store.save(&released).unwrap();
    assert!(completed(&released));
    std::fs::write(
        paths.workspace.join("policy.toml"),
        "# later operator edit\n",
    )
    .unwrap();
    assert_eq!(publish(&store, &paths).unwrap(), released);
    assert_eq!(
        std::fs::read_to_string(paths.workspace.join("policy.toml")).unwrap(),
        "# later operator edit\n"
    );
    assert!(store.save(&published).is_err());
    status.phase = PoolImportPhase::Released {
        publication_sha256: [77; 32],
    };
    assert!(matches!(
        retain_released(&published, &status),
        Err(CapacityImportJournalError::PublicationConflict)
    ));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn review_attempt_budgets_survive_reopen_and_cannot_decrease() {
    let (directory, paths, store) = fixture();
    let mut journal = ready(&store);
    let original = journal.clone();
    for _ in 0..MAX_SUBMISSIONS {
        journal = reserve_submission(&journal, "release").unwrap();
        store.save(&journal).unwrap();
    }
    assert!(store.save(&original).is_err());
    drop(store);
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    let journal = store.read().unwrap().unwrap();
    assert!(matches!(
        reserve_submission(&journal, "release"),
        Err(CapacityImportJournalError::BudgetExhausted { .. })
    ));
    let id = journal.plan.sources[0].binding.canister_id;
    let mut journal = journal;
    for _ in 0..MAX_INSPECTIONS {
        journal = reserve_inspection(&journal, id).unwrap();
        store.save(&journal).unwrap();
    }
    assert!(matches!(
        reserve_inspection(&journal, id),
        Err(CapacityImportJournalError::BudgetExhausted { .. })
    ));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn archived_completion_replays_locally_while_next_import_is_staged() {
    let (directory, paths, store) = fixture();
    let journal = ready(&store);
    let published = publish(&store, &paths).unwrap();
    let mut status = settled(&journal.plan);
    status.phase = PoolImportPhase::Released {
        publication_sha256: publication_digest(&published).unwrap(),
    };
    let released = retain_released(&published, &status).unwrap();
    store.save(&released).unwrap();
    let original_bytes = std::fs::read(paths.plan.with_file_name("capacity-import.json")).unwrap();
    let digest = released.operation.as_ref().unwrap().review.review_sha256;
    let mut authority = journal.plan.authority.clone();
    authority.import_sequence += 1;
    let mut candidates = journal.plan.sources.clone();
    candidates[0].binding.canister_id =
        crate::fleet_ensure::policy::capacity_import::tests::principal(90);
    let next = crate::fleet_ensure::ops::capacity_import::prepare_review(
        authority,
        candidates,
        journal.plan.root_budget,
    )
    .unwrap();
    let next = bind(
        &paths,
        &journal::reviewed(next).unwrap(),
        Path::new("policy.toml"),
        Path::new("seed.toml"),
    )
    .unwrap();
    store.stage_review(next.clone()).unwrap();
    let archive = paths
        .plan
        .with_file_name("capacity-import-history")
        .join(format!("{}.json", hex_bytes(digest)));
    assert_eq!(std::fs::read(archive).unwrap(), original_bytes);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let unavailable =
        crate::icp::IcpCli::new("/nonexistent/capacity-import-icp", Some("local".into()));
    assert_eq!(
        runtime
            .block_on(crate::fleet_ensure::workflow::capacity_import::complete(
                &store,
                &paths,
                digest,
                &unavailable
            ))
            .unwrap(),
        released
    );
    assert_eq!(store.read().unwrap().unwrap(), next);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn reviewed_host_continuation_preserves_root_caps_and_submission_consumption() {
    let (directory, paths, store) = fixture();
    let mut retained = ready(&store);
    for _ in 0..MAX_SUBMISSIONS {
        retained = reserve_submission(&retained, "release").unwrap();
        store.save(&retained).unwrap();
    }
    let id = retained.plan.authority.root;
    for _ in 0..MAX_INSPECTIONS {
        retained = reserve_inspection(&retained, id).unwrap();
        store.save(&retained).unwrap();
    }
    let original = retained.clone();
    drop(store);
    let recovery = crate::fleet_ensure::workflow::attempt_recovery::review(
        &directory,
        "staging",
        "test-fleet",
    )
    .unwrap();
    crate::fleet_ensure::workflow::attempt_recovery::apply(
        &directory,
        "staging",
        "test-fleet",
        recovery.review_sha256,
    )
    .unwrap();
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    retained = store.read().unwrap().unwrap();
    assert_eq!(retained.plan, original.plan);
    assert_eq!(retained.handoffs, original.handoffs);
    assert_eq!(retained.reservation, original.reservation);
    assert_eq!(
        retained.operation.as_ref().unwrap().review,
        original.operation.as_ref().unwrap().review
    );
    assert_eq!(
        retained.operation.as_ref().unwrap().submissions["release"],
        2
    );
    assert_eq!(
        retained.operation.as_ref().unwrap().inspections[&id.to_text()],
        4
    );
    for _ in 0..2 {
        retained = reserve_submission(&retained, "release").unwrap();
        retained = reserve_inspection(&retained, id).unwrap();
        store.save(&retained).unwrap();
    }
    assert!(matches!(
        reserve_submission(&retained, "release"),
        Err(CapacityImportJournalError::BudgetExhausted { .. })
    ));
    assert!(matches!(
        reserve_inspection(&retained, id),
        Err(CapacityImportJournalError::BudgetExhausted { .. })
    ));
    drop(store);
    let bytes = std::fs::read(paths.plan.with_file_name("capacity-import.json")).unwrap();
    crate::fleet_ensure::workflow::attempt_recovery::apply(
        &directory,
        "staging",
        "test-fleet",
        recovery.review_sha256,
    )
    .unwrap();
    assert_eq!(
        std::fs::read(paths.plan.with_file_name("capacity-import.json")).unwrap(),
        bytes
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn rejected_envelope_continuation_keeps_exact_history_and_original_balances() {
    use crate::fleet_ensure::ops::capacity_import::{
        journal::retirement,
        transport::tests::{rejected, request_with_expiry},
    };
    let (directory, paths, store) = fixture();
    let mut record = reserved(&store);
    let source = sources(&record.plan).remove(0);
    let id = source.binding.canister_id;
    for expiry in 1..=2 {
        let request = request_with_expiry(&record.plan, id, expiry);
        record = if expiry == 1 {
            journal::prepare_handoff(&record, &source, request).unwrap()
        } else {
            retirement::renew(&record, &source, request).unwrap()
        };
        store.save(&record).unwrap();
        record = journal::issue_handoff(&record, id).unwrap();
        store.save(&record).unwrap();
        record = retirement::retain(
            &record,
            id,
            &rejected(
                &record.plan,
                id,
                record.handoffs[0].request.as_ref().unwrap(),
            ),
        )
        .unwrap();
        store.save(&record).unwrap();
    }
    let request = request_with_expiry(&record.plan, id, 3);
    assert!(matches!(
        retirement::renew(&record, &source, request.clone()),
        Err(CapacityImportJournalError::BudgetExhausted { .. })
    ));
    drop(store);
    let review = crate::fleet_ensure::workflow::attempt_recovery::review(
        &directory,
        "staging",
        "test-fleet",
    )
    .unwrap();
    assert_eq!(
        review.owners[0].grants[0].resource,
        format!("handoff_envelopes:{id}")
    );
    crate::fleet_ensure::workflow::attempt_recovery::apply(
        &directory,
        "staging",
        "test-fleet",
        review.review_sha256,
    )
    .unwrap();
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    let continued = store.read().unwrap().unwrap();
    let renewed = retirement::renew(&continued, &source, request).unwrap();
    store.save(&renewed).unwrap();
    assert_eq!(renewed.plan, record.plan);
    assert_eq!(renewed.reservation, record.reservation);
    assert_eq!(
        renewed.handoffs[0].retirements,
        record.handoffs[0].retirements
    );
    assert_eq!(
        renewed.handoffs[0].effect.as_ref().unwrap().pre_cycles,
        record.handoffs[0].effect.as_ref().unwrap().pre_cycles
    );
    assert_eq!(
        renewed.handoffs[0].before_reserved_cycles,
        record.handoffs[0].before_reserved_cycles
    );
    drop(store);
    crate::fleet_ensure::workflow::attempt_recovery::apply(
        &directory,
        "staging",
        "test-fleet",
        review.review_sha256,
    )
    .unwrap();
    let store = CapacityImportJournalStore::open(&paths).unwrap();
    assert_eq!(store.read().unwrap().unwrap(), renewed);
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}
