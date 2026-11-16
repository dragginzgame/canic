//! Identified storage preserves pending authority and refuses an incompatible predecessor.
use super::*;

#[tokio::test]
async fn identified_storage_migrates_pending_children_and_refuses_downgrade() {
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(180),
            stdout_bytes: 1024 * 1024,
            stderr_bytes: 1024 * 1024,
        },
    )
    .unwrap();
    let owner = server
        .instance(
            PocketIcBuilder::new().with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    let pic = owner.client();
    let old = std::fs::read(
        repo().join("qualification/canic-qualification/fixtures/root-positional-children.wasm"),
    )
    .unwrap();
    let hash: String = digest(&old).iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hash,
        "a00ce6fcac8687c9c350e317c53d2d653665c1c531639af9d07961ade96cc90d"
    );
    let root = pic.create_canister().await;
    pic.install_canister(root, old.clone(), vec![], None).await;
    pic.add_cycles(root, 10_000_000_000_000).await;
    let mut approvals = Vec::new();
    // Admission order must never decide identity, including while no principal exists.
    for (slot, name) in [
        (ChildId::SECOND, "app-v2.wasm"),
        (ChildId::FIRST, "app.wasm"),
    ] {
        let stage = input(
            root,
            artifact(name),
            pic.get_time().await.as_nanos_since_unix_epoch(),
        );
        assert_eq!(
            admit(pic, root, slot, stage.clone()).await.unwrap().phase,
            Phase::Scheduled
        );
        approvals.push((slot, stage));
    }
    let scheduled = read(pic, root).await;
    let stable = pic.get_stable_memory(root).await;
    canic_qualification::upgrade_stopped(pic, root, artifact("root.wasm"), vec![], None)
        .await
        .unwrap();
    let migrated = pic.get_stable_memory(root).await;
    assert_ne!(migrated, stable);
    assert_eq!(migrated.len(), stable.len(), "reuse the checked backing");
    assert_eq!(read(pic, root).await, scheduled);
    for (slot, stage) in &approvals {
        assert_eq!(
            admit(pic, root, *slot, stage.clone()).await.unwrap().phase,
            Phase::Scheduled
        );
    }
    assert_eq!(read(pic, root).await, scheduled);
    assert_eq!(pic.get_stable_memory(root).await, migrated);
    // Unsupported code must trap at load, leaving the migrated state and new code intact.
    assert!(
        canic_qualification::upgrade_stopped(pic, root, old.clone(), vec![], None)
            .await
            .is_err()
    );
    assert_eq!(pic.get_stable_memory(root).await, migrated);
    assert_eq!(read(pic, root).await, scheduled);
    assert_eq!(
        pic.canister_status(root, None).await.unwrap().module_hash,
        Some(digest(&artifact("root.wasm")).to_vec())
    );
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    let completed = read(pic, root).await;
    for (slot, stage) in &approvals {
        let child = &completed[slot.index()].creation;
        assert_eq!(child.phase, Phase::Ready);
        assert_eq!(child.approval, Some(stage.approval.clone()));
        assert_eq!(child.attempts, [1; 3]);
    }
    assert_ne!(completed[0].creation.worker, completed[1].creation.worker);
    let stable = pic.get_stable_memory(root).await;
    assert!(
        canic_qualification::upgrade_stopped(pic, root, old, vec![], None)
            .await
            .is_err()
    );
    assert_eq!(pic.get_stable_memory(root).await, stable);
    // Compatible executables reopen the collection without another rewrite or reset.
    for executable in [artifact("maintenance.wasm"), artifact("root.wasm")] {
        canic_qualification::upgrade_stopped(pic, root, executable, vec![], None)
            .await
            .unwrap();
        assert_eq!(pic.get_stable_memory(root).await, stable);
        assert_eq!(read(pic, root).await, completed);
    }
    let subnet = pic.get_subnet(root).await.unwrap();
    assert_eq!(
        pic.get_subnet_metrics(subnet).await.unwrap().num_canisters,
        3
    );
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
