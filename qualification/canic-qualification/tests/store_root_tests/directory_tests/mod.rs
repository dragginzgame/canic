//! Root derives the shared directory from its own ready service installations.
use super::*;
use canic::{
    method::Contract,
    topology::directory::{Directory, ReceiveDirectory, ServiceInstall},
};

async fn publish(root: &Client<'_>, name: &str, revision: u64) -> Result<u64, String> {
    root.update(
        "publish_directory",
        ic_wire::method_id("publish_directory"),
        &(name, revision),
        canic::method::LIMITS,
    )
    .await
}

async fn resolve(pic: &PocketIc, hub: CanisterId) -> Option<(u64, Vec<CanisterId>)> {
    let frame = |direction| ic_wire::Encoding::Frame {
        method: ic_wire::method_id("resolve_service"),
        direction,
    };
    let reply = pic
        .query_call(
            hub,
            CanisterId::anonymous(),
            "resolve_service",
            ic_wire::encode(
                &"hub",
                frame(ic_wire::Direction::Request),
                canic::method::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    ic_wire::decode(
        &reply,
        frame(ic_wire::Direction::Response),
        canic::method::LIMITS,
    )
    .unwrap()
}

mod capacity;

enum Scenario {
    Capacity(u8),
    BothHubs,
    StoppedHub,
    Descendant,
}

#[tokio::test]
async fn readiness_publishes_to_existing_and_new_services() {
    installed_publication(Scenario::BothHubs).await;
}

#[tokio::test]
async fn root_reopen_catches_up_a_service_that_missed_publication() {
    installed_publication(Scenario::StoppedHub).await;
}

#[tokio::test]
async fn root_installs_descendant_with_hub_as_its_directory_publisher() {
    installed_publication(Scenario::Descendant).await;
}

#[tokio::test]
async fn two_hubs_compete_for_the_last_subnet_slot() {
    installed_publication(Scenario::Capacity(3)).await;
}

#[tokio::test]
async fn one_full_pool_leaves_capacity_for_the_other_hub() {
    installed_publication(Scenario::Capacity(4)).await;
}

#[tokio::test]
async fn scheduled_permanent_child_blocks_a_second_direct_creation() {
    installed_publication(Scenario::Capacity(1)).await;
}

async fn installed_publication(scenario: Scenario) {
    let module = if matches!(scenario, Scenario::Capacity(_)) {
        "pool-root.wasm"
    } else {
        "root.wasm"
    };
    let (mut server, instance) = support::instance(&canic_qualification::server_directory()).await;
    let pic = instance.client();
    let (root, supplier) =
        readiness_tests::fixture_with_root(pic, std::fs::read(artifacts().join(module)).unwrap())
            .await;
    let manifest = supplier
        .publish_named(
            "hub",
            "1",
            &std::fs::read(artifacts().join("directory.wasm")).unwrap(),
        )
        .await;
    pic.set_controllers(supplier.id, Some(root.operator), vec![root.id])
        .await
        .unwrap();
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let source = stored::Stage {
        approval: management::Approval {
            manager: root.id.to_text(),
            module_hash: manifest.hash,
            creation_cycles: management::CREATION_CYCLES,
            execute_at_ns: now + 10_000_000_000,
            expires_at_ns: now + 240_000_000_000,
        },
        reference: stored::Reference {
            store: supplier.id.to_text(),
            manifest,
        },
    };
    let stage = async |slot, args: Vec<u8>| -> Result<management::Status, management::Refusal> {
        root.update(
            "stage_application_child",
            management::application::STAGE,
            &Request {
                slot,
                value: management::application::Stage {
                    source: source.clone(),
                    arguments: management::application::Arguments::new(args).unwrap(),
                },
            },
            stored::LIMITS,
        )
        .await
    };
    let named = |name: &str| {
        ServiceInstall {
            name: name.into(),
            role: "hub".into(),
            application: vec![],
        }
        .encode()
        .unwrap()
    };
    let ready = async |slot| {
        settle(pic).await;
        let status: management::Status = root
            .update("observe_child", children::STATUS, &slot, management::LIMITS)
            .await;
        assert_eq!(status.phase, management::Phase::Ready, "{status:?}");
        CanisterId::from_text(status.worker.unwrap()).unwrap()
    };
    stage(ChildId::FIRST, named("accounts_hub")).await.unwrap();
    if matches!(scenario, Scenario::Capacity(1)) {
        capacity::approve(&root, 1).await;
        assert_eq!(
            stage(ChildId::SECOND, named("project_hub")).await,
            Err(management::Refusal::Occupied)
        );
        stage(ChildId::FIRST, named("accounts_hub")).await.unwrap();
        let status: management::Status = root
            .update(
                "observe_child",
                children::STATUS,
                &ChildId::SECOND,
                management::LIMITS,
            )
            .await;
        assert_eq!(status.phase, management::Phase::Empty);
        assert_eq!(
            instance.close(Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        assert!(server.stop().unwrap().process.cleanup.is_empty());
        return;
    }
    assert_eq!(
        publish(&root, "accounts_hub", 1).await,
        Err("no ready named services".into())
    );
    if let Scenario::Capacity(limit) = scenario {
        // First hub already has an outstanding install. Second hub will stage under the ceiling.
        capacity::approve(&root, limit).await;
    }
    pic.advance_time(Duration::from_secs(11)).await;
    let user = ready(ChildId::FIRST).await;
    assert_eq!(resolve(pic, user).await, Some((1, vec![user])));
    let outsider = CanisterId::self_authenticating(b"directory observer");
    let request = ic_wire::encode(
        &("accounts_hub", 1_u64),
        ic_wire::Encoding::Frame {
            method: ic_wire::method_id("publish_directory"),
            direction: ic_wire::Direction::Request,
        },
        canic::method::LIMITS,
    )
    .unwrap();
    let denial = pic
        .update_call(root.id, outsider, "publish_directory", request)
        .await
        .unwrap_err();
    assert!(
        denial.reject_message.contains("controller required"),
        "{denial:?}"
    );
    assert_eq!(resolve(pic, user).await, Some((1, vec![user])));

    assert_eq!(publish(&root, "accounts_hub", 1).await, Ok(1));
    assert_eq!(resolve(pic, user).await, Some((1, vec![user])));
    assert_eq!(
        stage(ChildId::SECOND, named("accounts_hub")).await,
        Err(management::Refusal::Binding)
    );
    if matches!(scenario, Scenario::Descendant) {
        let install = canic::topology::Install {
            root: root.id.to_text().parse().unwrap(),
            parent: user.to_text().parse().unwrap(),
            application: vec![],
        };
        stage(ChildId::SECOND, install.encode().unwrap())
            .await
            .unwrap();
        let child = ready(ChildId::SECOND).await;
        assert_eq!(resolve(pic, child).await, None);
        assert_eq!(resolve(pic, user).await, Some((1, vec![user])));
        let hub = Client {
            pic,
            id: user,
            operator: root.operator,
        };
        hub.update::<_, ()>(
            "forward_directory",
            ic_wire::method_id("forward_directory"),
            &child,
            canic::method::LIMITS,
        )
        .await;
        let before = pic.get_stable_memory(child).await;
        let denial = pic
            .update_call(
                child,
                outsider,
                ReceiveDirectory::NAME,
                ReceiveDirectory::encode_request(&Directory {
                    revision: 2,
                    services: vec![],
                })
                .unwrap(),
            )
            .await
            .unwrap_err();
        assert!(
            denial.reject_message.contains("unexpected topology sender"),
            "{denial:?}"
        );
        assert_eq!(pic.get_stable_memory(child).await, before);
        assert_eq!(resolve(pic, child).await, Some((1, vec![user])));
        assert_eq!(
            instance.close(Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        assert!(server.stop().unwrap().process.cleanup.is_empty());
        return;
    }
    let miss_delivery = matches!(scenario, Scenario::StoppedHub);
    if miss_delivery {
        pic.stop_canister(user, Some(root.id)).await.unwrap();
    }
    stage(ChildId::SECOND, named("project_hub")).await.unwrap();
    let project = ready(ChildId::SECOND).await;
    let latest = Some((2, vec![user, project]));
    assert_eq!(resolve(pic, project).await, latest);
    assert_eq!(
        publish(&root, "project_hub", 1).await,
        Err("directory revision changed".into())
    );
    if miss_delivery {
        assert_eq!(
            publish(&root, "accounts_hub", 2).await,
            Err("directory delivery unconfirmed".into())
        );
        pic.start_canister(user, Some(root.id)).await.unwrap();
        assert_eq!(resolve(pic, user).await, Some((1, vec![user])));
    } else {
        assert_eq!(resolve(pic, user).await, latest);
    }
    if let Scenario::Capacity(limit) = scenario {
        capacity::exercise(&root, &source, user, project, limit).await;
    }
    canic_qualification::upgrade_stopped(
        pic,
        root.id,
        std::fs::read(artifacts().join(module)).unwrap(),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
    settle(pic).await;
    assert_eq!(resolve(pic, user).await, latest);
    assert_eq!(resolve(pic, project).await, latest);
    let bytes = pic.get_stable_memory(user).await;
    assert_eq!(publish(&root, "accounts_hub", 2).await, Ok(2));
    assert_eq!(pic.get_stable_memory(user).await, bytes);
    assert_eq!(
        instance.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}
