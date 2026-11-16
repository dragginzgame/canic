//! Only actual Root calls may close and stop its Store. Lost callbacks retain external intent.
use super::{Client, artifacts, inject, maintain, readiness_tests, recover_store, store, support};
use canic_management_protocol::{
    self as management, publication,
    store_maintenance::{self as wire, Approval, Phase, Status},
};
use ic_testkit::pocket_ic::{CanisterId, CanisterSettings};
use std::time::Duration;

pub(super) async fn approval(root: &Client<'_>, supplier: &Client<'_>) -> Approval {
    Approval {
        store: supplier.id.to_text(),
        previous_module: management::digest(
            &std::fs::read(artifacts().join("original.wasm")).unwrap(),
        ),
        module_hash: management::digest(
            &std::fs::read(artifacts().join("candidate.wasm")).unwrap(),
        ),
        attempt: store::MaintenanceAttempt {
            sequence: 1,
            operation: [21; 32],
        },
        expires_at_ns: root.pic.get_time().await.as_nanos_since_unix_epoch() + 240_000_000_000,
    }
}
async fn observe(root: &Client<'_>) -> Option<Status> {
    root.update(
        "observe_store_maintenance",
        wire::OBSERVE,
        &(),
        wire::LIMITS,
    )
    .await
}
async fn advance(root: &Client<'_>, approval: &Approval) -> Status {
    root.update(
        "advance_store_maintenance",
        wire::ADVANCE,
        approval,
        wire::LIMITS,
    )
    .await
}
pub(super) async fn child_stage(
    root: &Client<'_>,
) -> Result<management::Status, management::Refusal> {
    root.update(
        "stage",
        management::STAGE,
        &management::Stage {
            approval: management::Approval {
                manager: root.id.to_text(),
                module_hash: management::digest(b"\0asm\x01\0\0\0"),
                creation_cycles: management::CREATION_CYCLES,
                execute_at_ns: root.pic.get_time().await.as_nanos_since_unix_epoch()
                    + 120_000_000_000,
                expires_at_ns: root.pic.get_time().await.as_nanos_since_unix_epoch()
                    + 240_000_000_000,
            },
            wasm: b"\0asm\x01\0\0\0".to_vec(),
        },
        management::STAGE_LIMITS,
    )
    .await
}

