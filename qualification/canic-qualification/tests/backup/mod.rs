//! Direct, stopped fixture with disposable external secp256k1 custody.
mod gateway;
mod managed;
mod signed_bindings;
mod signer_failures;
use canic_host::backup::{DirectSigner, SnapshotCodec};
use ic_backup::store::OperationStep;
use ic_backup::store::{
    BackupApplicationGuard, BackupConnection, BackupDigest, BackupHttpTransport, BackupStore,
    BackupTarget, CaptureRequest, CaptureTiming, OperationAction, OperationLimits,
    OperationOptions, OperationPlan, OperationProgress, RestoreRequest,
};
use ic_backup::store::{RootCodec, RootManager};
use ic_host_sign::{
    Signer,
    protocol::{Configuration, tool_hash},
};
use ic_testkit::{
    pocket_ic::{CanisterId, CanisterSettings, PocketIcBuilder, common::rest::BlobCompression},
    server::{Limits, PreparedServer, Server},
};
use std::sync::atomic::Ordering;
use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime},
};

struct StoppedFixture(BackupTarget);
impl BackupApplicationGuard for StoppedFixture {
    type Error = ();
    fn protocol(&self) -> [u8; 32] {
        [9; 32]
    }
    fn admit(
        &self,
        plan: OperationPlan<'_>,
        _: OperationAction,
        application: &[u8; 32],
    ) -> Result<(), ()> {
        (plan.target() == self.0 && *application == [3; 32])
            .then_some(())
            .ok_or(())
    }
}

#[tokio::test]
async fn external_signer_captures_and_restores_direct_child() {
    exercise(false).await;
}

