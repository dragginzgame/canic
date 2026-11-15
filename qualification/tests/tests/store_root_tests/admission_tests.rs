//! Root is the real admission caller; the fixture retains direct upgrade control explicitly.
use super::{Client, artifacts, inject, maintain, readiness_tests, store, support};
use canic_management_protocol::publication::{self, AdmissionRequest, Error};
use ic_testkit::pocket_ic::{CanisterId, CanisterSettings};
use store::{AdmissionCommand as Command, MaintenanceAttempt};

#[tokio::test(flavor = "current_thread")]
async fn admission_fences_an_in_flight_cache_repair_without_losing_pins() {
    let (_, binary) = support::inputs();
    let owner = support::instance(&binary).await;
    let pic = owner.instance();
    let (_, supplier) = readiness_tests::fixture(pic).await;
    let mut bytes = vec![0; store::CHUNK_BYTES * 8];
    for (i, chunk) in bytes.chunks_mut(store::CHUNK_BYTES).enumerate() {
        chunk.fill(i as u8);
    }
    bytes[..8].copy_from_slice(b"\0asm\x01\0\0\0");
    let manifest = supplier.publish_bytes("cache-fence", &bytes).await;
    let pin = store::Pin {
        operation: [17; 32],
        manifest,
    };
    supplier.pin(&pin).await;
    let input = store::PrepareCache {
        pin: pin.clone(),
        expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 240_000_000_000,
    };
    let message = pic
        .submit_call(
            supplier.id,
            supplier.operator,
            "prepare_cache",
            ic_wire::native::encode_request(store::PREPARE_CACHE, &input, store::LIMITS).unwrap(),
        )
        .await
        .unwrap();
    pic.tick().await;
    assert!(pic.ingress_status(message.clone()).await.is_none());
    let attempt = MaintenanceAttempt {
        sequence: 1,
        operation: [18; 32],
    };
    for command in [Command::Close(attempt.clone()), Command::Activate(attempt)] {
        supplier
            .update::<_, Result<Option<store::Admission>, store::Error>>(
                "admission",
                store::ADMISSION,
                &command,
                store::LIMITS,
            )
            .await
            .unwrap();
    }
    let bytes = pic.await_call(message).await.unwrap();
    let reply: Result<store::CacheReady, store::CacheError> =
        ic_wire::native::decode_response(store::PREPARE_CACHE, &bytes, store::LIMITS).unwrap();
    assert!(matches!(
        reply,
        Err(store::CacheError::Pin(
            store::Error::Closed | store::Error::Conflict
        ))
    ));
    let ready: Result<store::CacheReady, store::CacheError> = supplier
        .update("prepare_cache", store::PREPARE_CACHE, &input, store::LIMITS)
        .await;
    assert_eq!(ready.unwrap().pin, pin);
    owner.close().await;
}

async fn request(root: &Client<'_>, supplier: &Client<'_>, command: Command) -> AdmissionRequest {
    AdmissionRequest {
        store: supplier.id.to_text(),
        command,
        expires_at_ns: root.pic.get_time().await.as_nanos_since_unix_epoch() + 240_000_000_000,
    }
}
async fn send(root: &Client<'_>, input: &AdmissionRequest) -> publication::AdmissionReply {
    root.update(
        "store_admission",
        publication::ADMISSION,
        input,
        store::LIMITS,
    )
    .await
}

