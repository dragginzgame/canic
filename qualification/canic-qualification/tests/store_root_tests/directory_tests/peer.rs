//! One real sender and receiver under the same installing Root.
use super::*;
use canic::topology::peer::{ApplyPeer, Decision, Status};
use ic_testkit::held::{Catalog, Fixture, Limits, Reply};

pub(super) async fn exercise(root: &Client<'_>, accounts: CanisterId, project: CanisterId) {
    let pic = root.pic;
    let sender = Client {
        pic,
        id: project,
        operator: root.id,
    };
    let receiver = Client {
        pic,
        id: accounts,
        operator: root.operator,
    };
    let change = async |generation: u64, allow: bool| {
        root.update::<_, Result<Status, String>>(
            "change_peer",
            ic_wire::method_id("change_peer"),
            &(
                "project_hub",
                "accounts_hub",
                "project_sync",
                generation,
                allow,
            ),
            canic::method::LIMITS,
        )
        .await
    };
    let sync = async || {
        sender
            .update::<_, Result<u64, String>>(
                "send_sync",
                ic_wire::method_id("send_sync"),
                &None::<CanisterId>,
                canic::method::LIMITS,
            )
            .await
    };
    assert_eq!(sync().await, Err("peer denied".into()));
    assert_eq!(change(1, true).await, Ok(Status::Applied));
    assert_eq!(sync().await, Ok(1));
    assert_eq!(
        receiver
            .update::<_, Result<u64, String>>(
                "project_sync",
                ic_wire::method_id("project_sync"),
                &None::<CanisterId>,
                canic::method::LIMITS
            )
            .await,
        Err("peer denied".into())
    );

    pic.stop_canister(accounts, Some(root.id)).await.unwrap();
    assert_eq!(change(2, false).await, Ok(Status::Pending));
    canic_qualification::upgrade_stopped(
        pic,
        root.id,
        std::fs::read(artifacts().join("peer-root.wasm")).unwrap(),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
    let name = "observe_peer";
    let frame = |direction| ic_wire::Encoding::Frame {
        method: ic_wire::method_id(name),
        direction,
    };
    let bytes = pic
        .query_call(
            root.id,
            root.operator,
            name,
            ic_wire::encode(
                &("project_hub", "accounts_hub", "project_sync"),
                frame(ic_wire::Direction::Request),
                canic::method::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let (pending, status) = ic_wire::decode::<Result<Option<(Decision, Status)>, String>>(
        &bytes,
        frame(ic_wire::Direction::Response),
        canic::method::LIMITS,
    )
    .unwrap()
    .unwrap()
    .unwrap();
    assert_eq!(
        (pending.generation, pending.allow, status),
        (2, false, Status::Pending)
    );
    assert!(change(3, true).await.is_err());
    pic.start_canister(accounts, Some(root.id)).await.unwrap();
    let refused = pic
        .update_call(
            accounts,
            project,
            ApplyPeer::NAME,
            ApplyPeer::encode_request(&pending).unwrap(),
        )
        .await
        .unwrap();
    assert!(ApplyPeer::decode_reply(&refused).unwrap().is_err());
    assert_eq!(change(2, false).await, Ok(Status::Applied));
    canic_qualification::upgrade_stopped(
        pic,
        accounts,
        std::fs::read(artifacts().join("directory.wasm")).unwrap(),
        vec![],
        Some(root.id),
    )
    .await
    .unwrap();
    assert_eq!(sync().await, Err("peer denied".into()));
    assert_eq!(change(3, true).await, Ok(Status::Applied));

    let gate = pic.create_canister().await;
    let catalog = Catalog::new(
        &[Reply {
            method: "hold",
            bytes: &[],
        }],
        Limits {
            slots: 1,
            name_bytes: 8,
            reply_bytes: 0,
        },
    )
    .unwrap();
    let mut held = Fixture::new(
        pic,
        gate.to_text().parse().unwrap(),
        &catalog,
        100_000_000_000,
        8192,
    )
    .await
    .unwrap();
    pic.add_cycles(gate, 1_000_000_000_000).await;
    pic.install_canister(gate, held.wasm().to_vec(), vec![], None)
        .await;
    let frame = |direction| ic_wire::Encoding::Frame {
        method: ic_wire::method_id("send_sync"),
        direction,
    };
    let msg = pic
        .submit_call(
            project,
            root.id,
            "send_sync",
            ic_wire::encode(
                &Some(gate),
                frame(ic_wire::Direction::Request),
                canic::method::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let mut token = None;
    for _ in 0..40 {
        pic.tick().await;
        token = held.pending(pic).await.unwrap().pop();
        if token.is_some() {
            break;
        }
    }
    let token = token.expect("admitted call reached barrier");
    assert_eq!(change(4, false).await, Ok(Status::Applied));
    assert_eq!(change(5, true).await, Ok(Status::Applied));
    held.release(pic, &token).await.unwrap();
    let reply = pic.await_call(msg).await.unwrap();
    assert_eq!(
        ic_wire::decode::<Result<u64, String>>(
            &reply,
            frame(ic_wire::Direction::Response),
            canic::method::LIMITS
        )
        .unwrap(),
        Err("peer permission changed".into())
    );
    assert_eq!(sync().await, Ok(1));
}
