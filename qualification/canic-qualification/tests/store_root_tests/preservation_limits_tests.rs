//! Installed supplier limitations that must be resolved before Canic admits removal.
use super::{Client, maintenance_tests, quiescence_tests, readiness_tests, store, support};
use canic_management_protocol::{self as management, store_maintenance as wire};
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
            store::LIMITS,
        )
        .await
        .unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn preservation_cannot_distinguish_removed_identity_from_restored_empty_upload() {
    let (_, binary) = support::inputs();
    let owner = support::instance(&binary).await;
    let (_, supplier) = readiness_tests::fixture(owner.instance()).await;
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
                store::LIMITS,
            )
            .await
            .unwrap();
    }
    supplier
        .update::<_, Result<store::Manifest, store::Error>>(
            "cancel",
            store::CANCEL,
            &removed,
            store::LIMITS,
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
            store::LIMITS,
        )
        .await
        .unwrap();
    let tombstone = page(&supplier, &attempt).await;
    assert_eq!(tombstone.entry.as_ref().unwrap().status.manifest, removed);
    assert_eq!(tombstone.catalog.releases, 2);
    supplier
        .update::<_, Result<Option<store::Admission>, store::Error>>(
            "admission",
            store::ADMISSION,
            &store::AdmissionCommand::Activate(attempt),
            store::LIMITS,
        )
        .await
        .unwrap();
    supplier
        .update::<_, Result<store::Status, store::Error>>(
            "prepare",
            store::PREPARE,
            &removed,
            store::LIMITS,
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
            store::LIMITS,
        )
        .await
        .unwrap();
    // Even the counters match: the incomplete alias kept the same reservation.
    // This is a supplier-contract reproducer, not a preservation success claim.
    assert_eq!(page(&supplier, &attempt).await, tombstone);
    let tombstones = supplier
        .update::<_, Result<Vec<store::Manifest>, store::Error>>(
            "tombstones",
            store::TOMBSTONES,
            &None::<store::Release>,
            store::LIMITS,
        )
        .await
        .unwrap();
    assert!(tombstones.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn root_refuses_non_monotonic_tombstone_catalog_before_stopping_store() {
    let (_, binary) = support::inputs();
    let owner = support::instance(&binary).await;
    let pic = owner.instance();
    let (root, supplier) = readiness_tests::fixture(pic).await;
    let removed = supplier.publish_bytes("a", b"\0asm\x01\0\0\0").await;
    supplier.publish_bytes("z", b"\0asm\x01\0\0\0").await;
    supplier
        .update::<_, Result<store::Manifest, store::Error>>(
            "delete",
            store::DELETE,
            &removed,
            store::LIMITS,
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
    // Duplicate continuation cannot bypass the refused cursor or issue stop.
    assert_eq!(maintenance_tests::advance(&root, &approval).await, status);
    assert_eq!(maintenance_tests::activate(&root, &approval).await, status);
}