#[tokio::test(flavor = "current_thread")]
async fn closed_store_survives_upgrade_and_requires_its_exact_owner_attempt() {
    let (_, binary) = support::inputs();
    let owner = support::instance(&binary).await;
    let pic = owner.instance();
    let (root, supplier) = readiness_tests::fixture(pic).await;
    let manifest = supplier.publish_bytes("kept", b"\0asm\x01\0\0\0").await;
    let pin = store::Pin {
        operation: [7; 32],
        manifest: manifest.clone(),
    };
    supplier.pin(&pin).await;
    let custody: Result<store::Custody, store::Error> = supplier
        .update(
            "observe_custody",
            store::CUSTODY,
            &manifest.release,
            store::LIMITS,
        )
        .await;
    let mut pending = manifest.clone();
    pending.release.version = "pending".into();
    pending.hash = [8; 32];
    pending.chunks = vec![[9; 32]];
    supplier
        .update::<_, Result<store::Status, store::Error>>(
            "prepare",
            store::PREPARE,
            &pending,
            store::LIMITS,
        )
        .await
        .unwrap();
    let partial = supplier.status(&pending.release).await;
    // No production controller handover is claimed: the fixture performs the upgrade directly.
    pic.update_canister_settings(
        supplier.id,
        Some(root.operator),
        CanisterSettings {
            controllers: Some(vec![root.id, root.operator]),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let root_before = pic.get_stable_memory(root.id).await;
    let first = MaintenanceAttempt {
        sequence: 1,
        operation: [1; 32],
    };
    let second = MaintenanceAttempt {
        sequence: 2,
        operation: [2; 32],
    };
    let observe = request(&root, &supplier, Command::Observe).await;
    assert_eq!(send(&root, &observe).await.unwrap(), None);
    let close = request(&root, &supplier, Command::Close(first.clone())).await;
    let frame =
        ic_wire::native::encode_request(publication::ADMISSION, &close, store::LIMITS).unwrap();
    assert!(
        pic.update_call(
            root.id,
            CanisterId::anonymous(),
            "store_admission",
            frame.clone()
        )
        .await
        .is_err()
    );
    assert!(
        pic.update_call(root.id, root.operator, "store_admission", vec![1, 2])
            .await
            .is_err()
    );
    assert_eq!(
        send(
            &root,
            &AdmissionRequest {
                expires_at_ns: 0,
                ..close.clone()
            }
        )
        .await,
        Err(Error::Expired)
    );
    assert_eq!(send(&root, &observe).await.unwrap(), None);
    inject(&root, 12).await;
    assert!(
        pic.update_call(root.id, root.operator, "store_admission", frame)
            .await
            .is_err()
    );
    inject(&root, 0).await;
    // Store committed despite Root's reply trap; observe instead of inventing another attempt.
    let closed = send(&root, &observe).await.unwrap().unwrap();
    assert!(closed.closed);
    assert_eq!(closed.owner, root.id.as_slice());
    assert_eq!(closed.attempt, first);
    assert_eq!(send(&root, &close).await.unwrap(), Some(closed.clone()));
    let store_closed = pic.get_stable_memory(supplier.id).await;
    let blocked: Result<store::Status, store::Error> = supplier
        .update("publish", store::PUBLISH, &manifest.release, store::LIMITS)
        .await;
    assert_eq!(blocked, Err(store::Error::Closed));
    let cache: Result<store::CacheReady, store::CacheError> = supplier
        .update(
            "prepare_cache",
            store::PREPARE_CACHE,
            &store::PrepareCache {
                pin: pin.clone(),
                expires_at_ns: close.expires_at_ns,
            },
            store::LIMITS,
        )
        .await;
    assert_eq!(cache, Err(store::CacheError::Pin(store::Error::Closed)));
    let wrong_owner: Result<Option<store::Admission>, store::Error> = supplier
        .update(
            "admission",
            store::ADMISSION,
            &Command::Activate(first.clone()),
            store::LIMITS,
        )
        .await;
    assert_eq!(wrong_owner, Err(store::Error::Conflict));
    assert_eq!(pic.get_stable_memory(supplier.id).await, store_closed);
    pic.stop_canister(supplier.id, Some(root.operator))
        .await
        .unwrap();
    pic.upgrade_canister(
        supplier.id,
        std::fs::read(artifacts().join("candidate.wasm")).unwrap(),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
    pic.start_canister(supplier.id, Some(root.operator))
        .await
        .unwrap();
    assert!(supplier.info().await.maintenance_candidate);
    assert_eq!(send(&root, &observe).await.unwrap(), Some(closed));
    assert_eq!(supplier.status(&pending.release).await, partial);
    let after: Result<store::Custody, store::Error> = supplier
        .update(
            "observe_custody",
            store::CUSTODY,
            &manifest.release,
            store::LIMITS,
        )
        .await;
    assert_eq!(after, custody);
    let activate = request(&root, &supplier, Command::Activate(first.clone())).await;
    let active = send(&root, &activate).await.unwrap().unwrap();
    assert!(!active.closed);
    assert_eq!(send(&root, &close).await.unwrap(), Some(active));
    send(
        &root,
        &request(&root, &supplier, Command::Close(second.clone())).await,
    )
    .await
    .unwrap();
    assert_eq!(
        send(&root, &activate).await,
        Err(Error::Store(store::Error::Conflict))
    );
    maintain(&root).await;
    let latest = send(&root, &observe).await.unwrap().unwrap();
    assert!(latest.closed);
    assert_eq!(latest.attempt, second);
    send(
        &root,
        &request(&root, &supplier, Command::Activate(second)).await,
    )
    .await
    .unwrap();
    supplier.pin(&pin).await; // Mutation becomes available only after exact explicit activation.
    assert_eq!(pic.get_stable_memory(root.id).await, root_before);
    owner.close().await;
}
