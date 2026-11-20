//! Root derives the shared directory from its own ready service installations.
use super::*;
use canic::{
    method::Contract,
    topology::directory::{Delivery, Directory, Hop, ReceiveDirectory, Service, ServiceInstall},
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
    query_service(pic, hub, "resolve_service", "hub").await
}

async fn query_service<T: serde::de::DeserializeOwned>(
    pic: &PocketIc,
    hub: CanisterId,
    method: &str,
    role: impl serde::Serialize,
) -> T {
    let frame = |direction| ic_wire::Encoding::Frame {
        method: ic_wire::method_id(method),
        direction,
    };
    let reply = pic
        .query_call(
            hub,
            CanisterId::anonymous(),
            method,
            ic_wire::encode(
                &role,
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

async fn refused_delivery(
    pic: &PocketIc,
    target: CanisterId,
    caller: CanisterId,
    role: &str,
    directory: Directory,
) -> String {
    let update = Delivery {
        directory,
        path: vec![Hop {
            canister: target.to_text().parse().unwrap(),
            role: role.into(),
            generation: [0; 32],
        }],
    };
    let reply = pic
        .update_call(
            target,
            caller,
            ReceiveDirectory::NAME,
            ReceiveDirectory::encode_request(&update).unwrap(),
        )
        .await
        .unwrap();
    ReceiveDirectory::decode_reply(&reply).unwrap().unwrap_err()
}

mod capacity;
mod peer;
mod role_proof;

enum Scenario {
    Capacity(u8),
    RoleProof,
    BothHubs,
    Peer,
    StoppedHub,
    OperatorReplay,
    StoppedChild,
    Descendant,
}

#[tokio::test]
async fn receiver_guards_named_service_and_rechecks_writes_after_await() {
    installed_publication(Scenario::Peer).await;
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
async fn replay_catches_up_a_reopened_hub_without_restarting_root() {
    installed_publication(Scenario::OperatorReplay).await;
}

#[tokio::test]
async fn stopped_child_lags_while_hub_advances_and_recovers_after_hub_upgrade() {
    installed_publication(Scenario::StoppedChild).await;
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

#[tokio::test]
async fn shard_renews_and_uses_role_until_retirement() {
    installed_publication(Scenario::RoleProof).await;
}

async fn installed_publication(scenario: Scenario) {
    let capacity = match scenario {
        Scenario::Capacity(limit) => Some(limit),
        Scenario::RoleProof => Some(4),
        _ => None,
    };
    let module = if matches!(scenario, Scenario::RoleProof) {
        "role-root.wasm"
    } else if matches!(
        scenario,
        Scenario::Capacity(_) | Scenario::StoppedChild | Scenario::StoppedHub
    ) {
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
    let pin = if matches!(scenario, Scenario::RoleProof) {
        Some(role_proof::prepare(&root).await)
    } else {
        None
    };
    let named = |name: &str| {
        ServiceInstall {
            name: name.into(),
            role: "hub".into(),
            application: if name == "accounts_hub" {
                pin.clone().unwrap_or_default()
            } else {
                vec![]
            },
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
    if let Some(limit) = capacity {
        // First hub already has an outstanding install. Second hub will stage under the ceiling.
        capacity::approve(&root, limit).await;
    }
    pic.advance_time(Duration::from_secs(11)).await;
    let user = ready(ChildId::FIRST).await;
    assert_eq!(resolve(pic, user).await, Some((1, vec![user])));
    type Single = Option<(u64, Result<CanisterId, String>)>;
    let unique =
        async |hub, role: &str| -> Single { query_service(pic, hub, "unique_service", role).await };
    assert_eq!(unique(user, "hub").await, Some((1, Ok(user))));
    assert_eq!(
        unique(user, "unknown").await,
        Some((1, Err("service role missing".into())))
    );
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
    if matches!(scenario, Scenario::BothHubs) {
        // Ordinary growth cannot rename an installed service or claim its name
        // for a different installation, even with a valid controller approval.
        let before = pic.get_stable_memory(root.id).await;
        assert_eq!(
            stage(ChildId::FIRST, named("project_hub")).await,
            Err(management::Refusal::Occupied)
        );
        assert_eq!(
            stage(ChildId::SECOND, named("accounts_hub")).await,
            Err(management::Refusal::Binding)
        );
        let retry = stage(ChildId::FIRST, named("accounts_hub")).await.unwrap();
        assert_eq!(retry.worker, Some(user.to_text()));
        assert_eq!(retry.attempts, [1; 3]);
        assert_eq!(pic.get_stable_memory(root.id).await, before);
        assert_eq!(unique(user, "hub").await, Some((1, Ok(user))));
    }
    if matches!(scenario, Scenario::Descendant) {
        let install = canic::topology::Install {
            role: "worker".into(),
            root: root.id.to_text().parse().unwrap(),
            parent: user.to_text().parse().unwrap(),
            application: vec![],
        };
        // A well-formed topology for another installing Root cannot spend this Root's slot.
        let wrong_root = canic::topology::Install {
            role: "worker".into(),
            root: install.parent,
            parent: install.root,
            application: vec![],
        };
        assert_eq!(
            stage(ChildId::SECOND, wrong_root.encode().unwrap()).await,
            Err(management::Refusal::Binding)
        );
        // Even a real canister under Root controller authority is not an
        // application parent unless Root has its ready installation record.
        let foreign_parent = canic::topology::Install {
            parent: supplier.id.to_text().parse().unwrap(),
            root: install.root,
            role: install.role.clone(),
            application: vec![],
        };
        assert_eq!(
            stage(ChildId::SECOND, foreign_parent.encode().unwrap()).await,
            Err(management::Refusal::Binding)
        );
        let untouched: management::Status = root
            .update(
                "observe_child",
                children::STATUS,
                &ChildId::SECOND,
                management::LIMITS,
            )
            .await;
        assert_eq!(untouched.phase, management::Phase::Empty);
        assert_eq!(untouched.attempts, [0; 3]);
        stage(ChildId::SECOND, install.encode().unwrap())
            .await
            .unwrap();
        let child = ready(ChildId::SECOND).await;
        assert_eq!(resolve(pic, child).await, Some((1, vec![user])));
        let status: (String, Option<Option<u64>>) =
            query_service(pic, user, "topology_status", child).await;
        assert_eq!(status, ("hub".into(), Some(Some(1))));
        canic_qualification::upgrade_stopped(
            pic,
            child,
            std::fs::read(artifacts().join("directory.wasm")).unwrap(),
            vec![],
            Some(root.id),
        )
        .await
        .unwrap();
        let status: (String, Option<Option<u64>>) =
            query_service(pic, child, "topology_status", user).await;
        assert_eq!(status, ("worker".into(), None));
        let before = pic.get_stable_memory(child).await;
        for (sender, role, expected) in [
            (outsider, "worker", "unexpected topology sender"),
            (user, "hub", "unexpected topology receiver or role"),
            (user, "worker", "conflicting receiver installation"),
        ] {
            assert_eq!(
                refused_delivery(
                    pic,
                    child,
                    sender,
                    role,
                    Directory {
                        revision: 2,
                        services: vec![]
                    }
                )
                .await,
                expected
            );
        }
        assert_eq!(pic.get_stable_memory(child).await, before);
        assert_eq!(resolve(pic, child).await, Some((1, vec![user])));
        assert_eq!(
            instance.close(Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        assert!(server.stop().unwrap().process.cleanup.is_empty());
        return;
    }
    let stopped_child = if matches!(scenario, Scenario::StoppedChild) {
        capacity::approve(&root, 3).await;
        let child = capacity::ready_project(&root, user, &source).await;
        assert_eq!(resolve(pic, child).await, Some((1, vec![user])));
        pic.stop_canister(child, Some(root.id)).await.unwrap();
        Some(child)
    } else {
        None
    };
    let miss_delivery = matches!(
        scenario,
        Scenario::StoppedHub | Scenario::OperatorReplay | Scenario::Peer
    );
    if miss_delivery {
        pic.stop_canister(user, Some(root.id)).await.unwrap();
    }
    stage(ChildId::SECOND, named("project_hub")).await.unwrap();
    let project = ready(ChildId::SECOND).await;
    if matches!(scenario, Scenario::BothHubs) {
        let sibling = Client {
            pic,
            id: project,
            operator: root.operator,
        };
        let before = pic.get_stable_memory(user).await;
        assert_eq!(
            sibling
                .update::<_, CanisterId>(
                    "attempt_delivery",
                    ic_wire::method_id("attempt_delivery"),
                    &(),
                    canic::method::LIMITS
                )
                .await,
            user
        );
        assert_eq!(pic.get_stable_memory(user).await, before);
    }

    let latest = Some((2, vec![user, project]));
    assert_eq!(resolve(pic, project).await, latest);
    assert_eq!(
        unique(project, "hub").await,
        Some((2, Err("service role ambiguous".into())))
    );
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
    if matches!(scenario, Scenario::Peer) {
        peer::exercise(&root, user, project).await;
    }
    if matches!(scenario, Scenario::StoppedHub) {
        capacity::approve(&root, 3).await;
        let child = capacity::ready_project(&root, project, &source).await;
        assert_eq!(resolve(pic, child).await, latest);
        // The child joined the project branch. Its readiness cannot replay the
        // missed publication to the independently restarted accounts hub.
        assert_eq!(resolve(pic, user).await, Some((1, vec![user])));
    }
    if let Some(child) = stopped_child {
        assert_eq!(
            publish(&root, "accounts_hub", 2).await,
            Err("directory delivery unconfirmed".into())
        );
        canic_qualification::upgrade_stopped(
            pic,
            user,
            std::fs::read(artifacts().join("directory.wasm")).unwrap(),
            vec![],
            Some(root.id),
        )
        .await
        .unwrap();
        let status: (String, Option<Option<u64>>) =
            query_service(pic, user, "topology_status", child).await;
        assert_eq!(status.1, Some(Some(1)));
        assert_eq!(resolve(pic, user).await, latest);
        pic.start_canister(child, Some(root.id)).await.unwrap();
        assert_eq!(resolve(pic, child).await, Some((1, vec![user])));
        assert_eq!(publish(&root, "accounts_hub", 2).await, Ok(2));
        assert_eq!(resolve(pic, child).await, latest);
        let status: (String, Option<Option<u64>>) =
            query_service(pic, user, "topology_status", child).await;
        assert_eq!(status.1, Some(Some(2)));
    }
    if matches!(scenario, Scenario::OperatorReplay) {
        canic_qualification::upgrade_stopped(
            pic,
            user,
            std::fs::read(artifacts().join("directory.wasm")).unwrap(),
            vec![],
            Some(root.id),
        )
        .await
        .unwrap();
        assert_eq!(unique(user, "hub").await, Some((1, Ok(user))));
        assert_eq!(publish(&root, "accounts_hub", 2).await, Ok(2));
        // Inject delayed publisher frames at the installed receiver. The normal
        // Root command above only emits the current, complete projection.
        let retained = pic.get_stable_memory(user).await;
        for revision in [1, 2] {
            let old_members = Directory {
                revision,
                services: vec![Service {
                    name: "accounts_hub".into(),
                    role: "hub".into(),
                    canister: user.to_text().parse().unwrap(),
                }],
            };
            assert_eq!(
                refused_delivery(pic, user, root.id, "hub", old_members).await,
                "stale or conflicting service directory"
            );
            assert_eq!(pic.get_stable_memory(user).await, retained);
        }
        assert_eq!(resolve(pic, user).await, latest);
        assert_eq!(
            unique(user, "hub").await,
            Some((2, Err("service role ambiguous".into())))
        );
    }
    if let Some(limit) = capacity {
        capacity::exercise(&root, &source, user, project, limit, module).await;
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
    if let Some(pin) = pin {
        role_proof::exercise(&root, user, project, pin).await;
    }
    assert_eq!(
        instance.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}
