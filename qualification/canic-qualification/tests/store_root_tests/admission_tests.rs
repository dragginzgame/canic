//! Root is the real admission caller; the fixture retains direct upgrade control explicitly.
use super::{Client, artifacts, inject, maintain, readiness_tests, store, support};
use canic_management_protocol::{
    publication::{self, AdmissionChange, AdmissionRequest, Error},
    stored::STORE_LIMITS,
};
use ic_testkit::pocket_ic::{CanisterId, CanisterSettings};
use store::{AdmissionCommand as Command, MaintenanceAttempt};

#[tokio::test(flavor = "current_thread")]
async fn admission_fences_an_in_flight_cache_repair_without_losing_pins() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
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
            ic_wire::encode(
                &input,
                ic_wire::Encoding::Frame {
                    method: store::PREPARE_CACHE,
                    direction: ic_wire::Direction::Request,
                },
                STORE_LIMITS,
            )
            .unwrap(),
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
                STORE_LIMITS,
            )
            .await
            .unwrap();
    }
    let bytes = pic.await_call(message).await.unwrap();
    let reply: Result<store::CacheReady, store::CacheError> = ic_wire::decode(
        &bytes,
        ic_wire::Encoding::Frame {
            method: store::PREPARE_CACHE,
            direction: ic_wire::Direction::Response,
        },
        STORE_LIMITS,
    )
    .unwrap();
    assert!(matches!(
        reply,
        Err(store::CacheError::Pin(
            store::Error::Closed | store::Error::Conflict
        ))
    ));
    let ready: Result<store::CacheReady, store::CacheError> = supplier
        .update("prepare_cache", store::PREPARE_CACHE, &input, STORE_LIMITS)
        .await;
    assert_eq!(ready.unwrap().pin, pin);
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

async fn request(
    root: &Client<'_>,
    supplier: &Client<'_>,
    command: AdmissionChange,
) -> AdmissionRequest {
    AdmissionRequest {
        store: supplier.id.to_text(),
        command,
        expires_at_ns: root.pic.get_time().await.as_nanos_since_unix_epoch() + 240_000_000_000,
    }
}

async fn send(root: &Client<'_>, input: &AdmissionRequest) -> publication::AdmissionReply {
    root.update(
        "change_store_admission",
        publication::ADMISSION,
        input,
        STORE_LIMITS,
    )
    .await
}

#[tokio::test(flavor = "current_thread")]
async fn closed_store_survives_upgrade_and_requires_its_exact_owner_attempt() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let (root, supplier) = readiness_tests::fixture(pic).await;
    let manifest = supplier.publish_bytes("kept", b"\0asm\x01\0\0\0").await;
    let pin = store::Pin {
        operation: [7; 32],
        manifest: manifest.clone(),
    };
    supplier.pin(&pin).await;
    let custody: Result<store::Custody, store::Error> =
        supplier.release_custody(&manifest.release).await;
    let mut pending = manifest.clone();
    pending.release.version = "pending".into();
    pending.hash = [8; 32];
    pending.chunks = vec![[9; 32]];
    supplier
        .update::<_, Result<store::Status, store::Error>>(
            "prepare",
            store::PREPARE,
            &pending,
            STORE_LIMITS,
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
    let read = publication::ObservationRequest::Admission(publication::InfoRequest {
        store: supplier.id.to_text(),
        expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 240_000_000_000,
    });
    let observe = async || {
        let reply = root
            .update::<_, publication::ObservationReply>(
                "observe_store",
                publication::OBSERVATION,
                &read,
                STORE_LIMITS,
            )
            .await
            .unwrap();
        let publication::Observation::Admission(status) = reply else {
            panic!("expected admission status")
        };
        status
    };
    assert_eq!(observe().await, None);
    let close = request(&root, &supplier, AdmissionChange::Close(first.clone())).await;
    let frame = ic_wire::encode(
        &close,
        ic_wire::Encoding::Frame {
            method: publication::ADMISSION,
            direction: ic_wire::Direction::Request,
        },
        STORE_LIMITS,
    )
    .unwrap();
    let old = ic_wire::encode(
        &(
            close.store.clone(),
            close.expires_at_ns,
            Command::Close(first.clone()),
        ),
        ic_wire::Encoding::Frame {
            method: 49,
            direction: ic_wire::Direction::Request,
        },
        STORE_LIMITS,
    )
    .unwrap();
    assert!(
        pic.update_call(root.id, root.operator, "change_store_admission", old)
            .await
            .is_err()
    );
    assert!(
        pic.update_call(
            root.id,
            CanisterId::anonymous(),
            "change_store_admission",
            frame.clone()
        )
        .await
        .is_err()
    );
    assert!(
        pic.update_call(root.id, root.operator, "change_store_admission", vec![1, 2])
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
    assert_eq!(observe().await, None);
    inject(&root, 12).await;
    assert!(
        pic.update_call(root.id, root.operator, "change_store_admission", frame)
            .await
            .is_err()
    );
    inject(&root, 0).await;
    // Store committed despite Root's reply trap; observe instead of inventing another attempt.
    let closed = observe().await.unwrap();
    assert!(closed.closed);
    assert_eq!(closed.owner, root.id.as_slice());
    assert_eq!(closed.attempt, first);
    assert_eq!(send(&root, &close).await.unwrap(), Some(closed.clone()));
    let store_closed = pic.get_stable_memory(supplier.id).await;
    let blocked: Result<store::Status, store::Error> = supplier
        .update("publish", store::PUBLISH, &manifest.release, STORE_LIMITS)
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
            STORE_LIMITS,
        )
        .await;
    assert_eq!(cache, Err(store::CacheError::Pin(store::Error::Closed)));
    let wrong_owner: Result<Option<store::Admission>, store::Error> = supplier
        .update(
            "admission",
            store::ADMISSION,
            &Command::Activate(first.clone()),
            STORE_LIMITS,
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
    assert_eq!(observe().await, Some(closed));
    assert_eq!(supplier.status(&pending.release).await, partial);
    let after: Result<store::Custody, store::Error> =
        supplier.release_custody(&manifest.release).await;
    assert_eq!(after, custody);
    let activate = request(&root, &supplier, AdmissionChange::Activate(first.clone())).await;
    let active = send(&root, &activate).await.unwrap().unwrap();
    assert!(!active.closed);
    assert_eq!(send(&root, &close).await.unwrap(), Some(active));
    send(
        &root,
        &request(&root, &supplier, AdmissionChange::Close(second.clone())).await,
    )
    .await
    .unwrap();
    assert_eq!(
        send(&root, &activate).await,
        Err(Error::Store(store::Error::Conflict))
    );
    maintain(&root).await;
    let latest = observe().await.unwrap();
    assert!(latest.closed);
    assert_eq!(latest.attempt, second);
    send(
        &root,
        &request(&root, &supplier, AdmissionChange::Activate(second)).await,
    )
    .await
    .unwrap();
    supplier.pin(&pin).await; // Mutation becomes available only after exact explicit activation.
    assert_eq!(pic.get_stable_memory(root.id).await, root_before);
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
