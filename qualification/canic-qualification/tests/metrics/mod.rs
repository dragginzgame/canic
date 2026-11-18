//! Installed public metrics do not grant authority, mutate custody or survive owner upgrades.
use super::*;
use canic::metrics::{METHOD, REQUEST_LIMITS, RESPONSE_LIMITS, Snapshot};

fn request() -> Vec<u8> {
    ic_wire::encode(
        &(),
        ic_wire::Encoding::Frame {
            method: METHOD,
            direction: ic_wire::Direction::Request,
        },
        REQUEST_LIMITS,
    )
    .unwrap()
}

pub(super) async fn read(pic: &PocketIc, owner: CanisterId) -> Snapshot {
    let bytes = pic
        .query_call(
            owner,
            CanisterId::self_authenticating(b"public metrics reader"),
            "metrics",
            request(),
        )
        .await
        .unwrap();
    ic_wire::decode(
        &bytes,
        ic_wire::Encoding::Frame {
            method: METHOD,
            direction: ic_wire::Direction::Response,
        },
        RESPONSE_LIMITS,
    )
    .unwrap()
}

#[tokio::test]
async fn public_queries_preserve_state_and_upgrade_resets_only_diagnostics() {
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
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
    let root = pic.create_canister().await;
    pic.install_canister(root, children::artifact("root.wasm"), vec![], None)
        .await;
    let empty = read(pic, root).await.timers.unwrap();
    assert_eq!(empty.accepted.total, 0);
    let input = children::input(
        root,
        children::artifact("app.wasm"),
        pic.get_time().await.as_nanos_since_unix_epoch(),
    );
    stage(pic, root, &input).await.unwrap();
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    let complete = status(pic, root).await;
    assert_eq!(complete.phase, Phase::Ready);
    let before = read(pic, root).await;
    let timers = before.timers.unwrap();
    assert!(timers.accepted.total > 0);
    assert_eq!(timers.completed.total, timers.accepted.total);
    assert_eq!(timers.provider_faults.total, 0);
    assert!(!timers.accepted.overflowed);
    let stable = pic.get_stable_memory(root).await;
    let mut wrong_method = request();
    wrong_method[2] ^= 1;
    let mut wrong_direction = request();
    wrong_direction[10] = 1;
    let mut trailing = request();
    trailing.push(1);
    for bytes in [
        vec![],
        vec![0; REQUEST_LIMITS.bytes + 1],
        wrong_method,
        wrong_direction,
        trailing,
    ] {
        assert!(
            pic.query_call(root, CanisterId::anonymous(), "metrics", bytes)
                .await
                .is_err()
        );
    }
    for _ in 0..3 {
        assert_eq!(read(pic, root).await.timers, before.timers);
    }
    assert_eq!(pic.get_stable_memory(root).await, stable);
    assert_eq!(status(pic, root).await, complete);
    canic_qualification::upgrade_stopped(
        pic,
        root,
        children::artifact("maintenance.wasm"),
        vec![],
        None,
    )
    .await
    .unwrap();
    let reset = read(pic, root).await.timers.unwrap();
    assert!(reset.started_at_version > timers.started_at_version);
    assert!(reset.started_at_ns >= timers.started_at_ns);
    assert_eq!(reset.accepted.total, 0);
    assert_eq!(reset.completed.total, 0);
    let mut after = status(pic, root).await;
    assert!(after.maintenance_candidate);
    // The replacement intentionally changes only this executable marker.
    after.maintenance_candidate = complete.maintenance_candidate;
    assert_eq!(after, complete);
    assert_eq!(pic.get_stable_memory(root).await, stable);
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
