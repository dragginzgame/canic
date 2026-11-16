//! Actual Host Root replacement preserves the separate, still-closed Store operation.
use super::{Client, artifacts, fixture, quiescence_tests, recover_store, settle, stage, support};
use canic_management_protocol::{self as management, custody, store_maintenance as maintenance};
use ic_testkit::pocket_ic::PocketIcBuilder;
use ic_testkit::pocket_ic::{CanisterId, CanisterSettings};
use ic_testkit::server::{Limits, Server};
use std::{
    path::Path,
    process::Command,
    time::{Duration, SystemTime},
};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
async fn complete(root: &Client<'_>) -> custody::Root {
    root.update(
        "observe_root_custody",
        custody::ROOT,
        &(),
        custody::ROOT_LIMITS,
    )
    .await
}
#[tokio::test(flavor = "current_thread")]
async fn host_root_maintenance_preserves_stopped_store_operation_and_signed_install() {
    let (_, binary) = support::inputs();
    let mut server = Server::start(
        ic_testkit::server::PreparedServer::check(&binary).unwrap(),
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(120),
            stdout_bytes: 1024 * 1024,
            stderr_bytes: 1024 * 1024,
        },
    )
    .unwrap();
    let mut owner = server
        .instance(
            PocketIcBuilder::new()
                .with_nns_subnet()
                .with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    // Drive the bounded timer fixture before starting the real-time gateway.
    // A recent baseline avoids charging years of storage when make_live catches up.
    owner
        .client()
        .set_time((SystemTime::now() - Duration::from_secs(60)).into())
        .await;
    let (root_id, store_id, operator, approval, expected, root_bytes, store_bytes) = {
        let pic = owner.client();
        let (root, supplier, child, _) = fixture(pic, true).await;
        stage(&root, &child).await.unwrap();
        pic.advance_time(Duration::from_secs(11)).await;
        settle(pic).await;
        let before = complete(&root).await;
        assert_eq!(
            before.stored.children[0].creation.phase,
            management::Phase::Ready
        );
        assert!(before.store_operation.is_none());
        assert_eq!(before.stored, super::custody(&root).await);
        let approval = quiescence_tests::approval(&root, &supplier).await;
        root.update::<_, Result<maintenance::Status, management::Refusal>>(
            "stage_store_maintenance",
            maintenance::STAGE,
            &approval,
            maintenance::LIMITS,
        )
        .await
        .unwrap();
        for _ in 0..16 {
            let value: maintenance::Status = root
                .update(
                    "advance_store_maintenance",
                    maintenance::ADVANCE,
                    &approval,
                    maintenance::LIMITS,
                )
                .await;
            if value.phase == maintenance::Phase::Stopped {
                break;
            }
        }
        let expected = complete(&root).await;
        assert_eq!(expected.stored, before.stored);
        assert_eq!(
            expected.store_operation.as_ref().unwrap().phase,
            maintenance::Phase::Stopped
        );
        let root_bytes = pic.get_stable_memory(root.id).await;
        let store_bytes = pic.get_stable_memory(supplier.id).await;
        for (caller, frame) in [
            (
                CanisterId::anonymous(),
                ic_wire::encode(
                    &(),
                    ic_wire::Encoding::Frame {
                        method: custody::ROOT,
                        direction: ic_wire::Direction::Request,
                    },
                    custody::ROOT_LIMITS,
                )
                .unwrap(),
            ),
            (root.operator, vec![0, 1, 2]),
        ] {
            assert!(
                pic.update_call(root.id, caller, "observe_root_custody", frame)
                    .await
                    .is_err()
            );
        }
        // An incomplete custody reader must still refuse; the new endpoint is explicit.
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
        assert_eq!(pic.get_stable_memory(root.id).await, root_bytes);
        // Local test-only maintenance authority handover. The captured Store operator
        // must survive unchanged and receives control back before recovery below.
        pic.update_canister_settings(
            root.id,
            Some(root.operator),
            CanisterSettings {
                controllers: Some(vec![CanisterId::anonymous()]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        (
            root.id,
            supplier.id,
            root.operator,
            approval,
            expected,
            root_bytes,
            store_bytes,
        )
    };
    let url = owner.client_mut().make_live(None).await.to_string();
    let pic = owner.client();
    let stopped_status = pic
        .canister_status(store_id, Some(root_id))
        .await
        .unwrap()
        .status;
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let directory = repo
        .join("target/store")
        .join(format!("root-custody-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let input = directory.join("input.json");
    std::fs::write(&input, serde_json::to_vec(&serde_json::json!({
        "network": {"kind":"Local", "url":url, "root_key":hex(&pic.root_key().await.unwrap()), "routing_canister":root_id.to_text()},
        "root":root_id.to_text(), "store":store_id.to_text(),
        "expected":expected, "artifacts":artifacts(),
    })).unwrap()).unwrap();
    let result = Command::new(
        std::env::var_os("CANIC_HOST_CUSTODY_TEST").expect("run test-store.sh root-maintenance"),
    )
    .args([
        "maintenance::installed_tests::root_maintenance_preserves_store_operation",
        "--exact",
        "--ignored",
        "--nocapture",
    ])
    .env("CANIC_ROOT_CUSTODY_INPUT", &input)
    .output()
    .unwrap();
    assert!(
        result.status.success(),
        "Host witness failed:\n{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        pic.canister_status(root_id, Some(CanisterId::anonymous()))
            .await
            .unwrap()
            .module_hash,
        Some(
            management::digest(&std::fs::read(artifacts().join("root-maintenance.wasm")).unwrap())
                .to_vec()
        )
    );
    assert_eq!(pic.get_stable_memory(root_id).await, root_bytes);
    assert_eq!(pic.get_stable_memory(store_id).await, store_bytes);
    pic.update_canister_settings(
        root_id,
        Some(CanisterId::anonymous()),
        CanisterSettings {
            controllers: Some(vec![operator]),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let root = Client {
        pic,
        id: root_id,
        operator,
    };
    assert_eq!(complete(&root).await, expected);
    assert_eq!(
        recover_store(&root, &approval).await,
        expected.store_operation.unwrap()
    );
    assert_eq!(
        pic.canister_status(store_id, Some(root_id))
            .await
            .unwrap()
            .status,
        stopped_status
    );
    owner.client_mut().stop_live().await;
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    let stopped = server.stop().unwrap();
    assert!(stopped.process.cleanup.is_empty(), "{stopped:?}");
}
