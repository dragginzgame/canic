//! Real counter persistence and admission; no Root or Backup recovery claim.
use canic::readiness;
use canic_demo::{Error, LIMITS, METHOD, Request, Snapshot};
use canic_qualification::counter;
use ic_testkit::{
    pocket_ic::{
        CanisterId, CanisterSettings, PocketIcBuilder, common::rest::BlobCompression,
        nonblocking::PocketIc,
    },
    server::{Limits, PreparedServer, Server},
};
use ic_wire::{Direction, Encoding};
use std::{path::PathBuf, time::Duration};

fn encode(request: &impl serde::Serialize) -> Vec<u8> {
    ic_wire::encode(
        request,
        Encoding::Frame {
            method: METHOD,
            direction: Direction::Request,
        },
        LIMITS,
    )
    .unwrap()
}

async fn readiness(pic: &PocketIc, id: CanisterId) -> readiness::State {
    let request = ic_wire::encode(
        &(),
        Encoding::Frame {
            method: readiness::METHOD,
            direction: Direction::Request,
        },
        readiness::REQUEST_LIMITS,
    )
    .unwrap();
    let bytes = pic
        .update_call(id, CanisterId::anonymous(), "readiness", request)
        .await
        .unwrap();
    ic_wire::decode(
        &bytes,
        Encoding::Frame {
            method: readiness::METHOD,
            direction: Direction::Response,
        },
        readiness::RESPONSE_LIMITS,
    )
    .unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn counter_preserves_value_authority_and_fence_across_upgrade() {
    let directory =
        PathBuf::from(std::env::var_os("CANIC_COUNTER_ARTIFACTS").expect("run make counter-test"));
    let a = std::fs::read(directory.join("v1.wasm")).unwrap();
    let b = std::fs::read(directory.join("v2.wasm")).unwrap();
    let rejected = std::fs::read(directory.join("reject.wasm")).unwrap();
    let incompatible = std::fs::read(directory.join("incompatible.wasm")).unwrap();
    assert_ne!(a, b);
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
    let operator = CanisterId::self_authenticating(b"counter installer");
    let users = [
        CanisterId::self_authenticating(b"counter visitor"),
        CanisterId::anonymous(),
    ];
    let id = pic
        .create_canister_with_settings(Some(operator), None)
        .await;
    pic.add_cycles(id, 2_000_000_000_000).await;
    pic.install_canister(id, a.clone(), vec![], Some(operator))
        .await;
    // IC controller membership does not replace the stored application authority.
    pic.update_canister_settings(
        id,
        Some(operator),
        CanisterSettings {
            controllers: Some(vec![operator, users[0]]),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(readiness(pic, id).await, readiness::State::Ready);
    for value in 1..=8 {
        assert_eq!(
            counter(pic, id, users[value as usize % 2], Request::Increment)
                .await
                .unwrap()
                .value,
            value
        );
    }
    let initial = counter(pic, id, users[0], Request::Read).await.unwrap();
    assert_eq!(
        initial,
        Snapshot {
            value: 8,
            fence: None,
            executable: 1
        }
    );
    let stable = pic.get_stable_memory(id).await;
    assert!(stable.len() <= 8 * 65_536, "counter physical page budget");
    let valid = encode(&Request::Increment);
    for invalid in [
        vec![],
        vec![0; LIMITS.bytes + 1],
        valid[..valid.len() - 1].to_vec(),
        encode(&(Request::Fence { attempt: 1 }, operator)),
        ic_wire::encode(
            &Request::Increment,
            Encoding::Frame {
                method: METHOD + 1,
                direction: Direction::Request,
            },
            LIMITS,
        )
        .unwrap(),
        ic_wire::encode(
            &Request::Increment,
            Encoding::Frame {
                method: METHOD,
                direction: Direction::Response,
            },
            LIMITS,
        )
        .unwrap(),
    ] {
        assert!(
            pic.update_call(id, users[0], "counter", invalid)
                .await
                .is_err()
        );
        assert_eq!(
            counter(pic, id, users[0], Request::Read).await.unwrap(),
            initial
        );
        assert_eq!(pic.get_stable_memory(id).await, stable);
    }
    assert_eq!(
        counter(pic, id, users[0], Request::Fence { attempt: 1 }).await,
        Err(Error::NotOwner)
    );
    assert_eq!(
        counter(pic, id, operator, Request::Fence { attempt: 0 }).await,
        Err(Error::Attempt)
    );
    assert_eq!(pic.get_stable_memory(id).await, stable);
    let fenced = counter(pic, id, operator, Request::Fence { attempt: 1 })
        .await
        .unwrap();
    let retained = pic.get_stable_memory(id).await;
    for _ in 0..3 {
        assert_eq!(readiness(pic, id).await, readiness::State::Fenced);
        assert_eq!(
            counter(pic, id, users[1], Request::Increment).await,
            Err(Error::Fenced)
        );
        assert_eq!(
            counter(pic, id, operator, Request::Fence { attempt: 1 })
                .await
                .unwrap(),
            fenced
        );
    }
    assert_eq!(
        pic.get_stable_memory(id).await,
        retained,
        "observing cannot activate"
    );
    let before = pic.canister_status(id, Some(operator)).await.unwrap();
    pic.stop_canister(id, Some(operator)).await.unwrap();
    for candidate in [rejected, incompatible] {
        assert!(
            pic.upgrade_canister(id, candidate, vec![], Some(operator))
                .await
                .is_err()
        );
        assert_eq!(
            pic.get_stable_memory(id).await,
            retained,
            "failed upgrade changed durable state"
        );
        assert_eq!(
            pic.canister_status(id, Some(operator))
                .await
                .unwrap()
                .module_hash,
            before.module_hash
        );
    }
    // A different IC controller can upgrade code without becoming the app's owner.
    pic.upgrade_canister(id, b.clone(), vec![], Some(users[0]))
        .await
        .unwrap();
    pic.start_canister(id, Some(operator)).await.unwrap();
    assert_eq!(
        counter(pic, id, users[0], Request::Read).await.unwrap(),
        Snapshot {
            executable: 2,
            ..fenced
        }
    );
    assert_eq!(readiness(pic, id).await, readiness::State::Fenced);
    assert_eq!(pic.get_stable_memory(id).await, retained);
    assert_eq!(
        counter(pic, id, users[0], Request::Activate { attempt: 1 }).await,
        Err(Error::NotOwner)
    );
    assert_eq!(
        counter(pic, id, operator, Request::Activate { attempt: 0 }).await,
        Err(Error::Attempt)
    );
    counter(pic, id, operator, Request::Activate { attempt: 1 })
        .await
        .unwrap();
    counter(pic, id, operator, Request::Activate { attempt: 1 })
        .await
        .unwrap();
    assert_eq!(
        counter(pic, id, operator, Request::Fence { attempt: 1 }).await,
        Err(Error::Attempt)
    );
    counter(pic, id, operator, Request::Fence { attempt: 2 })
        .await
        .unwrap();
    assert_eq!(
        counter(pic, id, operator, Request::Activate { attempt: 1 }).await,
        Err(Error::Attempt)
    );
    assert_eq!(readiness(pic, id).await, readiness::State::Fenced);
    pic.stop_canister(id, Some(operator)).await.unwrap();
    pic.upgrade_canister(id, a, vec![], Some(operator))
        .await
        .unwrap();
    pic.start_canister(id, Some(operator)).await.unwrap();
    assert_eq!(
        counter(pic, id, users[0], Request::Read).await.unwrap(),
        Snapshot {
            value: 8,
            fence: Some(2),
            executable: 1
        }
    );
    counter(pic, id, operator, Request::Activate { attempt: 2 })
        .await
        .unwrap();
    assert_eq!(readiness(pic, id).await, readiness::State::Ready);
    assert_eq!(
        counter(pic, id, users[1], Request::Increment)
            .await
            .unwrap()
            .value,
        9
    );

    // Administrative fixture corruption tests reopen refusal, never a repair path.
    pic.stop_canister(id, Some(operator)).await.unwrap();
    let mut corrupt = retained;
    corrupt[0] ^= 0xff;
    for damaged in [corrupt, vec![]] {
        pic.set_stable_memory(id, damaged.clone(), BlobCompression::NoCompression)
            .await;
        assert!(
            pic.upgrade_canister(id, b.clone(), vec![], Some(operator))
                .await
                .is_err()
        );
        assert_eq!(
            pic.get_stable_memory(id).await,
            damaged,
            "reopen initialized or repaired damaged state"
        );
    }
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert_eq!(server.stop().unwrap().abandoned, 0);
}
