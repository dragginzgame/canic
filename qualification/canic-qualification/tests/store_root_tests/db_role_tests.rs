//! Role-authenticated service calls between installed, database-backed siblings.
use super::*;
use canic::{
    auth::{
        enrollment,
        protocol::{EstablishSession, model, wire},
    },
    method::Contract,
    sharding::provisioning::Progress,
    topology::attestation::PrepareRoleKey,
};

#[tokio::test]
async fn placed_db_shard_recovers_its_project_after_receiver_upgrade() {
    const HUB: &str = "ex-user-directory-hub";
    let wasm = |name: &str| std::fs::read(artifacts().join(format!("{name}.wasm"))).unwrap();
    let (mut server, mut instance) = support::instance_with(
        &canic_qualification::server_directory(),
        ic_testkit::pocket_ic::PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet(),
    )
    .await;
    let gateway = instance.client_mut().make_live(None).await;
    instance.client_mut().stop_live().await;
    let pic = instance.client();
    let (root, store) = readiness_tests::fixture_with_root(pic, wasm("db-role-root")).await;
    let hub_wasm = wasm("db-role-hub");
    let hubs = store.publish_named("hubs", "1", &hub_wasm).await;
    let shards = store
        .publish_named("user_shard", "1", &wasm("db-role-shard"))
        .await;
    pic.set_controllers(store.id, Some(root.operator), vec![root.id])
        .await
        .unwrap();

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/examples/user-directory/canic.toml");
    let mut config = canic_host::load_config(&path).unwrap();
    let app = config.apps.get_mut("ex-user-directory").unwrap();
    let hub = app.canisters.get_mut(HUB).unwrap();
    hub.parent = Some("root".into());
    let mut project = hub.clone();
    project.role = Some("project_registry".into());
    app.canisters.insert("project_hub".into(), project);
    let local = app.fleets.get_mut("local").unwrap();
    // Only compile install policy here. All effects run against this local instance.
    local.network = canic_host::FleetNetwork::Bound(canic_host::Network::Local {
        url: gateway.to_string(),
        routing_canister: root.id.to_text(),
        root_subnet: Some(pic.topology().await.get_nns().unwrap().to_text()),
        root_key: pic
            .root_key()
            .await
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    });
    let receivers = local.receiver_policy.as_mut().unwrap();
    receivers.roles.push("project_registry".into());
    receivers.storage = canic::auth::install::Storage::Stable;
    local.role_receivers = vec![HUB.into(), "project_hub".into()];
    for (name, pid) in [("root", root.id), ("store", store.id)] {
        local.canisters.insert(
            name.into(),
            canic_host::CanisterBinding {
                id: pid.to_text(),
                profile_manager: false,
            },
        );
    }
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let deadline = now + 600_000_000_000;
    local.shard_pools.first_mut().unwrap().expires_at_ns = Some(deadline);
    let mut fleet = config.select(None, None).unwrap().config;
    let pin: Result<ic_wire::Bytes<32>, String> = root
        .update(
            PrepareRoleKey::NAME,
            PrepareRoleKey::ID,
            &(),
            PrepareRoleKey::REPLY,
        )
        .await;
    let pin = pin.unwrap().into_vec().try_into().unwrap();
    let args = |name, pin| {
        fleet
            .db_service_install_args(name, &root.id.to_text(), &[5; 16], pin)
            .unwrap()
    };
    let policy = canic::auth::install::Install::decode(&args(HUB, pin)).unwrap();
    let stage = async |slot, bytes| -> Result<management::Status, management::Refusal> {
        root.update(
            "stage_application_child",
            management::application::STAGE,
            &Request {
                slot,
                value: management::application::Stage {
                    source: stored::Stage {
                        approval: management::Approval {
                            manager: root.id.to_text(),
                            module_hash: hubs.hash,
                            creation_cycles: management::CREATION_CYCLES,
                            execute_at_ns: now,
                            expires_at_ns: deadline,
                        },
                        reference: stored::Reference {
                            store: store.id.to_text(),
                            manifest: hubs.clone(),
                        },
                    },
                    arguments: management::application::Arguments::new(bytes).unwrap(),
                },
            },
            stored::LIMITS,
        )
        .await
    };
    assert_eq!(
        stage(ChildId::FIRST, args(HUB, [0; 32])).await,
        Err(management::Refusal::Binding)
    );
    let mut missing = canic::auth::install::Install::decode(&args(HUB, pin)).unwrap();
    let mut service =
        canic::topology::directory::ServiceInstall::decode(&missing.application).unwrap();
    service.application = vec![5; 16];
    missing.application = service.encode().unwrap();
    assert_eq!(
        stage(ChildId::FIRST, missing.encode().unwrap()).await,
        Err(management::Refusal::Binding)
    );
    root.update::<_, Result<(), management::Refusal>>(
        "configure_capacity",
        stored::CONFIGURE_CAPACITY,
        &stored::SubnetCapacity {
            root: root.id.to_text(),
            max_children: 3,
        },
        stored::LIMITS,
    )
    .await
    .unwrap();
    for (slot, name) in [(ChildId::FIRST, HUB), (ChildId::SECOND, "project_hub")] {
        stage(slot, args(name, pin)).await.unwrap();
    }
    settle(pic).await;
    let ready = async |slot| {
        let status: management::Status = root
            .update("observe_child", children::STATUS, &slot, management::LIMITS)
            .await;
        assert_eq!(status.phase, management::Phase::Ready, "{status:?}");
        CanisterId::from_text(status.worker.unwrap()).unwrap()
    };
    let hub = ready(ChildId::FIRST).await;
    let project = ready(ChildId::SECOND).await;
    fleet.canisters.get_mut(HUB).unwrap().id = Some(hub.to_text());
    let pool = fleet
        .shard_pool_approval(
            "ex-user-directory-shard",
            shards,
            management::application::Arguments::new(vec![7; 16]).unwrap(),
        )
        .unwrap();
    root.update::<_, Result<(), management::Refusal>>(
        "configure_shards",
        stored::CONFIGURE_SHARDS,
        &pool,
        stored::LIMITS,
    )
    .await
    .unwrap();
    let grant = enrollment::Grant {
        receiver: ic_auth_protocol_types::Principal::from_slice(hub.as_slice()),
        issuer: policy.issuer,
        audience: model::AudienceId {
            canonical_network_id: policy.network,
            fleet_id: policy.audience,
        },
        key: canic_auth_witness::KEY,
        grant: model::DelegatedRoleGrant {
            target: wire::Text::new(policy.role).unwrap(),
            scopes: wire::List::new(vec![
                wire::Text::new("read".to_owned()).unwrap(),
                wire::Text::new("write".to_owned()).unwrap(),
            ])
            .unwrap(),
        },
        not_before_ns: now,
        expires_at_ns: deadline,
    };
    user_directory_tests::enroll(&root, hub, &grant, true).await;
    let (browser, credential) = crate::credential::signed(
        pic.get_time().await.as_nanos_since_unix_epoch(),
        9,
        0,
        policy.issuer,
        ic_auth_protocol_types::Principal::from_slice(&[9, 2]),
        &["read", "write"],
    );
    let login = async |id| {
        pic.advance_time(Duration::from_secs(3)).await;
        let bytes = pic
            .update_call(
                id,
                browser,
                "establish_portable_session",
                EstablishSession::encode_request(&credential).unwrap(),
            )
            .await
            .unwrap();
        EstablishSession::decode_response(&bytes).unwrap().unwrap();
    };
    login(hub).await;
    let user = Client {
        id: hub,
        operator: browser,
        pic,
    };
    let register = async || {
        user.update::<_, Result<Progress, user_directory_schema::ProvisionRefusal>>(
            "register",
            ic_wire::method_id("register"),
            &(),
            canic::method::LIMITS,
        )
        .await
        .unwrap()
    };
    assert_eq!(register().await, Progress::PlacementPending);
    settle(pic).await;
    assert_eq!(register().await, Progress::Ready);
    let shard =
        user_directory_tests::query::<Option<ic_auth_protocol_types::Principal>>(&user, "my_shard")
            .await
            .unwrap()
            .unwrap();
    let shard = CanisterId::from_slice(shard.as_slice());
    user_directory_tests::enroll(&root, shard, &grant, true).await;
    login(shard).await;
    let shard = Client { id: shard, ..user };
    let reserve = async || {
        shard
            .update::<_, Result<[u8; 16], String>>(
                "request_project",
                ic_wire::method_id("request_project"),
                &(),
                canic::method::LIMITS,
            )
            .await
            .unwrap()
    };
    // A browser with a valid Auth session still cannot enter a shard-only method.
    let account = canic::sharding::provisioning::AuthSubject {
        issuer: policy.issuer,
        subject: ic_auth_protocol_types::Principal::from_slice(&[9, 2]),
    };
    let unsigned = ic_wire::encode(
        &account,
        ic_wire::Encoding::Frame {
            method: ic_wire::method_id("reserve_project"),
            direction: ic_wire::Direction::Request,
        },
        canic::method::LIMITS,
    )
    .unwrap();
    let before = pic.get_stable_memory(project).await;
    assert!(
        pic.update_call(project, browser, "reserve_project", unsigned)
            .await
            .is_err()
    );
    assert_eq!(pic.get_stable_memory(project).await, before);
    user_directory_tests::enroll(&root, project, &grant, true).await;
    login(project).await;
    let sender = Client {
        id: project,
        ..user
    };
    let lookup = async || {
        sender
            .update::<_, Result<Option<ic_auth_protocol_types::Principal>, String>>(
                "find_my_shard",
                ic_wire::method_id("find_my_shard"),
                &(),
                canic::method::LIMITS,
            )
            .await
    };
    let expected = Ok(Some(ic_auth_protocol_types::Principal::from_slice(
        shard.id.as_slice(),
    )));
    assert_eq!(lookup().await, expected);
    let issuer_version = async || {
        pic.canister_status(root.id, Some(root.operator))
            .await
            .unwrap()
            .version
    };
    let warm = issuer_version().await;
    assert_eq!(lookup().await, expected);
    assert_eq!(issuer_version().await, warm, "a warm call must skip Root");
    let id = reserve().await;
    canic_qualification::upgrade_stopped(pic, project, hub_wasm, vec![], Some(root.id))
        .await
        .unwrap();
    assert_eq!(reserve().await, id);
    user_directory_tests::enroll(&root, project, &grant, true).await;
    assert_eq!(lookup().await, expected);

    // Reopen the sender once more to force a renewal, then log out
    // after ingress authorization and before the issuer callback can dispatch.
    canic_qualification::upgrade_stopped(pic, project, wasm("db-role-hub"), vec![], Some(root.id))
        .await
        .unwrap();
    user_directory_tests::enroll(&root, project, &grant, true).await;
    use canic::auth::protocol::Logout;
    use canic::topology::attestation::Issue;
    use ic_testkit::held::{Catalog, Fixture, Limits, Reply};
    // Hold the genuine Root reply at its original principal in this disposable fleet.
    let proof = pic
        .update_call(
            root.id,
            project,
            Issue::NAME,
            Issue::encode_request(&ic_auth_protocol_types::Principal::from_slice(
                hub.as_slice(),
            ))
            .unwrap(),
        )
        .await
        .unwrap();
    Issue::decode_reply(&proof).unwrap().unwrap();
    let catalog = Catalog::new(
        &[Reply {
            method: Issue::NAME,
            bytes: &proof,
        }],
        Limits {
            slots: 1,
            name_bytes: 32,
            reply_bytes: 1024,
        },
    )
    .unwrap();
    let mut held = Fixture::new(
        pic,
        root.id.to_text().parse().unwrap(),
        &catalog,
        100_000_000_000,
        8192,
    )
    .await
    .unwrap();
    pic.reinstall_canister(root.id, held.wasm().to_vec(), vec![], Some(root.operator))
        .await
        .unwrap();
    let before = pic
        .canister_status(hub, Some(root.id))
        .await
        .unwrap()
        .version;
    let frame = |direction| ic_wire::Encoding::Frame {
        method: ic_wire::method_id("find_my_shard"),
        direction,
    };
    let ingress = pic
        .submit_call(
            project,
            browser,
            "find_my_shard",
            ic_wire::encode(
                &(),
                frame(ic_wire::Direction::Request),
                canic::method::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let mut pending = vec![];
    for _ in 0..40 {
        pic.tick().await;
        pending = held.pending(pic).await.unwrap();
        if !pending.is_empty() {
            break;
        }
    }
    let token = pending.pop().expect("renewal reached Root");
    let reply = pic
        .update_call(
            project,
            browser,
            "logout_portable_session",
            Logout::encode_request(&()).unwrap(),
        )
        .await
        .unwrap();
    assert!(matches!(
        Logout::decode_response(&reply).unwrap(),
        model::LogoutResult::Revoked(_)
    ));
    held.release(pic, &token).await.unwrap();
    let bytes = pic.await_call(ingress).await.unwrap();
    let refusal: Result<Option<ic_auth_protocol_types::Principal>, String> = ic_wire::decode(
        &bytes,
        frame(ic_wire::Direction::Response),
        canic::method::LIMITS,
    )
    .unwrap();
    assert!(refusal.unwrap_err().starts_with("Authorization("));
    assert_eq!(
        before,
        pic.canister_status(hub, Some(root.id))
            .await
            .unwrap()
            .version
    );
    assert_eq!(
        instance.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}

#[tokio::test]
async fn schema_mount_reopens_two_stores_by_entity() {
    let wasm = std::fs::read(artifacts().join("db-role-schema.wasm")).unwrap();
    let (mut server, instance) = support::instance(&canic_qualification::server_directory()).await;
    let pic = instance.client();
    let id = pic.create_canister().await;
    pic.add_cycles(id, 3_000_000_000_000).await;
    pic.install_canister(id, wasm.clone(), vec![], None).await;
    let client = Client {
        pic,
        id,
        operator: CanisterId::anonymous(),
    };
    client
        .update::<_, ()>(
            "seed",
            ic_wire::method_id("seed"),
            &(),
            canic::method::LIMITS,
        )
        .await;
    let values = || user_directory_tests::query::<(u64, u64)>(&client, "values");
    assert_eq!(values().await.unwrap(), (7, 19));
    canic_qualification::upgrade_stopped(pic, id, wasm, vec![], None)
        .await
        .unwrap();
    assert_eq!(values().await.unwrap(), (7, 19));
    // A different roster must fail reopen before either retained store is lost.
    let before = pic.get_stable_memory(id).await;
    let other = std::fs::read(artifacts().join("db-role-hub.wasm")).unwrap();
    let refusal = canic_qualification::upgrade_stopped(pic, id, other, vec![], None)
        .await
        .unwrap_err();
    assert_eq!(
        refusal.reject_code,
        ic_testkit::pocket_ic::RejectCode::CanisterError
    );
    assert_eq!(pic.get_stable_memory(id).await, before);
    assert_eq!(values().await.unwrap(), (7, 19));
    assert_eq!(
        instance.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}
