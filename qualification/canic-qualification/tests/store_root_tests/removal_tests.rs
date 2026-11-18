//! Real Root forwarding. The publication harness separately qualifies CLI settlement/resumption.
use super::{Client, inject, maintain, readiness_tests, store, support};
use canic_management_protocol::publication::{
    self as wire, Command, Error, Removal, RemovalRequest,
};
use canic_management_protocol::stored::STORE_LIMITS;
use ic_testkit::pocket_ic::{CanisterId, CanisterSettings};
use std::time::Duration;

async fn request(
    root: &Client<'_>,
    supplier: &Client<'_>,
    action: Removal,
    manifest: store::Manifest,
) -> RemovalRequest {
    RemovalRequest {
        store: supplier.id.to_text(),
        expires_at_ns: root.pic.get_time().await.as_nanos_since_unix_epoch() + 30_000_000_000,
        action,
        manifest,
    }
}
async fn remove(root: &Client<'_>, input: &RemovalRequest) -> wire::RemovalReply {
    root.update("store_removal", wire::REMOVE, input, STORE_LIMITS)
        .await
}
async fn publish(root: &Client<'_>, supplier: &Client<'_>, command: Command) -> wire::Reply {
    root.update(
        "store_publication",
        wire::EXECUTE,
        &wire::Request {
            store: supplier.id.to_text(),
            expires_at_ns: root.pic.get_time().await.as_nanos_since_unix_epoch() + 240_000_000_000,
            command,
        },
        wire::REQUEST_LIMITS,
    )
    .await
}
async fn handover(root: &Client<'_>, supplier: &Client<'_>) {
    root.pic
        .update_canister_settings(
            supplier.id,
            Some(root.operator),
            CanisterSettings {
                controllers: Some(vec![root.id]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn root_removal_keeps_pins_labels_and_exact_identity_before_expired_authority_restoration() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let (root, supplier) = readiness_tests::fixture(pic).await;
    let bytes = b"\0asm\x01\0\0\0";
    let removed = supplier.publish_bytes("removed", bytes).await;
    let shared = supplier.publish_bytes("shared", bytes).await;
    let pinned = supplier
        .publish_bytes("pinned", b"\0asm\x01\0\0\0\0\x01\0")
        .await;
    supplier
        .pin(&store::Pin {
            operation: [56; 32],
            manifest: pinned.clone(),
        })
        .await;
    handover(&root, &supplier).await;
    let root_before = pic.get_stable_memory(root.id).await;
    let input = request(&root, &supplier, Removal::Delete, removed.clone()).await;
    let pinned_input = request(&root, &supplier, Removal::Delete, pinned).await;
    let before = pic.get_stable_memory(supplier.id).await;
    assert_eq!(
        remove(&root, &pinned_input).await,
        Err(Error::Store(store::Error::Conflict))
    );
    let mut wrong = input.clone();
    wrong.manifest.hash = [90; 32];
    assert_eq!(
        remove(&root, &wrong).await,
        Err(Error::Store(store::Error::Conflict))
    );
    wrong = input.clone();
    wrong.action = Removal::Cancel;
    assert_eq!(
        remove(&root, &wrong).await,
        Err(Error::Store(store::Error::Conflict))
    );
    let frame = ic_wire::encode(
        &input,
        ic_wire::Encoding::Frame {
            method: wire::REMOVE,
            direction: ic_wire::Direction::Request,
        },
        STORE_LIMITS,
    )
    .unwrap();
    for (caller, args) in [
        (CanisterId::anonymous(), frame.clone()),
        (root.operator, vec![0]),
    ] {
        assert!(
            pic.update_call(root.id, caller, "store_removal", args)
                .await
                .is_err()
        );
    }
    assert_eq!(pic.get_stable_memory(supplier.id).await, before);
    // Only this actual reply settles this invocation. It does not settle any other request.
    let settled = remove(&root, &input).await.unwrap();
    assert_eq!(settled, removed);
    assert_eq!(
        publish(&root, &supplier, Command::Observe(shared.release.clone()))
            .await
            .unwrap()
            .manifest,
        shared
    );
    assert_eq!(
        publish(&root, &supplier, Command::Observe(removed.release.clone())).await,
        Err(Error::Store(store::Error::NotFound))
    );
    let info: store::Info = root
        .update::<_, wire::InfoReply>(
            "observe_store",
            wire::INFO,
            &wire::InfoRequest {
                store: supplier.id.to_text(),
                expires_at_ns: input.expires_at_ns,
            },
            STORE_LIMITS,
        )
        .await
        .unwrap();
    assert_eq!(info.releases, 3); // Removed labels still consume quota.
    maintain(&root).await;
    assert_eq!(pic.get_stable_memory(root.id).await, root_before);
    // Expire original dispatch authority as well as retaining the successful result.
    pic.advance_time(Duration::from_secs(31)).await;
    pic.tick().await;
    let restored = publish(&root, &supplier, Command::Prepare(settled))
        .await
        .unwrap();
    assert_eq!(restored.manifest, removed);
    assert!(restored.published); // Shared live alias retained the verified bytes.
    assert_eq!(remove(&root, &input).await, Err(Error::Expired));
    assert_eq!(
        publish(&root, &supplier, Command::Observe(removed.release.clone()))
            .await
            .unwrap(),
        restored
    );
    let approval = canic_management_protocol::store_maintenance::Approval {
        store: supplier.id.to_text(),
        previous_module: [1; 32],
        module_hash: [2; 32],
        attempt: store::MaintenanceAttempt {
            sequence: 1,
            operation: [57; 32],
        },
        expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 240_000_000_000,
    };
    root.update::<_, Result<
        canic_management_protocol::store_maintenance::Status,
        canic_management_protocol::Refusal,
    >>(
        "stage_store_maintenance",
        canic_management_protocol::store_maintenance::STAGE,
        &approval,
        canic_management_protocol::store_maintenance::LIMITS,
    )
    .await
    .unwrap();
    let fresh = request(&root, &supplier, Removal::Delete, removed).await;
    let before = pic.get_stable_memory(supplier.id).await;
    assert_eq!(remove(&root, &fresh).await, Err(Error::Maintenance));
    assert_eq!(pic.get_stable_memory(supplier.id).await, before);
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

#[tokio::test(flavor = "current_thread")]
async fn root_cancellation_preserves_tombstone_and_lost_reply_never_becomes_a_receipt() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let (root, supplier) = readiness_tests::fixture(pic).await;
    handover(&root, &supplier).await;
    let bytes = b"\0asm\x01\0\0\0";
    let manifest = store::Manifest {
        release: store::Release {
            name: "greeting".into(),
            version: "pending".into(),
        },
        hash: store::digest(bytes),
        bytes: bytes.len() as u64,
        chunks: vec![store::digest(bytes)],
    };
    publish(&root, &supplier, Command::Prepare(manifest.clone()))
        .await
        .unwrap();
    let input = request(&root, &supplier, Removal::Cancel, manifest.clone()).await;
    assert_eq!(remove(&root, &input).await.unwrap(), manifest);
    pic.advance_time(Duration::from_secs(31)).await;
    pic.tick().await;
    let restored = publish(&root, &supplier, Command::Prepare(manifest.clone()))
        .await
        .unwrap();
    assert!(!restored.published);
    assert_eq!(restored.missing, vec![0]);
    assert_eq!(remove(&root, &input).await, Err(Error::Expired));
    let input = request(&root, &supplier, Removal::Cancel, manifest).await;
    inject(&root, 12).await;
    let frame = ic_wire::encode(
        &input,
        ic_wire::Encoding::Frame {
            method: wire::REMOVE,
            direction: ic_wire::Direction::Request,
        },
        STORE_LIMITS,
    )
    .unwrap();
    assert!(
        pic.update_call(root.id, root.operator, "store_removal", frame)
            .await
            .is_err()
    );
    inject(&root, 0).await;
    // Store committed, but no successful removal reply was retained. Do not restore
    // or dispatch a replacement removal merely because this observation says absent.
    assert_eq!(
        publish(&root, &supplier, Command::Observe(input.manifest.release)).await,
        Err(Error::Store(store::Error::NotFound))
    );
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
