//! Strict opening refuses old/corrupt images and preserves the installed owner.
use super::*;
use ic_testkit::pocket_ic::{ErrorCode, common::rest::BlobCompression};

#[tokio::test]
async fn old_and_invalid_backings_refuse_without_initialization() {
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
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
    let current = children::artifact("root.wasm");
    for (name, expected) in [
        (
            "root-one-child-history.wasm",
            "3de14eb8422bb99c57f9c63870523460fb9d7b3d4c5f0ef5f7a75cc0c1333c3a",
        ),
        (
            "root-greeting-records.wasm",
            "0b51b0b4c5f980c9a40d13ac65f6d92f4d1b6b47c211727b6b59eb5ec1aeaa30",
        ),
    ] {
        let old = std::fs::read(
            repo()
                .join("qualification/canic-qualification/fixtures")
                .join(name),
        )
        .unwrap();
        let hash: String = digest(&old).iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hash, expected);
        let root = pic.create_canister().await;
        pic.install_canister(root, old.clone(), vec![], None).await;
        let input = children::input(
            root,
            children::artifact("app.wasm"),
            pic.get_time().await.as_nanos_since_unix_epoch(),
        );
        historical_stage(pic, root, &input).await.unwrap();
        pic.advance_time(Duration::from_secs(11)).await;
        settle(pic).await;
        let retained = historical_status(pic, root).await;
        assert_eq!(retained.phase, Phase::Ready);
        let stable = pic.get_stable_memory(root).await;
        assert!(
            canic_qualification::upgrade_stopped(pic, root, current.clone(), vec![], None)
                .await
                .is_err()
        );
        assert_eq!(pic.get_stable_memory(root).await, stable);
        assert_eq!(historical_status(pic, root).await, retained);
        assert_eq!(
            pic.canister_status(root, None).await.unwrap().module_hash,
            Some(digest(&old).to_vec())
        );
    }

    let fresh = pic.create_canister().await;
    pic.install_canister(fresh, children::artifact("faults.wasm"), vec![], None)
        .await;
    let stable = pic.get_stable_memory(fresh).await;
    assert!(stable.len() <= 512 * 65_536);
    // The actual endpoint encoder fails after both records and the timer changed.
    pic.update_call(
        fresh,
        CanisterId::anonymous(),
        "inject_child_fault",
        ic_wire::encode(
            &canic_management_protocol::children::Request {
                slot: canic_management_protocol::children::ChildId::FIRST,
                value: 9_u8,
            },
            ic_wire::Encoding::Frame {
                method: canic_management_protocol::children::INJECT,
                direction: ic_wire::Direction::Request,
            },
            LIMITS,
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let input = children::input(
        fresh,
        children::artifact("app.wasm"),
        pic.get_time().await.as_nanos_since_unix_epoch(),
    );
    let store = ChildStore::install(pic, fresh, children::artifact("store.wasm")).await;
    let failed = pic
        .update_call(
            fresh,
            CanisterId::anonymous(),
            "stage_child_creation",
            store
                .frame(
                    application::STAGE_CHILD_CREATION,
                    canic_management_protocol::children::ChildId::FIRST,
                    &input.approval,
                    &input.wasm,
                )
                .await,
        )
        .await
        .unwrap_err();
    assert!(
        failed
            .reject_message
            .contains("management endpoint: Encode(")
    );
    assert_eq!(pic.get_stable_memory(fresh).await, stable);
    assert_eq!(status(pic, fresh).await.phase, Phase::Empty);
    let subnet = pic.get_subnet(fresh).await.unwrap();
    let count = pic.get_subnet_metrics(subnet).await.unwrap().num_canisters;
    pic.advance_time(Duration::from_secs(11)).await;
    settle(pic).await;
    assert_eq!(
        pic.get_subnet_metrics(subnet).await.unwrap().num_canisters,
        count
    );
    canic_qualification::upgrade_stopped(pic, fresh, current.clone(), vec![], None)
        .await
        .unwrap();
    assert_eq!(pic.get_stable_memory(fresh).await, stable);
    assert_eq!(status(pic, fresh).await.phase, Phase::Empty);
    // Give each damaged image its own installed owner and install-instruction budget.
    let mut old = stable.clone();
    let offset = old.windows(4).position(|tag| tag == b"CUJG").unwrap();
    old[offset..offset + 4].copy_from_slice(b"CUJF");
    let mut corrupt = stable;
    corrupt[0] ^= 0xff;
    for (damaged, old_journal) in [(old, true), (corrupt, false), (vec![], false)] {
        let target = pic.create_canister().await;
        pic.install_canister(target, current.clone(), vec![], None)
            .await;
        let count = pic.get_subnet_metrics(subnet).await.unwrap().num_canisters;
        pic.set_stable_memory(target, damaged.clone(), BlobCompression::NoCompression)
            .await;
        let refusal = canic_qualification::upgrade_stopped(
            pic,
            target,
            children::artifact("maintenance.wasm"),
            vec![],
            None,
        )
        .await
        .unwrap_err();
        assert!(
            matches!(
                refusal.error_code,
                ErrorCode::CanisterCalledTrap | ErrorCode::CanisterTrapped
            ),
            "{refusal:?}"
        );
        if old_journal {
            assert!(
                refusal
                    .reject_message
                    .contains("unsupported Root journal format"),
                "{refusal:?}"
            );
        }
        pic.advance_time(Duration::from_secs(11)).await;
        settle(pic).await;
        assert_eq!(
            pic.get_subnet_metrics(subnet).await.unwrap().num_canisters,
            count
        );
        assert_eq!(pic.get_stable_memory(target).await, damaged);
        assert_eq!(
            pic.canister_status(target, None).await.unwrap().module_hash,
            Some(digest(&current).to_vec())
        );
    }
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}

pub(super) async fn historical_status(pic: &PocketIc, root: CanisterId) -> Status {
    let reply = pic
        .query_call(
            root,
            CanisterId::anonymous(),
            "status",
            ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method: STATUS,
                    direction: ic_wire::Direction::Request,
                },
                LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: STATUS,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
}

pub(super) async fn historical_stage(
    pic: &PocketIc,
    root: CanisterId,
    input: &Stage,
) -> Result<Status, Refusal> {
    let reply = pic
        .update_call(
            root,
            CanisterId::anonymous(),
            "stage",
            ic_wire::encode(
                input,
                ic_wire::Encoding::Frame {
                    method: STAGE,
                    direction: ic_wire::Direction::Request,
                },
                STAGE_LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: STAGE,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap()
}
