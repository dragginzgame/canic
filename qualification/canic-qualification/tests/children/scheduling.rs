//! A full execution window fences only its assigned children.
use super::*;
use ic_testkit::held::{Catalog, Fixture, Limits as HeldLimits, Reply};

#[tokio::test]
async fn queued_children_survive_upgrade_and_advance_while_other_pins_are_held() {
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(180),
            stdout_bytes: 1_048_576,
            stderr_bytes: 1_048_576,
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
    let wasm = artifact("root.wasm");
    pic.install_canister(root, wasm.clone(), vec![], None).await;
    pic.add_cycles(root, 300_000_000_000_000).await;
    let store = ChildStore::install(pic, root, artifact("store.wasm")).await;
    let blocker = pic.create_canister().await;
    let replies = std::array::from_fn::<_, 4, _>(|_| Reply {
        method: "pin",
        bytes: &[],
    });
    let catalog = Catalog::new(
        &replies,
        HeldLimits {
            slots: 4,
            name_bytes: 16,
            reply_bytes: 32,
        },
    )
    .unwrap();
    let mut held = Fixture::new(
        pic,
        blocker.to_text().parse().unwrap(),
        &catalog,
        100_000_000_000,
        32768,
    )
    .await
    .unwrap();
    pic.add_cycles(blocker, 2_000_000_000_000).await;
    pic.install_canister(blocker, held.wasm().to_vec(), vec![], None)
        .await;

    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let original = input(root, artifact("app.wasm"), now);
    let method = application::STAGE_CHILD_CREATION;
    let frame = |direction| ic_wire::Encoding::Frame { method, direction };
    let request = frame(ic_wire::Direction::Request);
    let bytes = store
        .frame(method, ChildId::FIRST, &original.approval, &original.wasm)
        .await;
    let mut stage: Request<application::Creation> =
        ic_wire::decode(&bytes, request, application::LIMITS).unwrap();
    let application::Creation::Stored(source) = &stage.value else {
        unreachable!()
    };
    let real = source.reference.store.clone();
    for index in 0..70 {
        stage.slot = ChildId::service(index).unwrap();
        let application::Creation::Stored(source) = &mut stage.value else {
            unreachable!()
        };
        source.reference.store = if index < 4 {
            blocker.to_text()
        } else {
            real.clone()
        };
        if index >= 5 {
            source.approval.expires_at_ns = now + 16_000_000_000;
        }
        let bytes = ic_wire::encode(&stage, request, application::LIMITS).unwrap();
        let reply = pic
            .update_call(root, CanisterId::anonymous(), "stage_child_creation", bytes)
            .await
            .unwrap();
        let value: Result<Status, Refusal> =
            ic_wire::decode(&reply, frame(ic_wire::Direction::Response), LIMITS).unwrap();
        assert_eq!(value.unwrap().phase, Phase::Scheduled);
    }
    canic_qualification::upgrade_stopped(pic, root, wasm, vec![], None)
        .await
        .unwrap();
    pic.advance_time(Duration::from_secs(11)).await;
    for _ in 0..40 {
        pic.tick().await;
    }
    let pending = held.pending(pic).await.unwrap();
    assert_eq!(pending.len(), 4);
    let healthy = ChildId::service(4).unwrap();
    assert_eq!(creation_status(pic, root, healthy).await.attempts, [0; 3]);
    pic.advance_time(Duration::from_secs(8)).await;
    held.release(pic, &pending[0]).await.unwrap();
    for _ in 0..180 {
        pic.tick().await;
    }
    let completed = creation_status(pic, root, healthy).await;
    assert_eq!(
        (completed.phase, completed.attempts),
        (Phase::Ready, [1; 3])
    );
    let tail = creation_status(pic, root, ChildId::service(69).unwrap()).await;
    assert_eq!((tail.phase, tail.attempts), (Phase::Expired, [0; 3]));
    let pending = held.pending(pic).await.unwrap();
    assert_eq!(pending.len(), 3);
    for request in &pending {
        held.release(pic, request).await.unwrap();
    }
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}
