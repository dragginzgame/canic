//! Actual Root is the Store caller and sole child controller, not an impersonated ingress sender.
mod admission_tests;
mod dispatch_tests;
mod maintenance_tests;
mod publication_tests;
mod quiescence_tests;
mod readiness_tests;
mod root_custody_tests;
use super::{Client, store, support};
use canic_management_protocol::{
    self as management,
    children::{self, Request, Slot},
    store_maintenance, stored, upgrade,
};
use ic_testkit::pocket_ic::{CanisterId, CanisterSettings, nonblocking::PocketIc};
use std::{path::PathBuf, time::Duration};

fn artifacts() -> PathBuf {
    PathBuf::from(std::env::var_os("CANIC_STORE_APP_DIRECTORY").expect("make store-test"))
}
async fn recover_store(
    root: &Client<'_>,
    approval: &store_maintenance::Approval,
) -> store_maintenance::Status {
    root.update(
        "recover_store_maintenance",
        store_maintenance::RECOVER,
        approval,
        store_maintenance::LIMITS,
    )
    .await
}
async fn settle(pic: &PocketIc) {
    for _ in 0..100 {
        pic.advance_time(Duration::from_millis(100)).await;
        pic.tick().await;
    }
}
async fn custody(root: &Client<'_>) -> stored::Custody {
    root.update(
        "observe_stored_children",
        stored::CUSTODY,
        &(),
        stored::LIMITS,
    )
    .await
}
async fn inject(root: &Client<'_>, point: u8) {
    root.update::<_, ()>(
        "inject_child_fault",
        children::INJECT,
        &Request {
            slot: Slot::First,
            value: point,
        },
        management::LIMITS,
    )
    .await;
}
async fn maintain(root: &Client<'_>) {
    root.pic
        .stop_canister(root.id, Some(root.operator))
        .await
        .unwrap();
    root.pic
        .upgrade_canister(
            root.id,
            std::fs::read(artifacts().join("root-maintenance.wasm")).unwrap(),
            vec![],
            Some(root.operator),
        )
        .await
        .unwrap();
    root.pic
        .start_canister(root.id, Some(root.operator))
        .await
        .unwrap();
}
fn padded(mut wasm: Vec<u8>) -> Vec<u8> {
    // A valid custom section makes the actual executable larger than Root's former byte limit.
    let payload = 1_048_577_u32;
    wasm.push(0);
    let mut length = payload;
    loop {
        let byte = (length & 127) as u8;
        length >>= 7;
        wasm.push(byte | if length == 0 { 0 } else { 128 });
        if length == 0 {
            break;
        }
    }
    wasm.push(0); // Empty custom-section name.
    wasm.resize(wasm.len() + payload as usize - 1, 42);
    wasm
}
async fn fixture<'a>(
    pic: &'a PocketIc,
    fault: bool,
) -> (
    Client<'a>,
    Client<'a>,
    stored::Stage<management::Approval>,
    store::Manifest,
) {
    let operator = CanisterId::self_authenticating(b"Root Store fixture");
    let root = pic
        .create_canister_with_settings(Some(operator), None)
        .await;
    let store_id = pic
        .create_canister_with_settings(Some(operator), None)
        .await;
    pic.add_cycles(root, 10_000_000_000_000).await;
    pic.add_cycles(store_id, 10_000_000_000_000).await;
    pic.install_canister(
        root,
        std::fs::read(artifacts().join(if fault {
            "root-faults.wasm"
        } else {
            "root.wasm"
        }))
        .unwrap(),
        vec![],
        Some(operator),
    )
    .await;
    pic.install_canister(
        store_id,
        std::fs::read(artifacts().join("original.wasm")).unwrap(),
        vec![],
        Some(operator),
    )
    .await;
    let supplier = Client {
        pic,
        id: store_id,
        operator,
    };
    let first = supplier
        .publish_bytes(
            "root-1",
            &padded(std::fs::read(artifacts().join("app.wasm")).unwrap()),
        )
        .await;
    let second = supplier
        .publish_bytes(
            "root-2",
            &padded(std::fs::read(artifacts().join("app-v2.wasm")).unwrap()),
        )
        .await;
    // Root alone controls the Store during autonomous work; upload ingress is finished.
    pic.update_canister_settings(
        store_id,
        Some(operator),
        CanisterSettings {
            controllers: Some(vec![root]),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let input = stored::Stage {
        approval: management::Approval {
            manager: root.to_text(),
            module_hash: first.hash,
            creation_cycles: management::CREATION_CYCLES,
            execute_at_ns: now + 10_000_000_000,
            expires_at_ns: now + 240_000_000_000,
        },
        reference: stored::Reference {
            store: store_id.to_text(),
            manifest: first,
        },
    };
    (
        Client {
            pic,
            id: root,
            operator,
        },
        supplier,
        input,
        second,
    )
}
async fn stage(
    root: &Client<'_>,
    input: &stored::Stage<management::Approval>,
) -> Result<management::Status, management::Refusal> {
    root.update(
        "stage_stored_child",
        stored::STAGE,
        &Request {
            slot: Slot::First,
            value: input,
        },
        stored::LIMITS,
    )
    .await
}
async fn reconcile(
    root: &Client<'_>,
    input: &stored::Stage<management::Approval>,
) -> management::Status {
    root.update(
        "reconcile_stored_child",
        stored::RECONCILE,
        &Request {
            slot: Slot::First,
            value: input,
        },
        stored::LIMITS,
    )
    .await
}

#[tokio::test(flavor = "current_thread")]
async fn root_creates_upgrades_and_preserves_store_custody_without_retaining_wasm() {
    let (_, binary) = support::inputs();
    let owner = support::instance(&binary).await;
    let pic = owner.instance();
    let (root, supplier, input, second) = fixture(pic, true).await;
    stage(&root, &input).await.unwrap();
    let staged = custody(&root).await;
    assert_eq!(
        staged.sources[0].as_ref().unwrap().creation.state,
        stored::PinState::Pending
    );
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await; // No operator calls while Root pins, creates, prepares and installs.
    let ready = custody(&root).await;
    assert_eq!(
        ready.children[0].creation.phase,
        management::Phase::Ready,
        "{ready:?}"
    );
    assert_eq!(ready.children[0].creation.attempts, [1; 3]);
    assert_eq!(
        ready.sources[0].as_ref().unwrap().creation.state,
        stored::PinState::Retained
    );
    assert!(
        pic.stored_chunks(root.id, Some(root.operator))
            .await
            .unwrap()
            .is_empty()
    );
    let worker =
        CanisterId::from_text(ready.children[0].creation.worker.as_ref().unwrap()).unwrap();
    let installed = pic.canister_status(worker, Some(root.id)).await.unwrap();
    assert_eq!(installed.settings.controllers, [root.id]);
    assert_eq!(
        installed.module_hash,
        Some(input.reference.manifest.hash.to_vec())
    );
    assert_eq!(
        pic.get_subnet(worker).await,
        pic.get_subnet(supplier.id).await
    );
    assert!(
        !pic.stored_chunks(supplier.id, Some(root.id))
            .await
            .unwrap()
            .is_empty()
    );
    // Exact replay is observational. Changing the Store under the same old approval is refused.
    assert_eq!(stage(&root, &input).await.unwrap().attempts, [1; 3]);
    let mut changed = input.clone();
    changed.reference.store = worker.to_text();
    assert_eq!(
        stage(&root, &changed).await,
        Err(management::Refusal::Occupied)
    );
    assert!(
        pic.update_call(
            root.id,
            root.operator,
            "observe_children",
            ic_wire::native::encode_request(children::CUSTODY, &(), management::LIMITS).unwrap()
        )
        .await
        .is_err()
    );
    assert!(
        pic.update_call(
            root.id,
            root.operator,
            "reconcile_child",
            ic_wire::native::encode_request(
                children::RECONCILE,
                &Request {
                    slot: Slot::First,
                    value: &input.approval
                },
                management::LIMITS
            )
            .unwrap()
        )
        .await
        .is_err()
    );
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let upgrade = stored::Stage {
        approval: upgrade::Approval {
            manager: root.id.to_text(),
            child: worker.to_text(),
            previous_module: input.approval.module_hash,
            module_hash: second.hash,
            execute_at_ns: now + 5_000_000_000,
            expires_at_ns: now + 200_000_000_000,
        },
        reference: stored::Reference {
            store: supplier.id.to_text(),
            manifest: second,
        },
    };
    // Lose the cache completion callback, retaining only its original dispatch intent.
    inject(&root, 6).await;
    root.update::<_, Result<upgrade::Status, management::Refusal>>(
        "stage_stored_upgrade",
        stored::STAGE_UPGRADE,
        &Request {
            slot: Slot::First,
            value: &upgrade,
        },
        stored::LIMITS,
    )
    .await
    .unwrap();
    pic.advance_time(Duration::from_secs(6)).await;
    settle(pic).await;
    let interrupted = custody(&root).await;
    assert_eq!(
        interrupted.children[0].upgrades[0].phase,
        upgrade::Phase::UploadIssued
    );
    maintain(&root).await;
    assert_eq!(custody(&root).await, interrupted);
    root.update::<_, upgrade::Status>(
        "reconcile_stored_upgrade",
        stored::RECONCILE_UPGRADE,
        &Request {
            slot: Slot::First,
            value: &upgrade,
        },
        stored::LIMITS,
    )
    .await;
    settle(pic).await;
    let upgraded = custody(&root).await;
    assert_eq!(
        upgraded.children[0].upgrades[0].phase,
        upgrade::Phase::Ready,
        "{upgraded:?}"
    );
    assert_eq!(upgraded.children[0].upgrades[0].attempts, [1; 2]);
    assert_eq!(
        upgraded.sources[0].as_ref().unwrap().upgrades[0].reference,
        upgrade.reference
    );
    assert_eq!(
        pic.canister_status(worker, Some(root.id))
            .await
            .unwrap()
            .module_hash,
        Some(upgrade.approval.module_hash.to_vec())
    );
    assert!(
        pic.stored_chunks(root.id, Some(root.operator))
            .await
            .unwrap()
            .is_empty()
    );
    maintain(&root).await;
    assert_eq!(custody(&root).await, upgraded);
    // A second upgrade retains the first source; an inline sibling still uses its old codec.
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let successor = stored::Stage {
        approval: upgrade::Approval {
            previous_module: upgrade.approval.module_hash,
            module_hash: input.approval.module_hash,
            execute_at_ns: now + 5_000_000_000,
            expires_at_ns: now + 200_000_000_000,
            ..upgrade.approval.clone()
        },
        reference: input.reference.clone(),
    };
    root.update::<_, Result<upgrade::Status, management::Refusal>>(
        "stage_stored_upgrade",
        stored::STAGE_UPGRADE,
        &Request {
            slot: Slot::First,
            value: &successor,
        },
        stored::LIMITS,
    )
    .await
    .unwrap();
    let wasm = std::fs::read(artifacts().join("app.wasm")).unwrap();
    let inline = management::Stage {
        approval: management::Approval {
            module_hash: management::digest(&wasm),
            execute_at_ns: now + 5_000_000_000,
            expires_at_ns: now + 200_000_000_000,
            ..input.approval.clone()
        },
        wasm,
    };
    root.update::<_, Result<management::Status, management::Refusal>>(
        "stage_child",
        children::STAGE,
        &Request {
            slot: Slot::Second,
            value: inline,
        },
        management::STAGE_LIMITS,
    )
    .await
    .unwrap();
    pic.advance_time(Duration::from_secs(6)).await;
    settle(pic).await;
    let history = custody(&root).await;
    assert_eq!(history.children[0].upgrades.len(), 2);
    assert_eq!(
        history.children[0].upgrades[0],
        upgraded.children[0].upgrades[0]
    );
    assert_eq!(history.children[0].upgrades[1].phase, upgrade::Phase::Ready);
    assert_eq!(
        history.sources[0].as_ref().unwrap().upgrades[0],
        upgraded.sources[0].as_ref().unwrap().upgrades[0]
    );
    assert_eq!(history.children[1].creation.phase, management::Phase::Ready);
    assert!(history.sources[1].is_none());
    let old = root
        .update::<_, Result<upgrade::Status, management::Refusal>>(
            "stage_stored_upgrade",
            stored::STAGE_UPGRADE,
            &Request {
                slot: Slot::First,
                value: &upgrade,
            },
            stored::LIMITS,
        )
        .await
        .unwrap();
    assert_eq!(old, history.children[0].upgrades[0]);
    maintain(&root).await;
    assert_eq!(custody(&root).await, history);
}

#[tokio::test(flavor = "current_thread")]
async fn lost_pin_and_creation_cache_callbacks_recover_without_reissuing_effects() {
    for fault in [11, 4] {
        let (_, binary) = support::inputs();
        let owner = support::instance(&binary).await;
        let pic = owner.instance();
        let (root, _, input, _) = fixture(pic, true).await;
        inject(&root, fault).await;
        stage(&root, &input).await.unwrap();
        pic.advance_time(Duration::from_secs(11)).await;
        settle(pic).await;
        let before = custody(&root).await;
        let expected = if fault == 11 {
            management::Phase::Scheduled
        } else {
            management::Phase::UploadIssued
        };
        assert_eq!(before.children[0].creation.phase, expected);
        if fault == 11 {
            assert_eq!(
                before.sources[0].as_ref().unwrap().creation.state,
                stored::PinState::Issued
            );
        }
        maintain(&root).await;
        assert_eq!(custody(&root).await, before);
        reconcile(&root, &input).await;
        settle(pic).await;
        let after = custody(&root).await;
        assert_eq!(
            after.children[0].creation.phase,
            management::Phase::Ready,
            "{after:?}"
        );
        assert_eq!(after.children[0].creation.attempts, [1; 3]);
        assert_eq!(
            after.sources[0].as_ref().unwrap().creation.state,
            stored::PinState::Retained
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn missing_cache_and_expired_authority_remain_bounded_without_install_replay() {
    for fault in [4, 11] {
        let (_, binary) = support::inputs();
        let owner = support::instance(&binary).await;
        let pic = owner.instance();
        let (root, supplier, input, _) = fixture(pic, true).await;
        inject(&root, fault).await;
        stage(&root, &input).await.unwrap();
        pic.advance_time(Duration::from_secs(11)).await;
        settle(pic).await;
        if fault == 4 {
            // Explicit fixture fault in the derived cache, not Root's production control flow.
            pic.clear_chunk_store(supplier.id, Some(root.id))
                .await
                .unwrap();
        }
        maintain(&root).await;
        pic.advance_time(Duration::from_secs(250)).await;
        reconcile(&root, &input).await;
        settle(pic).await;
        let observed = custody(&root).await;
        let journal = &observed.children[0].creation;
        if fault == 4 {
            assert_eq!(journal.phase, management::Phase::UploadIssued);
            assert_eq!(journal.failure, Some(management::Failure::Mismatch));
            assert_eq!(journal.attempts, [1, 1, 0]);
        } else {
            assert_eq!(journal.phase, management::Phase::Expired);
            assert_eq!(journal.attempts, [0; 3]);
        }
        reconcile(&root, &input).await;
        settle(pic).await;
        assert_eq!(custody(&root).await, observed);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn unpublished_reference_cannot_authorize_child_creation() {
    let (_, binary) = support::inputs();
    let owner = support::instance(&binary).await;
    let pic = owner.instance();
    let (root, _, mut input, _) = fixture(pic, false).await;
    input.reference.manifest.release.version = "absent".into();
    stage(&root, &input).await.unwrap();
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    let blocked = custody(&root).await;
    assert_eq!(
        blocked.children[0].creation.phase,
        management::Phase::Scheduled
    );
    assert_eq!(blocked.children[0].creation.attempts, [0; 3]);
    assert!(blocked.children[0].creation.worker.is_none());
    assert!(blocked.children[0].creation.failure.is_some());
    maintain(&root).await;
    reconcile(&root, &input).await;
    settle(pic).await;
    assert_eq!(custody(&root).await, blocked);
}