#[tokio::test]
async fn root_managed_backup_recovers_original_take_and_load() {
    exercise(true).await;
}
async fn exercise(managed: bool) {
    let dir = PathBuf::from(
        std::env::var_os("CANIC_BACKUP_FIXTURE").expect("use make backup-signer-test"),
    );
    let tool = PathBuf::from(std::env::var_os("CANIC_BACKUP_SIGNER").unwrap());
    let manifest = dir.join("identity/identity_list.json");
    let identities: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    let identity = &identities["identities"]["canic-backup-test"];
    assert_eq!(identity["algorithm"], "secp256k1");
    let sender = identity["principal"].as_str().unwrap();
    let caller = CanisterId::from_text(sender).unwrap();
    let config = Configuration {
        sender: sender.to_owned(),
        identity: "canic-backup-test".into(),
        manifest: manifest.to_str().unwrap().into(),
        local_bus: Some(std::env::var("DBUS_SESSION_BUS_ADDRESS").unwrap()),
    };
    let expected_tool = tool_hash(&tool).unwrap();
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(300),
            stdout_bytes: 1 << 20,
            stderr_bytes: 1 << 20,
        },
    )
    .unwrap();
    let mut instance = server
        .instance(
            PocketIcBuilder::new()
                .with_nns_subnet()
                .with_application_subnet()
                .with_initial_time(SystemTime::now().into()),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    let pic = instance.client_mut();
    let (manager, controller, child, wasm, stable) = if managed {
        let (root, child, root_wasm, wasm) = managed::install(pic, caller).await;
        let manager = RootManager {
            root: root.as_slice().try_into().unwrap(),
            protocol: SnapshotCodec.protocol(),
            authority_policy: canic_management_protocol::digest(&root_wasm),
            generation: 1,
            cycles: 0,
            cycle_limit: canic_management_protocol::backup::MAX_CYCLES,
            reply_bytes: 264192,
        };
        let stable = pic.get_stable_memory(child).await;
        assert!(!stable.is_empty());
        (Some(manager), root, child, wasm, stable)
    } else {
        let child = pic
            .create_canister_with_settings(
                Some(caller),
                Some(CanisterSettings {
                    controllers: Some(vec![caller]),
                    ..Default::default()
                }),
            )
            .await;
        let wasm = b"\0asm\x01\0\0\0\x05\x03\x01\x00\x01\x07\x0a\x01\x06memory\x02\x00";
        pic.install_canister(child, wasm.to_vec(), vec![], Some(caller))
            .await;
        let stable = vec![17; 65_536];
        pic.set_stable_memory(child, stable.clone(), BlobCompression::NoCompression)
            .await;
        (None, caller, child, wasm.to_vec(), stable)
    };
    pic.stop_canister(child, Some(controller)).await.unwrap();
    let trust = pic.root_key().await.unwrap();
    let trust_root = BackupDigest::compute(&trust);
    let target = BackupTarget {
        network: trust_root,
        canister: child.as_slice().try_into().unwrap(),
    };
    let endpoint = pic
        .make_live_with_params(
            Some("127.0.0.1".parse().unwrap()),
            None,
            Some(vec!["127.0.0.1".into()]),
            None,
        )
        .await;
    let gateway = gateway::Gateway::start(endpoint.port().unwrap()).await;
    let connect_with = |signer| {
        let connection = BackupConnection::new(
            BackupHttpTransport::new(
                &gateway.endpoint,
                target.network,
                trust.clone(),
                Duration::from_secs(30),
            )
            .unwrap(),
            signer,
            StoppedFixture(target),
        );
        if let Some(manager) = &manager {
            connection.with_root_manager(manager.clone(), &SnapshotCodec)
        } else {
            connection
        }
    };
    let connect = |config| {
        connect_with(DirectSigner(
            Signer::new(&tool, expected_tool, config).unwrap(),
        ))
    };
    let mut connection = connect(config.clone());
    let module = BackupDigest::compute(&wasm);
    let limits = OperationLimits {
        payload_bytes: 4 << 20,
        extent_bytes: if managed { 256 << 10 } else { 16 << 10 },
        network_requests: 128,
        observations: 1,
        signing_requests: 128,
        evidence_bytes: if managed { 1 << 30 } else { 256 << 20 },
    };
    let sender = caller.as_slice().try_into().unwrap();
    let options = OperationOptions {
        application: [3; 32],
        snapshot_capacity: 4,
        prepare_duration_ns: 30_000_000_000,
        submit_duration_ns: 120_000_000_000,
        timing: CaptureTiming {
            certificate_max_age_ns: 60_000_000_000,
            delegation_max_age_ns: 86_400_000_000_000,
            max_clock_skew_ns: 1_000_000_000,
        },
        signer_network: 0,
        request_bytes: if managed { 2 << 20 } else { 64 << 10 },
        result_bytes: if managed { 4 << 20 } else { 2 << 20 },
    };
    let plan = connection
        .plan(
            CaptureRequest {
                operation_id: "01".repeat(16),
                target,
                trust_root,
                initiator: sender,
                release: module,
                module_sha256: module,
                sender,
                limits,
            },
            options,
        )
        .unwrap();
    if !managed {
        let mut changed_sender = plan.request().clone();
        changed_sender.sender = CanisterId::self_authenticating(b"another fixture sender")
            .as_slice()
            .try_into()
            .unwrap();
        let other = connection.plan(changed_sender, options).unwrap();
        signed_bindings::exercise(
            &dir,
            config.clone(),
            DirectSigner(Signer::new(&tool, expected_tool, config.clone()).unwrap()),
            &plan,
            &other,
        )
        .await;
        signer_failures::exercise(&dir, &config, &connect_with, &plan, options).await;
        assert_eq!(gateway.requests.load(Ordering::SeqCst), 0);
    }
    let store_path = dir.join(if managed { "managed-store" } else { "store" });
    if managed {
        fs::create_dir(&store_path).unwrap();
    }
    let select = |store: BackupStore| {
        if managed {
            store.with_root_codec(&SnapshotCodec)
        } else {
            store
        }
    };
    let store = select(BackupStore::initialize(&store_path).unwrap());
    let mut capture = store.create_capture(plan.clone(), plan.digest()).unwrap();
    while capture.inspect().unwrap().next_call != Some(OperationStep::TakeSnapshot) {
        assert!(matches!(
            capture.advance(&mut connection).await.unwrap(),
            OperationProgress::Advanced
        ));
    }
    gateway.lose_next_reply.store(true, Ordering::SeqCst);
    assert!(capture.advance(&mut connection).await.is_err());
    assert_eq!(
        pic.list_canister_snapshots(child, Some(controller))
            .await
            .unwrap()
            .len(),
        1
    );
    let pending = capture.inspect().unwrap();
    assert!(pending.request_id.is_some());
    let submitted = gateway.requests.load(Ordering::SeqCst);
    drop(capture);
    drop(connection);
    let journal = store_path
        .join("operations")
        .join(&plan.request().operation_id)
        .join("journal.bin");
    let retained = fs::read(&journal).unwrap();
    drop(store);
    let store = select(BackupStore::open(&store_path).unwrap());
    let mut capture = store
        .open_capture(&plan.request().operation_id, plan.digest())
        .unwrap();
    // The same public identity through a different manifest is a new signer configuration.
    let alternate = store_path.with_extension("alternate.json");
    fs::copy(&manifest, &alternate).unwrap();
    let mut changed = config.clone();
    changed.manifest = alternate.to_str().unwrap().into();
    assert!(matches!(
        capture.reconcile_observation(&mut connect(changed)).await,
        Err(ic_backup::store::OperationAdvanceError::Binding)
    ));
    let mut connection = connect(config.clone());
    assert!(matches!(
        capture.advance(&mut connection).await.unwrap(),
        OperationProgress::NeedsAttention(OperationStep::TakeSnapshot)
    ));
    let reopened = capture.inspect().unwrap();
    assert_eq!(pending.request_id, reopened.request_id);
    assert_eq!(
        pending.signing_requests_reserved,
        reopened.signing_requests_reserved
    );
    assert_eq!(fs::read(&journal).unwrap(), retained);
    assert_eq!(gateway.requests.load(Ordering::SeqCst), submitted);
    assert!(matches!(
        capture
            .reconcile_observation(&mut connection)
            .await
            .unwrap(),
        OperationProgress::Advanced
    ));
    assert_eq!(gateway.requests.load(Ordering::SeqCst), submitted + 1);
    assert_eq!(capture.inspect().unwrap().observations_reserved, 1);
    assert_eq!(
        pic.list_canister_snapshots(child, Some(controller))
            .await
            .unwrap()
            .len(),
        1
    );
    let captured = loop {
        match capture.advance(&mut connection).await.unwrap() {
            OperationProgress::Complete(receipt) => break *receipt,
            OperationProgress::Advanced => (),
            OperationProgress::NeedsAttention(step) => panic!("unresolved capture {step:?}"),
        }
    };
    drop(capture);
    let mut capture = store
        .open_capture(&captured.bundle_id, plan.digest())
        .unwrap();
    assert_eq!(
        capture.verify_completion(captured.manifest_sha256).unwrap(),
        captured
    );
    let source = store
        .open_capture_source(
            &mut capture,
            &captured,
            captured.payload_bytes.try_into().unwrap(),
        )
        .unwrap();
    let restore = connection
        .plan_restore(
            &source,
            RestoreRequest {
                operation_id: "02".repeat(16),
                target,
                trust_root,
                initiator: sender,
                release: module,
                sender,
                limits,
            },
            options,
        )
        .unwrap();
    drop(source);
    drop(capture);
    pic.set_stable_memory(
        child,
        vec![42; stable.len()],
        BlobCompression::NoCompression,
    )
    .await;
    let mut operation = store
        .create_restore(&restore, restore.digest(), &plan)
        .unwrap();
    if managed {
        while operation.inspect().unwrap().next_call != Some(OperationStep::LoadSnapshot) {
            assert!(matches!(
                operation.advance(&mut connection).await.unwrap(),
                OperationProgress::Advanced
            ));
        }
        gateway.lose_next_reply.store(true, Ordering::SeqCst);
        assert!(operation.advance(&mut connection).await.is_err());
        let retained = operation.inspect().unwrap();
        let sent = gateway.requests.load(Ordering::SeqCst);
        drop(operation);
        drop(connection);
        connection = connect(config);
        operation = store
            .open_restore(&restore.request().operation_id, restore.digest())
            .unwrap();
        assert!(matches!(
            operation.advance(&mut connection).await.unwrap(),
            OperationProgress::NeedsAttention(OperationStep::LoadSnapshot)
        ));
        assert_eq!(operation.inspect().unwrap().request_id, retained.request_id);
        assert_eq!(gateway.requests.load(Ordering::SeqCst), sent);
        assert!(matches!(
            operation
                .reconcile_observation(&mut connection)
                .await
                .unwrap(),
            OperationProgress::Advanced
        ));
        assert_eq!(gateway.requests.load(Ordering::SeqCst), sent + 1);
    }
    loop {
        match operation.advance(&mut connection).await.unwrap() {
            OperationProgress::Complete(receipt) => {
                assert_eq!(receipt.source_operation_id, captured.bundle_id);
                break;
            }
            OperationProgress::Advanced => (),
            OperationProgress::NeedsAttention(step) => panic!("unresolved restore {step:?}"),
        }
    }
    assert_eq!(pic.get_stable_memory(child).await, stable);
    assert_eq!(
        format!(
            "{:?}",
            pic.canister_status(child, Some(controller))
                .await
                .unwrap()
                .status
        ),
        "Stopped"
    );
    drop(operation);
    pic.stop_live().await;
    let mut replay = store
        .open_restore(&restore.request().operation_id, restore.digest())
        .unwrap();
    assert!(replay.inspect().unwrap().completed.is_some());
    assert_eq!(
        instance.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}
