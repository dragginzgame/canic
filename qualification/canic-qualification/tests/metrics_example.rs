//! Exercise the metrics example through its fleet-call codec before and after upgrade.

use canic::{
    method::Contract,
    metrics::{ChildMetrics, ReadMetrics, Report, Request},
};
use canic_host::application::codec;
use ic_testkit::{
    pocket_ic::{CanisterId, PocketIcBuilder},
    server::{Deletion, Limits, PreparedServer, Server},
};
use std::{path::Path, time::Duration};

#[tokio::test(flavor = "current_thread")]
async fn controller_observes_only_committed_updates_in_this_incarnation() {
    let artifacts =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/examples/metrics/artifacts");
    let wasm = std::fs::read(artifacts.join("app.wasm")).expect("run make metrics-test");
    let codec = artifacts.join("app.client");
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
    let installer = CanisterId::self_authenticating(b"metrics installer");
    let id = pic
        .create_canister_with_settings(Some(installer), None)
        .await;
    pic.add_cycles(id, 2_000_000_000_000).await;
    pic.install_canister(id, wasm.clone(), vec![], Some(installer))
        .await;
    let root = pic
        .create_canister_with_settings(Some(installer), None)
        .await;
    pic.add_cycles(root, 3_000_000_000_000).await;
    pic.install_canister(
        root,
        std::fs::read(artifacts.join("root.wasm")).unwrap(),
        vec![],
        Some(installer),
    )
    .await;
    let root_codec = artifacts.join("root.client");
    for source in ["timers", "endpoints", "blob", "db", "jobs", "entropy"] {
        let args = ["child-metrics", "0", source].map(str::to_owned);
        let frame = codec::encode(&root_codec, &args).unwrap();
        assert_eq!(frame.method, ChildMetrics::NAME);
        let (slot, selected) = ChildMetrics::decode_request(&frame.arguments).unwrap();
        assert_eq!(slot, 0);
        assert!(matches!(
            (source, selected),
            ("timers", Request::Timers)
                | ("endpoints", Request::Endpoints(None))
                | ("blob", Request::Blob)
                | ("db", Request::Db)
                | ("jobs", Request::Jobs)
                | ("entropy", Request::Entropy)
        ));
    }
    let frame = codec::encode(&root_codec, &["logs".to_owned()]).unwrap();
    let reply = pic
        .update_call(root, installer, &frame.method, frame.arguments)
        .await
        .unwrap();
    let logs = codec::decode(&root_codec, &reply).unwrap();
    assert_eq!(logs["entries"][0]["message"], "installed");
    for (caller, reason) in [
        (CanisterId::anonymous(), "controller required"),
        (installer, "child is not ready"),
    ] {
        let denied = pic
            .update_call(
                root,
                caller,
                ChildMetrics::NAME,
                ChildMetrics::encode_request(&(0, Request::Timers)).unwrap(),
            )
            .await
            .unwrap_err();
        assert!(denied.reject_message.contains(reason), "{denied:?}");
    }
    let call = async |args: &[&str]| {
        let frame = codec::encode(
            &codec,
            &args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>(),
        )
        .unwrap();
        let bytes = pic
            .update_call(id, CanisterId::anonymous(), &frame.method, frame.arguments)
            .await
            .unwrap();

        codec::decode(&codec, &bytes).unwrap()
    };
    let read = async |caller, after| {
        pic.query_call(
            id,
            caller,
            ReadMetrics::NAME,
            ReadMetrics::encode_request(&Request::Endpoints(after)).unwrap(),
        )
        .await
        .map(|bytes| match ReadMetrics::decode_reply(&bytes).unwrap() {
            Report::Endpoints(page) => page,
            other => panic!("expected endpoints, got {other:?}"),
        })
    };
    for source in [
        Request::Timers,
        Request::Blob,
        Request::Db,
        Request::Jobs,
        Request::Entropy,
    ] {
        let bytes = ReadMetrics::encode_request(&source).unwrap();
        assert!(
            pic.query_call(
                id,
                CanisterId::anonymous(),
                ReadMetrics::NAME,
                bytes.clone()
            )
            .await
            .is_err()
        );
        let reply = pic
            .query_call(id, installer, ReadMetrics::NAME, bytes)
            .await
            .unwrap();
        let report = ReadMetrics::decode_reply(&reply).unwrap();
        if matches!(source, Request::Timers) {
            assert!(matches!(report, Report::Timers(_)));
        } else {
            assert!(matches!(report, Report::Unavailable));
        }
    }
    let empty = read(installer, None).await.unwrap();
    assert!(empty.measurements.is_empty());
    assert_eq!(call(&["ping", "é"]).await, 2);
    assert_eq!(call(&["ping", &"x".repeat(256)]).await, 256);
    assert_eq!(call(&["checksum", "abc"]).await, 96_354);
    assert!(
        pic.update_call(id, CanisterId::anonymous(), "ping", vec![])
            .await
            .is_err()
    );
    assert!(read(CanisterId::anonymous(), None).await.is_err());
    let page = read(installer, None).await.unwrap();
    assert!(page.next.is_none());
    let [checksum, ping] = page.measurements.as_slice() else {
        panic!("expected two endpoints: {page:?}")
    };
    assert_eq!((&*checksum.endpoint, checksum.calls), ("checksum", 1));
    assert_eq!((&*ping.endpoint, ping.calls), ("ping", 2));
    for row in [checksum, ping] {
        assert!(row.latest > 0 && row.latest <= row.maximum);
        assert!(row.average.is_some_and(|avg| avg > 0 && avg <= row.maximum));
        assert!(!row.overflowed);
    }
    assert_eq!(read(installer, None).await.unwrap(), page);
    assert_eq!(
        read(installer, Some("checksum".into()))
            .await
            .unwrap()
            .measurements,
        page.measurements[1..]
    );
    canic_qualification::upgrade_stopped(pic, id, wasm, vec![], Some(installer))
        .await
        .unwrap();
    assert_eq!(read(installer, None).await.unwrap(), empty);
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        Deletion::Attempted
    );
    assert_eq!(server.stop().unwrap().abandoned, 0);
}
