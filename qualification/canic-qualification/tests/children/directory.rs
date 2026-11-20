//! Direct recipients bind to the real installing Root, including across a managed upgrade.
use super::*;
use canic::method::Contract;
use canic::topology::directory::{Directory, ReceiveDirectory};

fn frame(method: &str, direction: ic_wire::Direction) -> ic_wire::Encoding {
    ic_wire::Encoding::Frame {
        method: ic_wire::method_id(method),
        direction,
    }
}

async fn view(pic: &PocketIc, target: CanisterId) -> (CanisterId, Option<u64>) {
    let reply = pic
        .query_call(
            target,
            CanisterId::anonymous(),
            "directory_status",
            ic_wire::encode(
                &(),
                frame("directory_status", ic_wire::Direction::Request),
                canic::method::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode(
        &reply,
        frame("directory_status", ic_wire::Direction::Response),
        canic::method::LIMITS,
    )
    .unwrap()
}

#[tokio::test]
async fn directory_recipients_retain_the_installing_root() {
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
    let instance = server
        .instance(
            PocketIcBuilder::new().with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    let pic = instance.client();
    let root = pic.create_canister().await;
    pic.install_canister(root, artifact("root.wasm"), vec![], None)
        .await;
    pic.add_cycles(root, 10_000_000_000_000).await;
    let wasm = artifact("directory.wasm");
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    for slot in ChildId::ALL {
        admit(pic, root, slot, input(root, wasm.clone(), now))
            .await
            .unwrap();
    }
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    let [hub, sibling] = read(pic, root).await.map(|c| {
        assert_eq!(c.creation.phase, Phase::Ready);
        CanisterId::from_text(c.creation.worker.unwrap()).unwrap()
    });
    assert_ne!(hub, sibling);
    for recipient in [hub, sibling] {
        assert_eq!(view(pic, recipient).await, (root, None));
    }
    let original = pic.get_stable_memory(hub).await;
    let refusal = pic
        .update_call(
            hub,
            CanisterId::anonymous(),
            ReceiveDirectory::NAME,
            ReceiveDirectory::encode_request(&Directory {
                revision: 1,
                services: vec![],
            })
            .unwrap(),
        )
        .await
        .unwrap_err();
    assert!(
        refusal
            .reject_message
            .contains("unexpected topology sender"),
        "{refusal:?}"
    );
    let reply = pic
        .update_call(
            sibling,
            CanisterId::anonymous(),
            "attempt_delivery",
            ic_wire::encode(
                &hub,
                frame("attempt_delivery", ic_wire::Direction::Request),
                canic::method::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        !ic_wire::decode::<bool>(
            &reply,
            frame("attempt_delivery", ic_wire::Direction::Response),
            canic::method::LIMITS
        )
        .unwrap()
    );
    assert_eq!(pic.get_stable_memory(hub).await, original);
    let mut successor = wasm.clone();
    successor.extend_from_slice(&[0, 2, 1, b'b']); // Valid custom section, distinct module hash.
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let change = upgrade::Stage {
        approval: upgrade::Approval {
            manager: root.to_text(),
            child: hub.to_text(),
            previous_module: digest(&wasm),
            module_hash: digest(&successor),
            execute_at_ns: now + 10_000_000_000,
            expires_at_ns: now + 120_000_000_000,
        },
        wasm: successor,
    };
    let reply = pic
        .update_call(
            root,
            CanisterId::anonymous(),
            "stage_upgrade",
            ic_wire::encode(
                &change,
                ic_wire::Encoding::Frame {
                    method: upgrade::STAGE,
                    direction: ic_wire::Direction::Request,
                },
                STAGE_LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode::<Result<upgrade::Status, Refusal>>(
        &reply,
        ic_wire::Encoding::Frame {
            method: upgrade::STAGE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
    .unwrap();
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    assert_eq!(
        read(pic, root).await[0].upgrades[0].phase,
        upgrade::Phase::Ready
    );
    assert_eq!(view(pic, hub).await, (root, None));
    assert_eq!(pic.get_stable_memory(hub).await, original);
    assert_eq!(
        instance.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}
