//! Installed Notebook writes, upgrades and application recovery after snapshot rollback.
use canic::{
    method::LIMITS,
    recovery::{self, Command},
};
use ic_testkit::{
    pocket_ic::{CanisterId, PocketIcBuilder},
    server::{Deletion, Limits, PreparedServer, Server},
};
use ic_wire::{Direction, Encoding};
use std::{path::Path, time::Duration};

fn frame(name: &str, direction: Direction) -> Encoding {
    Encoding::Frame {
        method: ic_wire::method_id(name),
        direction,
    }
}

#[tokio::test(flavor = "current_thread")]
async fn notebook_preserves_the_note_and_fences_recovery() {
    let wasm = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/examples/notebook/artifacts/notebook.wasm"),
    )
    .expect("run make notebook-test");
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory())
            .expect("run make qualification-setup"),
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(120),
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
    let installer = CanisterId::self_authenticating(b"notebook installer");
    let id = pic
        .create_canister_with_settings(Some(installer), None)
        .await;
    pic.add_cycles(id, 2_000_000_000_000).await;
    pic.install_canister(id, wasm.clone(), vec![], Some(installer))
        .await;
    let caller = CanisterId::self_authenticating(b"notebook visitor");
    let read = async || {
        let args = ic_wire::encode(&(), frame("read", Direction::Request), LIMITS).unwrap();
        let bytes = pic.query_call(id, caller, "read", args).await.unwrap();

        ic_wire::decode::<String>(&bytes, frame("read", Direction::Response), LIMITS).unwrap()
    };
    let edit = async |text: &str| {
        let args = ic_wire::encode(&text, frame("edit", Direction::Request), LIMITS).unwrap();

        pic.update_call(id, caller, "edit", args).await
    };
    let recover = async |sender, command| {
        let args = recovery::encode(command, Direction::Request).unwrap();
        pic.update_call(id, sender, "recovery", args)
            .await
            .map(|reply| {
                assert_eq!(
                    reply,
                    recovery::encode(command, Direction::Response).unwrap()
                );
            })
    };
    assert_eq!(read().await, "");
    let boundary = "é".repeat(128);
    edit(&boundary).await.unwrap();
    assert!(edit(&format!("{boundary}x")).await.is_err());
    assert_eq!(read().await, boundary);
    edit("draft").await.unwrap();
    assert!(recover(caller, Command::Fence(1)).await.is_err());
    recover(installer, Command::Fence(1)).await.unwrap();
    canic_qualification::upgrade_stopped(pic, id, wasm, vec![], Some(installer))
        .await
        .unwrap();
    assert_eq!(read().await, "draft");
    assert!(edit("fenced after upgrade").await.is_err());
    pic.stop_canister(id, Some(installer)).await.unwrap();
    let snapshot = pic
        .take_canister_snapshot(id, Some(installer), None)
        .await
        .unwrap();
    pic.start_canister(id, Some(installer)).await.unwrap();
    recover(installer, Command::Open(1)).await.unwrap();
    edit("published").await.unwrap();
    assert_eq!(read().await, "published");
    pic.stop_canister(id, Some(installer)).await.unwrap();
    pic.load_canister_snapshot(id, Some(installer), snapshot.id)
        .await
        .unwrap();
    pic.start_canister(id, Some(installer)).await.unwrap();
    assert_eq!(read().await, "draft");
    assert!(edit("too early").await.is_err());
    let accept = Command::Accept {
        checkpoint: 1,
        generation: 2,
    };
    assert!(recover(caller, accept).await.is_err());
    recover(installer, accept).await.unwrap();
    assert!(edit("still fenced").await.is_err());
    assert!(recover(installer, Command::Open(1)).await.is_err());
    assert!(recover(caller, Command::Open(2)).await.is_err());
    assert_eq!(read().await, "draft");
    recover(installer, Command::Open(2)).await.unwrap();
    edit("recovered").await.unwrap();
    assert_eq!(read().await, "recovered");
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        Deletion::Attempted
    );
    assert_eq!(server.stop().unwrap().abandoned, 0);
}
