//! The identified runtime preserves bytes written by the positional predecessor.
use super::*;

#[tokio::test]
async fn identified_runtime_preserves_positional_journal_bytes() {
    let owner = PocketIcStartupConfig::Spawn {
        server_binary: server_assets::check(&canic_qualification::server_directory()).unwrap(),
        timeout: Duration::from_secs(30),
        server_hard_ttl: Some(Duration::from_secs(180)),
        server_idle_ttl: None,
        server_output_files: None,
    }
    .build_instance(PocketIcBuilder::new().with_application_subnet())
    .await
    .unwrap();
    let pic = owner.instance();
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
    for (slot, name) in [(Slot::Second, "app-v2.wasm"), (Slot::First, "app.wasm")] {
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
    pic.upgrade_canister(root, artifact("root.wasm"), vec![], None)
        .await
        .unwrap();
    assert_eq!(pic.get_stable_memory(root).await, stable);
    assert_eq!(read(pic, root).await, scheduled);
    for (slot, stage) in &approvals {
        assert_eq!(
            admit(pic, root, *slot, stage.clone()).await.unwrap().phase,
            Phase::Scheduled
        );
    }
    assert_eq!(read(pic, root).await, scheduled);
    assert_eq!(pic.get_stable_memory(root).await, stable);
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
    // The same format remains readable in both directions, without migration or reset.
    for executable in [old, artifact("root.wasm")] {
        pic.upgrade_canister(root, executable, vec![], None)
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
    owner.close().await;
}
