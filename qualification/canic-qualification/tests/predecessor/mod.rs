//! The old allocator Root is preserved when a new records executable refuses its image.
use super::*;

#[tokio::test]
async fn retained_predecessor_maintenance_refuses_old_storage_without_reset() {
    for completed in [false, true] {
        qualify_predecessor(completed).await;
    }
}

async fn qualify_predecessor(completed: bool) {
    let original = std::fs::read(
        repo().join("qualification/canic-qualification/fixtures/root-single-upgrade.wasm"),
    )
    .unwrap();
    let hash: String = digest(&original)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        hash,
        "e4a4127abd525f59378503dcdb98fcb13f8eb9895673d4d8a4dea41d85e24296"
    );
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(240),
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
    let root = pic.create_canister().await;
    pic.install_canister(root, original.clone(), vec![], None)
        .await;
    let input = children::input(
        root,
        children::artifact("app.wasm"),
        pic.get_time().await.as_nanos_since_unix_epoch(),
    );
    storage_reopen::historical_stage(pic, root, &input)
        .await
        .unwrap();
    if completed {
        pic.advance_time(Duration::from_secs(11)).await;
        settle(pic).await;
    }
    let retained = storage_reopen::historical_status(pic, root).await;
    assert_eq!(
        retained.phase,
        if completed {
            Phase::Ready
        } else {
            Phase::Scheduled
        }
    );
    let stable = pic.get_stable_memory(root).await;
    canic_qualification::upgrade_stopped(pic, root, children::artifact("root.wasm"), vec![], None)
        .await
        .expect_err("old inline custody must stay with its original Root");
    assert_eq!(pic.get_stable_memory(root).await, stable);
    assert_eq!(storage_reopen::historical_status(pic, root).await, retained);
    assert_eq!(
        pic.canister_status(root, None).await.unwrap().module_hash,
        Some(digest(&original).to_vec())
    );
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}
