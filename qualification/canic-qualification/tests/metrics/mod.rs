//! Installed guarded metrics do not grant authority, mutate custody or survive owner upgrades.
use super::*;
use canic::metrics::{ReadMetrics, Report, Request, Snapshot};
use canic::{logging::Logs, method::Contract};

fn request() -> Vec<u8> {
    ReadMetrics::encode_request(&Request::Timers).unwrap()
}

pub(super) async fn read(pic: &PocketIc, owner: CanisterId, caller: CanisterId) -> Snapshot {
    let bytes = pic
        .query_call(owner, caller, ReadMetrics::NAME, request())
        .await
        .unwrap();
    let Report::Timers(snapshot) = ReadMetrics::decode_reply(&bytes).unwrap() else {
        panic!("timer report expected")
    };
    snapshot
}

#[tokio::test]
async fn controller_queries_preserve_state_and_upgrade_resets_only_diagnostics() {
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
    let store = ChildStore::install(pic, root, children::artifact("store.wasm")).await;
    pic.install_canister(root, children::artifact("root.wasm"), vec![], None)
        .await;
    let logs = async |before: Option<u64>| {
        let bytes = pic
            .update_call(
                root,
                CanisterId::anonymous(),
                Logs::NAME,
                Logs::encode_request(&before).unwrap(),
            )
            .await
            .unwrap();
        Logs::decode_reply(&bytes).unwrap()
    };
    let denied = pic
        .update_call(
            root,
            CanisterId::self_authenticating(b"other operator"),
            Logs::NAME,
            Logs::encode_request(&None).unwrap(),
        )
        .await
        .unwrap_err();
    assert!(denied.reject_message.contains("controller required"));
    let first = logs(None).await;
    assert_eq!(first.entries.len(), 1);
    assert_eq!(first.entries[0].source, "root");
    assert_eq!(first.entries[0].message, "installed");
    let empty = read(pic, root, CanisterId::anonymous())
        .await
        .timers
        .unwrap();
    assert_eq!(empty.accepted.total, 0);
    let input = children::input(
        root,
        children::artifact("app.wasm"),
        pic.get_time().await.as_nanos_since_unix_epoch(),
    );
    stage(pic, root, &store, &input).await.unwrap();
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    let complete = status(pic, root).await;
    assert_eq!(complete.phase, Phase::Ready);
    let ready = logs(None).await;
    assert_eq!(ready.entries[0].source, "management");
    assert_eq!(ready.entries[0].message, "child 0: Ready");
    let earlier = logs(Some(ready.entries[0].sequence)).await;
    assert_eq!(earlier.entries.last(), first.entries.last());
    assert!(
        earlier
            .entries
            .iter()
            .all(|entry| entry.sequence < ready.entries[0].sequence)
    );
    assert_eq!(logs(None).await, ready);
    let before = read(pic, root, CanisterId::anonymous()).await;
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
        vec![0; ReadMetrics::REQUEST.bytes + 1],
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
        assert_eq!(
            read(pic, root, CanisterId::anonymous()).await.timers,
            before.timers
        );
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
    let reset = read(pic, root, CanisterId::anonymous())
        .await
        .timers
        .unwrap();
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
    let reopened = logs(None).await;
    assert!(reopened.started_at_version > ready.started_at_version);
    assert_eq!(reopened.entries.len(), 1);
    assert_eq!(reopened.entries[0].message, "reopened after upgrade");
    let late = children::input(
        root,
        children::artifact("app.wasm"),
        pic.get_time().await.as_nanos_since_unix_epoch(),
    );
    children::admit(
        pic,
        root,
        &store,
        canic_management_protocol::children::ChildId::SECOND,
        late,
    )
    .await
    .unwrap();
    pic.advance_time(Duration::from_secs(121)).await;
    settle(pic).await;
    let expired = logs(None).await;
    assert_eq!(expired.entries[0].message, "child 1: Expired");
    assert_eq!(expired.entries[0].level, "Warn");
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