#[tokio::test(flavor = "current_thread")]
async fn root_retains_closure_and_stop_intent_across_lost_callbacks_and_root_upgrade() {
    let (_, binary) = support::inputs();
    let owner = support::instance(&binary).await;
    let pic = owner.instance();
    let (root, supplier) = readiness_tests::fixture(pic).await;
    let first = supplier.publish_bytes("kept", b"\0asm\x01\0\0\0").await;
    supplier
        .pin(&store::Pin {
            operation: [91; 32],
            manifest: first.clone(),
        })
        .await;
    let pending = store::Manifest {
        release: store::Release {
            name: "unfinished".into(),
            version: "v1".into(),
        },
        hash: store::digest(b"\0asm\x01\0\0\0\0"),
        bytes: 9,
        chunks: vec![store::digest(b"\0asm\x01\0\0\0\0")],
    };
    supplier
        .update::<_, Result<store::Status, store::Error>>(
            "prepare",
            store::PREPARE,
            &pending,
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
    let input = approval(&root, &supplier).await;
    let stage_frame = ic_wire::encode(
        &input,
        ic_wire::Encoding::Frame {
            method: wire::STAGE,
            direction: ic_wire::Direction::Request,
        },
        wire::LIMITS,
    )
    .unwrap();
    assert!(
        pic.update_call(
            root.id,
            CanisterId::anonymous(),
            "stage_store_maintenance",
            stage_frame
        )
        .await
        .is_err()
    );
    let reserved: Result<Status, management::Refusal> = root
        .update("stage_store_maintenance", wire::STAGE, &input, wire::LIMITS)
        .await;
    let reserved = reserved.unwrap();
    assert_eq!(reserved.phase, Phase::Reserved);
    assert_eq!(recover_store(&root, &input).await, reserved); // Recovery cannot close Store.
    assert_eq!(child_stage(&root).await, Err(management::Refusal::Occupied));
    let raw = publication::AdmissionRequest {
        store: input.store.clone(),
        expires_at_ns: input.expires_at_ns,
        command: store::AdmissionCommand::Activate(input.attempt.clone()),
    };
    assert_eq!(
        root.update::<_, publication::AdmissionReply>(
            "store_admission",
            publication::ADMISSION,
            &raw,
            store::LIMITS
        )
        .await,
        Err(publication::Error::Maintenance)
    );
    let publication = publication::Request {
        store: input.store.clone(),
        expires_at_ns: input.expires_at_ns,
        command: publication::Command::Publish(store::Release {
            name: "greeting".into(),
            version: "kept".into(),
        }),
    };
    assert_eq!(
        root.update::<_, publication::Reply>(
            "store_publication",
            publication::EXECUTE,
            &publication,
            publication::REQUEST_LIMITS
        )
        .await,
        Err(publication::Error::Maintenance)
    );
    inject(&root, 13).await;
    let frame = ic_wire::encode(
        &input,
        ic_wire::Encoding::Frame {
            method: wire::ADVANCE,
            direction: ic_wire::Direction::Request,
        },
        wire::LIMITS,
    )
    .unwrap();
    assert!(
        pic.update_call(
            root.id,
            root.operator,
            "advance_store_maintenance",
            frame.clone()
        )
        .await
        .is_err()
    );
    assert_eq!(observe(&root).await.unwrap().phase, Phase::Closing);
    inject(&root, 0).await;
    let closed = recover_store(&root, &input).await;
    assert_eq!(closed.phase, Phase::Closed);
    assert_eq!(recover_store(&root, &input).await, closed);
    let first_page = advance(&root, &input).await;
    assert_eq!(first_page.preservation.as_ref().unwrap().observed, 1);
    assert!(!first_page.preservation.as_ref().unwrap().complete);
    maintain(&root).await;
    assert_eq!(observe(&root).await.unwrap(), first_page);
    // Concurrent responses to this same cursor must not double-count or skip an entry.
    let (_, _) = tokio::join!(advance(&root, &input), advance(&root, &input));
    let mut scanned = observe(&root).await.unwrap();
    for _ in 0..3 {
        if scanned.preservation.as_ref().unwrap().complete {
            break;
        }
        scanned = advance(&root, &input).await;
    }
    let preservation = scanned.preservation.as_ref().unwrap();
    assert!(preservation.complete);
    assert_eq!(preservation.observed, 2);
    assert_eq!(preservation.catalog.releases, 2);
    assert_eq!(preservation.catalog.artifacts, 2);
    assert_eq!(preservation.catalog.chunks, 1);
    assert_eq!(recover_store(&root, &input).await, scanned); // Complete pre-scan cannot authorize stop through recovery.
    // The pre-existing maintenance helper installs a non-fault build. Restore the fault build
    // without changing the journal so the stop callback still exercises its failure boundary.
    pic.upgrade_canister(
        root.id,
        std::fs::read(artifacts().join("root-faults.wasm")).unwrap(),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
    let closed_store = pic.get_stable_memory(supplier.id).await;
    inject(&root, 14).await;
    assert!(
        pic.update_call(root.id, root.operator, "advance_store_maintenance", frame)
            .await
            .is_err()
    );
    let issued = observe(&root).await.unwrap();
    assert_eq!(issued.phase, Phase::StopIssued);
    assert!(issued.stop_request.is_some());
    // A Root upgrade must retain the external journal while its Store remains stopped.
    maintain(&root).await;
    assert_eq!(observe(&root).await.unwrap(), issued);
    pic.advance_time(Duration::from_secs(3601)).await;
    let stopped = recover_store(&root, &input).await;
    assert_eq!(stopped.phase, Phase::Stopped);
    assert_eq!(stopped.stop_request, issued.stop_request);
    assert!(stopped.failure.is_none());
    assert_eq!(advance(&root, &input).await, stopped);
    assert_eq!(pic.get_stable_memory(supplier.id).await, closed_store);
    let mut wrong = input.clone();
    wrong.attempt.operation[0] ^= 1;
    let refused: Result<Status, management::Refusal> = root
        .update("stage_store_maintenance", wire::STAGE, &wrong, wire::LIMITS)
        .await;
    assert_eq!(refused, Err(management::Refusal::Occupied));
    assert!(
        pic.update_call(
            root.id,
            root.operator,
            "observe_stored_children",
            ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method: management::stored::CUSTODY,
                    direction: ic_wire::Direction::Request
                },
                management::stored::LIMITS
            )
            .unwrap()
        )
        .await
        .is_err()
    );
    assert_eq!(observe(&root).await.unwrap(), stopped);
    owner.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn maintenance_refuses_unfinished_children_and_checks_live_store_authority_before_closure() {
    let (_, binary) = support::inputs();
    let owner = support::instance(&binary).await;
    let pic = owner.instance();
    let (busy, supplier) = readiness_tests::fixture(pic).await;
    child_stage(&busy).await.unwrap();
    let input = approval(&busy, &supplier).await;
    let refused: Result<Status, management::Refusal> = busy
        .update("stage_store_maintenance", wire::STAGE, &input, wire::LIMITS)
        .await;
    assert_eq!(refused, Err(management::Refusal::Occupied));
    assert!(observe(&busy).await.is_none());
    let (root, supplier) = readiness_tests::fixture(pic).await;
    let input = approval(&root, &supplier).await;
    root.update::<_, Result<Status, management::Refusal>>(
        "stage_store_maintenance",
        wire::STAGE,
        &input,
        wire::LIMITS,
    )
    .await
    .unwrap();
    let before = pic.get_stable_memory(supplier.id).await;
    let refused = advance(&root, &input).await; // Store has not handed authority to Root.
    assert_eq!(refused.phase, Phase::Reserved);
    assert!(refused.failure.is_some());
    assert_eq!(pic.get_stable_memory(supplier.id).await, before);
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
    pic.advance_time(Duration::from_secs(3601)).await;
    assert_eq!(advance(&root, &input).await.phase, Phase::Reserved); // Expiry grants no new close.
    assert_eq!(pic.get_stable_memory(supplier.id).await, before);
    owner.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn cuj5_maintenance_migration_preserves_real_attempt_without_inventing_a_scan() {
    #[derive(serde::Deserialize)]
    struct Previous {
        approval: Approval,
        operator: String,
        phase: Phase,
        stop_request: Option<[u8; 32]>,
        failure: Option<management::Failure>,
    }
    let (_, binary) = support::inputs();
    let owner = support::instance(&binary).await;
    let pic = owner.instance();
    let predecessor = include_bytes!("../../fixtures/root-store-quiescence.wasm");
    for stopped in [false, true] {
        let (root, supplier) = readiness_tests::fixture_with_root(pic, predecessor.to_vec()).await;
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
        let input = approval(&root, &supplier).await;
        root.update::<_, Result<Previous, management::Refusal>>(
            "stage_store_maintenance",
            wire::STAGE,
            &input,
            wire::LIMITS,
        )
        .await
        .unwrap();
        let mut old: Previous = root
            .update(
                "advance_store_maintenance",
                wire::ADVANCE,
                &input,
                wire::LIMITS,
            )
            .await;
        assert_eq!(old.phase, Phase::Closed);
        if stopped {
            old = root
                .update(
                    "advance_store_maintenance",
                    wire::ADVANCE,
                    &input,
                    wire::LIMITS,
                )
                .await;
            assert_eq!(old.phase, Phase::StopIssued);
            old = root
                .update(
                    "advance_store_maintenance",
                    wire::ADVANCE,
                    &input,
                    wire::LIMITS,
                )
                .await;
            assert_eq!(old.phase, Phase::Stopped);
        }
        maintain(&root).await;
        let restored = observe(&root).await.unwrap();
        assert_eq!(restored.approval, old.approval);
        assert_eq!(restored.operator, old.operator);
        assert_eq!(restored.phase, old.phase);
        assert_eq!(restored.stop_request, old.stop_request);
        assert_eq!(restored.failure, old.failure);
        assert!(restored.preservation.is_none());
        let resumed = advance(&root, &input).await;
        if stopped {
            assert_eq!(resumed, restored);
        } else {
            let scan = resumed.preservation.as_ref().unwrap();
            assert!(scan.complete);
            assert_eq!(scan.observed, 0);
            assert_eq!(resumed.phase, Phase::Closed);
        }
    }
    owner.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn root_journal_migration_adds_no_store_authority_to_a_retained_predecessor() {
    let (_, binary) = support::inputs();
    let owner = support::instance(&binary).await;
    let pic = owner.instance();
    let operator = CanisterId::self_authenticating(b"Root journal predecessor");
    let id = pic
        .create_canister_with_settings(Some(operator), None)
        .await;
    pic.add_cycles(id, 10_000_000_000_000).await;
    pic.install_canister(
        id,
        include_bytes!("../../fixtures/root-one-child-history.wasm").to_vec(),
        vec![],
        Some(operator),
    )
    .await;
    let root = Client { pic, id, operator };
    maintain(&root).await;
    assert!(observe(&root).await.is_none());
    let custody: management::children::Custody = root
        .update(
            "observe_children",
            management::children::CUSTODY,
            &(),
            management::children::LIMITS,
        )
        .await;
    assert!(
        custody
            .iter()
            .all(|c| c.creation.approval.is_none() && c.upgrades.is_empty())
    );
    owner.close().await;
}
