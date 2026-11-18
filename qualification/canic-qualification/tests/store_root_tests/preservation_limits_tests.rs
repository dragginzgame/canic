//! Released preservation contract and the remaining Root boundary before removal admission.
use super::{
    Client, artifacts, maintain, maintenance_tests, quiescence_tests, readiness_tests,
    recover_store, store, support,
};
use canic_management_protocol::{
    self as management, store_maintenance as wire, stored::STORE_LIMITS,
};
use ic_testkit::pocket_ic::CanisterSettings;

async fn page(
    supplier: &Client<'_>,
    attempt: &store::MaintenanceAttempt,
) -> store::preservation::Page {
    supplier
        .update::<_, Result<store::preservation::Page, store::Error>>(
            "observe_preservation",
            store::PRESERVATION,
            &store::preservation::Request {
                attempt: attempt.clone(),
                after: None,
            },
            STORE_LIMITS,
        )
        .await
        .unwrap()
}

async fn versioned_page(
    supplier: &Client<'_>,
    attempt: &store::MaintenanceAttempt,
) -> store::preservation::PageV2 {
    supplier
        .update::<_, Result<store::preservation::PageV2, store::Error>>(
            "observe_preservation_v2",
            store::PRESERVATION_V2,
            &store::preservation::Request {
                attempt: attempt.clone(),
                after: None,
            },
            STORE_LIMITS,
        )
        .await
        .unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn versioned_preservation_distinguishes_removal_without_reinterpreting_old_pages() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let (_, supplier) = readiness_tests::fixture(owner.client()).await;
    let bytes = b"\0asm\x01\0\0\0";
    let removed = store::Manifest {
        release: store::Release {
            name: "a".into(),
            version: "v1".into(),
        },
        hash: store::digest(bytes),
        bytes: bytes.len() as u64,
        chunks: vec![store::digest(bytes)],
    };
    let alias = store::Manifest {
        release: store::Release {
            name: "z".into(),
            ..removed.release.clone()
        },
        ..removed.clone()
    };
    for manifest in [&removed, &alias] {
        supplier
            .update::<_, Result<store::Status, store::Error>>(
                "prepare",
                store::PREPARE,
                manifest,
                STORE_LIMITS,
            )
            .await
            .unwrap();
    }
    supplier
        .update::<_, Result<store::Manifest, store::Error>>(
            "cancel",
            store::CANCEL,
            &removed,
            STORE_LIMITS,
        )
        .await
        .unwrap();
    let attempt = store::MaintenanceAttempt {
        sequence: 1,
        operation: [71; 32],
    };
    supplier
        .update::<_, Result<Option<store::Admission>, store::Error>>(
            "admission",
            store::ADMISSION,
            &store::AdmissionCommand::Close(attempt.clone()),
            STORE_LIMITS,
        )
        .await
        .unwrap();
    let tombstone = page(&supplier, &attempt).await;
    let versioned_tombstone = versioned_page(&supplier, &attempt).await;
    assert_eq!(versioned_tombstone.catalog, tombstone.catalog);
    assert_eq!(
        versioned_tombstone.entry,
        Some(store::preservation::Entry::Removed(removed.clone()))
    );
    assert_eq!(tombstone.entry.as_ref().unwrap().status.manifest, removed);
    assert_eq!(tombstone.catalog.releases, 2);
    supplier
        .update::<_, Result<Option<store::Admission>, store::Error>>(
            "admission",
            store::ADMISSION,
            &store::AdmissionCommand::Activate(attempt),
            STORE_LIMITS,
        )
        .await
        .unwrap();
    supplier
        .update::<_, Result<store::Status, store::Error>>(
            "prepare",
            store::PREPARE,
            &removed,
            STORE_LIMITS,
        )
        .await
        .unwrap();
    let attempt = store::MaintenanceAttempt {
        sequence: 2,
        operation: [72; 32],
    };
    supplier
        .update::<_, Result<Option<store::Admission>, store::Error>>(
            "admission",
            store::ADMISSION,
            &store::AdmissionCommand::Close(attempt.clone()),
            STORE_LIMITS,
        )
        .await
        .unwrap();
    // Even the counters match: the incomplete alias kept the same reservation.
    // The old wire contract is deliberately unchanged. Root cannot silently
    // reinterpret an in-progress digest as a commitment to the new entry kind.
    assert_eq!(page(&supplier, &attempt).await, tombstone);
    let restored = versioned_page(&supplier, &attempt).await;
    assert_eq!(restored.catalog, versioned_tombstone.catalog);
    let Some(store::preservation::Entry::Active(custody)) = restored.entry else {
        panic!("restored upload must be active, not a removed label");
    };
    assert_eq!(custody.status.manifest, removed);
    assert_eq!(Some(custody), tombstone.entry);
    let tombstones = supplier
        .update::<_, Result<Vec<store::Manifest>, store::Error>>(
            "tombstones",
            store::TOMBSTONES,
            &None::<store::Release>,
            STORE_LIMITS,
        )
        .await
        .unwrap();
    assert!(tombstones.is_empty());
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

#[tokio::test(flavor = "current_thread")]
async fn historical_scan_keeps_its_refusal_and_digest_after_refused_root_upgrade() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let (root, supplier) = readiness_tests::fixture_with_code(
        pic,
        include_bytes!("../../fixtures/root-release-scan.wasm").to_vec(),
        std::fs::read(artifacts().join("original.wasm")).unwrap(),
    )
    .await;
    let removed = supplier.publish_bytes("a", b"\0asm\x01\0\0\0").await;
    supplier.publish_bytes("z", b"\0asm\x01\0\0\0").await;
    supplier
        .update::<_, Result<store::Manifest, store::Error>>(
            "delete",
            store::DELETE,
            &removed,
            STORE_LIMITS,
        )
        .await
        .unwrap();
    pic.update_canister_settings(
        supplier.id,
        Some(root.operator),
        CanisterSettings {
            controllers: Some(vec![root.id]),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let approval = quiescence_tests::approval(&root, &supplier).await;
    root.update::<_, Result<wire::Status, management::Refusal>>(
        "stage_store_maintenance",
        wire::STAGE,
        &approval,
        wire::LIMITS,
    )
    .await
    .unwrap();
    let mut status = maintenance_tests::observe(&root).await;
    for _ in 0..8 {
        status = maintenance_tests::advance(&root, &approval).await;
        if status.failure.is_some() {
            break;
        }
    }
    assert_eq!(status.phase, wire::Phase::Closed);
    assert_eq!(status.failure, Some(management::Failure::Mismatch));
    let scan = status.preservation.as_ref().unwrap();
    assert!(!scan.complete);
    assert_eq!(scan.observed, 1);
    assert_eq!(scan.after.as_ref().unwrap().version, "z");
    assert_eq!(scan.catalog.releases, 2);
    assert!(matches!(status.preservation, wire::Scan::Legacy(_)));
    super::refuse_old_backing(&root).await;
    assert_eq!(maintenance_tests::observe(&root).await, status);
    // Duplicate continuation cannot bypass the refused cursor or issue stop.
    assert_eq!(maintenance_tests::advance(&root, &approval).await, status);
    assert_eq!(maintenance_tests::activate(&root, &approval).await, status);
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}

#[tokio::test(flavor = "current_thread")]
async fn root_preserves_removed_entries_and_cursor_across_both_owner_upgrades() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let (root, supplier) = readiness_tests::fixture(pic).await;
    let removed = supplier.publish_bytes("a", b"\0asm\x01\0\0\0").await;
    supplier.publish_bytes("z", b"\0asm\x01\0\0\0").await;
    supplier
        .update::<_, Result<store::Manifest, store::Error>>(
            "delete",
            store::DELETE,
            &removed,
            STORE_LIMITS,
        )
        .await
        .unwrap();
    let pending_bytes = b"\0asm\x01\0\0\0\0\x01\0";
    let pending = store::Manifest {
        release: store::Release {
            version: "m".into(),
            ..removed.release.clone()
        },
        hash: store::digest(pending_bytes),
        bytes: pending_bytes.len() as u64,
        chunks: vec![store::digest(pending_bytes)],
    };
    supplier
        .update::<_, Result<store::Status, store::Error>>(
            "prepare",
            store::PREPARE,
            &pending,
            STORE_LIMITS,
        )
        .await
        .unwrap();
    supplier
        .update::<_, Result<store::Manifest, store::Error>>(
            "cancel",
            store::CANCEL,
            &pending,
            STORE_LIMITS,
        )
        .await
        .unwrap();
    pic.update_canister_settings(
        supplier.id,
        Some(root.operator),
        CanisterSettings {
            controllers: Some(vec![root.id]),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let replacement = std::fs::read(artifacts().join("candidate.wasm")).unwrap();
    let mut approval = quiescence_tests::approval(&root, &supplier).await;
    approval.module_hash = store::digest(&replacement);
    root.update::<_, Result<wire::Status, management::Refusal>>(
        "stage_store_maintenance",
        wire::STAGE,
        &approval,
        wire::LIMITS,
    )
    .await
    .unwrap();
    assert_eq!(
        maintenance_tests::advance(&root, &approval).await.phase,
        wire::Phase::Closed
    );
    let first = maintenance_tests::advance(&root, &approval).await;
    assert!(matches!(first.preservation, wire::Scan::Entries(_)));
    assert_eq!(
        first
            .preservation
            .as_ref()
            .unwrap()
            .after
            .as_ref()
            .unwrap()
            .version,
        "z"
    );
    let removed_page = maintenance_tests::advance(&root, &approval).await;
    let cursor = removed_page.preservation.as_ref().unwrap();
    assert_eq!(cursor.after.as_ref(), Some(&removed.release));
    assert!(cursor.after_removed);
    assert_eq!(cursor.observed, 2);
    maintain(&root).await;
    assert_eq!(maintenance_tests::observe(&root).await, removed_page);
    // Downgrade cannot drop the scan discriminator or removed cursor.
    assert!(
        canic_qualification::upgrade_stopped(
            pic,
            root.id,
            include_bytes!("../../fixtures/root-release-scan.wasm").to_vec(),
            vec![],
            Some(root.operator)
        )
        .await
        .is_err()
    );
    assert_eq!(maintenance_tests::observe(&root).await, removed_page);
    for _ in 0..8 {
        if maintenance_tests::advance(&root, &approval).await.phase == wire::Phase::Stopped {
            break;
        }
    }
    let stopped = maintenance_tests::observe(&root).await;
    assert_eq!(stopped.phase, wire::Phase::Stopped);
    assert_eq!(stopped.preservation.as_ref().unwrap().observed, 3);
    let before = pic.get_stable_memory(supplier.id).await;
    maintenance_tests::install(
        &root,
        &wire::Install {
            approval: approval.clone(),
            wasm: replacement,
        },
    )
    .await
    .unwrap();
    for _ in 0..16 {
        if maintenance_tests::advance(&root, &approval).await.phase == wire::Phase::Verified {
            break;
        }
    }
    let verified = maintenance_tests::observe(&root).await;
    assert_eq!(verified.phase, wire::Phase::Verified);
    assert_eq!(verified.preservation, stopped.preservation);
    assert_eq!(
        verified.installation.as_ref().unwrap().verification,
        stopped.preservation
    );
    assert_eq!(pic.get_stable_memory(supplier.id).await, before);
    assert_eq!(maintenance_tests::advance(&root, &approval).await, verified);
    assert_eq!(
        maintenance_tests::activate(&root, &approval).await.phase,
        wire::Phase::ActivateIssued
    );
    let activated = recover_store(&root, &approval).await;
    assert_eq!(activated.phase, wire::Phase::Ready);
    assert_eq!(activated.preservation, verified.preservation);
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
