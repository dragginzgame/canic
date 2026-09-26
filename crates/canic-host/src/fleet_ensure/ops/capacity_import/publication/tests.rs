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
    let mut journal = reserved;
    for mut observed in sources(plan) {
        let id = observed.binding.canister_id;
        let request = request(plan, id);
        journal = journal::prepare_handoff(&journal, &observed, request.clone()).unwrap();
        store.save(&journal).unwrap();
        journal = journal::issue_handoff(&journal, id).unwrap();
        store.save(&journal).unwrap();
        observed
            .binding
            .controllers
            .clone_from(&plan.transitional_controllers);
        observed.binding.canister_version += 1;
        journal =
            journal::observe_handoff(&journal, &observed, &completion(plan, id, &request)).unwrap();
        store.save(&journal).unwrap();
    }
    journal = retain_settled(&journal, &settled(plan)).unwrap();
    store.save(&journal).unwrap();
    journal
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
