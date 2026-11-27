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
async fn never_granted_root_preserves_standby_import_policy_after_upgrade() {
    use canic::sharding::standby::StandbyPoolError;
    let (mut server, owner) = support::instance(&canic_qualification::server_directory()).await;
    let pic = owner.client();
    let wasm = std::fs::read(artifacts().join("root.wasm")).unwrap();
    let (root, _) = readiness_tests::fixture_with_root(pic, wasm.clone()).await;
    let spare = pic.create_canister().await;
    let policy = stored::StandbyPolicy {
        root: root.id.to_text(),
        minimum_available: 0,
        maximum_creations: 0,
        expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 60_000_000_000,
        minimum_import_cycles: 1_u64.try_into().unwrap(),
        imports: vec![stored::StandbyImport {
            principal: spare.to_text(),
            previous_root: None,
        }],
    };
    root.update::<_, Result<(), management::Refusal>>(
        "configure_subnet_capacity",
        stored::CONFIGURE_SUBNET_CAPACITY,
        &stored::SubnetCapacity {
            root: root.id.to_text(),
            max_children: 8,
            allow_large_subnet: false,
        },
        stored::LIMITS,
    )
    .await
    .unwrap();
    root.update::<_, Result<(), StandbyPoolError>>(
        "configure_standby",
        stored::CONFIGURE_STANDBY,
        &policy,
        stored::LIMITS,
    )
    .await
    .unwrap();
    pic.upgrade_canister(root.id, wasm, vec![], Some(root.operator))
        .await
        .unwrap();
    let result: Result<(), StandbyPoolError> = root
        .update(
            "configure_standby",
            stored::CONFIGURE_STANDBY,
            &policy,
            stored::LIMITS,
        )
        .await;
    assert_eq!(result, Ok(()));
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}

#[tokio::test]
async fn placed_db_shard_recovers_its_project_after_receiver_upgrade() {
    installed_roles(Journey::Shards).await;
}

#[tokio::test]
async fn dedicated_receiver_bootstraps_and_accepts_its_owners_write() {
    installed_roles(Journey::DedicatedAuth).await;
}

#[tokio::test]
async fn dedicated_receiver_grant_and_allocation_reopen_before_enrollment() {
    installed_roles(Journey::DedicatedReopen).await;
}

enum Journey {
    Shards,
    DedicatedAuth,
    DedicatedReopen,
}

async fn installed_roles(journey: Journey) {
    const HUB: &str = "hub";
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
    let (root, store) = readiness_tests::fixture_with_root(pic, wasm("root")).await;
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
    let app = config.apps.get_mut("user-directory").unwrap();
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
    let receivers = local.policy.receiver_policy.as_mut().unwrap();
    receivers.roles.push("project_registry".into());
    local.policy.role_receivers = vec![HUB.into(), "project_hub".into()];
    for (name, pid) in [("root", root.id), ("store", store.id)] {
        local.canisters.insert(
            name.into(),
            canic_host::CanisterBinding { id: pid.to_text() },
        );
    }
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let deadline = now + 600_000_000_000;
    local.policy.shard_pools.first_mut().unwrap().expires_at_ns = Some(deadline);
    let mut fleet = config.select(None, None).unwrap().config;
    let pin: Result<ic_wire::Bytes<32>, canic::topology::attestation::RoleError> = root
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
            .db_service_install_args(
                name,
                &root.id.to_text(),
                &[5; 16],
                pin,
                &root.operator.to_text(),
            )
            .unwrap()
    };
    let policy = canic::auth::install::Install::decode(&args(HUB, pin)).unwrap();
    let stage = async |slot, bytes| -> Result<management::Status, management::Refusal> {
        root.update(
            "stage_child_creation",
            canic_management_protocol::application::STAGE_CHILD_CREATION,
            &Request {
                slot,
                value: canic_management_protocol::application::Deployment::<(), _>::Application(
                    management::application::Stage {
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
                ),
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
        "configure_subnet_capacity",
        stored::CONFIGURE_SUBNET_CAPACITY,
        &stored::SubnetCapacity {
            allow_large_subnet: false,
            root: root.id.to_text(),
            max_children: 4,
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
        let status: management::Status = root.observe_creation(slot).await;
        assert_eq!(status.phase, management::Phase::Ready, "{status:?}");
        CanisterId::from_text(status.worker.unwrap()).unwrap()
    };
    let hub = ready(ChildId::FIRST).await;
    let project = ready(ChildId::SECOND).await;
    pic.add_cycles(hub, 10_000_000_000_000).await;
    pic.add_cycles(project, 10_000_000_000_000).await;
    fleet.canisters.get_mut(HUB).unwrap().id = Some(hub.to_text());
    let pool = fleet
        .shard_pool_approval(
            "shard",
            shards,
            management::application::Arguments::new(vec![7; 16]).unwrap(),
        )
        .unwrap();
    root.update::<_, Result<(), management::Refusal>>(
        "approve_child_creation",
        stored::APPROVE_CHILD_CREATION,
        &stored::ChildCreationGrant::Shared(pool.clone()),
        stored::LIMITS,
    )
    .await
    .unwrap();
    super::configure_routes(&root, hub, &pool.pool, 1)
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
    if !matches!(journey, Journey::Shards) {
        dedicated_receiver(
            &root,
            project,
            &fleet,
            &grant,
            &pool,
            matches!(journey, Journey::DedicatedReopen),
        )
        .await;
        assert_eq!(
            instance.close(Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        assert!(server.stop().unwrap().process.cleanup.is_empty());
        return;
    }
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
                "canic_establish_portable_session",
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
    let shard_client = Client { id: shard, ..user };
    let routing_root = || {
        user_directory_tests::query::<Option<ic_auth_protocol_types::Principal>>(
            &shard_client,
            "routing_root",
        )
    };
    let expected_root = Some(ic_auth_protocol_types::Principal::from_slice(
        root.id.as_slice(),
    ));
    assert_ne!(root.id, hub);
    assert_eq!(routing_root().await.unwrap(), expected_root);
    canic_qualification::upgrade_stopped(pic, shard, wasm("db-role-shard"), vec![], Some(root.id))
        .await
        .unwrap();
    assert_eq!(routing_root().await.unwrap(), expected_root);
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
            "canic_logout_portable_session",
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

// The same installed DB actor also serves a dedicated allocation. Its registry
// bootstraps the owner, who then authorizes a write through the child's endpoint.
async fn dedicated_receiver(
    root: &Client<'_>,
    registry: CanisterId,
    fleet: &canic_host::FleetConfig,
    grant: &enrollment::Grant,
    shard: &stored::ShardPool,
    reopen: bool,
) {
    use canic::sharding::{
        Placement,
        provisioning::{AuthSubject, Binding, Provision, Request},
    };
    use ic_auth_protocol_types::Principal;
    let pic = root.pic;
    let mut config = fleet.clone();
    config.policy.subnet_child_limit = Some(4);
    config.policy.shard_pools.clear();
    for slot in config.canisters.values_mut() {
        slot.wasm = None;
    }
    config.canisters.get_mut("project_hub").unwrap().id = Some(registry.to_text());
    let mut slot = config.canisters["shard"].clone();
    slot.parent = Some("project_hub".into());
    slot.wasm = Some("db-role-shard.wasm".into());
    config.canisters.insert("dedicated".into(), slot);
    config.policy.registry = Some(canic_host::RegistryConfig {
        singleton: None,
        component: "dedicated".into(),
        root: "root".into(),
        store: "store".into(),
        release: shard.reference.manifest.release.clone(),
        install_args: Some("db-role-nonce.bin".into()),
        max_instances: 1,
        increase_from: None,
        creation_budget: management::CREATION_CYCLES,
        expires_at_ns: Some(grant.expires_at_ns),
    });
    let dir = artifacts();
    let selection = canic_host::SelectedFleet {
        app: "user-directory".into(),
        fleet: "local".into(),
        config,
    };
    let canic_host::FleetPlanning::PlacementPools {
        instances: Some(plan),
        ..
    } = canic_host::plan_fleet(
        &dir,
        &dir.join("canic.toml"),
        &selection,
        &root.operator.to_text(),
        None,
        None,
    )
    .unwrap()
    else {
        panic!("dedicated allocation plan")
    };
    let approved = plan.approval;
    let receiver = canic::auth::install::Install::decode(approved.arguments.as_slice()).unwrap();
    let topology = canic::topology::Install::decode(&receiver.application).unwrap();
    assert_eq!(receiver.role, "user");
    assert_eq!(topology.role, "user_shard");
    assert_eq!(topology.application, b"local-shard-v001");
    let configure = async |approval: &stored::InstancePool| {
        root.update::<_, Result<(), management::Refusal>>(
            "approve_child_creation",
            stored::APPROVE_CHILD_CREATION,
            &stored::ChildCreationGrant::Dedicated(approval.clone()),
            stored::LIMITS,
        )
        .await
    };
    let mut wrong = receiver;
    wrong.root = Principal::from_slice(registry.as_slice());
    let mut invalid = approved.clone();
    invalid.arguments = management::application::Arguments::new(wrong.encode().unwrap()).unwrap();
    assert_eq!(configure(&invalid).await, Err(management::Refusal::Binding));
    invalid = approved.clone();
    invalid.hub = Principal::from_slice(&[99, 1]).to_text();
    assert_eq!(configure(&invalid).await, Err(management::Refusal::Binding));
    configure(&approved).await.unwrap();
    // The configured credential role and application role are independent.
    // Changing the approved topology role is a conflicting grant on replay.
    let mut changed_role = topology;
    changed_role.role = "other".into();
    wrong.root = changed_role.root;
    wrong.application = changed_role.encode().unwrap();
    invalid = approved.clone();
    invalid.arguments = management::application::Arguments::new(wrong.encode().unwrap()).unwrap();
    assert_eq!(
        configure(&invalid).await,
        Err(management::Refusal::Occupied)
    );
    let assign = async || {
        acquire_fixture(
            root,
            registry,
            canic::sharding::payment::Demand::Instance([3; 32]),
        )
        .await
    };
    assert!(matches!(assign().await, Placement::Pending(_)));
    settle(pic).await;
    let placement = assign().await;
    let Placement::Assigned(_, id) = &placement else {
        panic!("{placement:?}")
    };
    assert_eq!(assign().await, placement);
    if reopen {
        canic_qualification::upgrade_stopped(
            pic,
            root.id,
            std::fs::read(dir.join("root.wasm")).unwrap(),
            vec![],
            Some(root.operator),
        )
        .await
        .unwrap();
        configure(&approved).await.unwrap();
        assert_eq!(assign().await, placement);
        return;
    }
    let child = CanisterId::from_text(id).unwrap();
    let subject = Principal::from_slice(&[12, 2]);
    type Bootstrap = Provision<user_directory_schema::Users>;
    let bootstrap = Request {
        binding: Binding {
            id: [12; 16],
            account: AuthSubject {
                issuer: grant.issuer,
                subject,
            },
            shard: Principal::from_slice(child.as_slice()),
        },
        data: (),
    };
    let reply = pic
        .update_call(
            child,
            registry,
            Bootstrap::NAME,
            Bootstrap::encode_request(&bootstrap).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        Bootstrap::decode_reply(&reply).unwrap().unwrap(),
        bootstrap.binding
    );
    user_directory_tests::enroll(root, child, grant, true).await;
    for (device, who) in [(13, Principal::from_slice(&[13, 2])), (12, subject)] {
        pic.advance_time(Duration::from_secs(3)).await;
        let (browser, proof) = crate::credential::signed(
            pic.get_time().await.as_nanos_since_unix_epoch(),
            device,
            0,
            grant.issuer,
            who,
            &["read", "write"],
        );
        let reply = pic
            .update_call(
                child,
                browser,
                "canic_establish_portable_session",
                EstablishSession::encode_request(&proof).unwrap(),
            )
            .await
            .unwrap();
        EstablishSession::decode_response(&reply).unwrap().unwrap();
        let result = Client {
            pic,
            id: child,
            operator: browser,
        }
        .update::<_, Result<[u8; 16], String>>(
            "request_project",
            ic_wire::method_id("request_project"),
            &(),
            canic::method::LIMITS,
        )
        .await;
        if who == subject {
            result.unwrap();
        } else {
            assert!(result.unwrap_err().contains("unregistered account"));
        }
    }
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
    pic.advance_time(Duration::from_secs(1)).await;
    let replaced: u64 = client
        .update(
            "replace_project",
            ic_wire::method_id("replace_project"),
            &(),
            canic::method::LIMITS,
        )
        .await;
    assert_eq!(replaced, 20);
    canic_qualification::upgrade_stopped(pic, id, wasm, vec![], None)
        .await
        .unwrap();
    assert_eq!(values().await.unwrap(), (7, 20));
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
    assert_eq!(values().await.unwrap(), (7, 20));
    assert_eq!(
        instance.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}

#[tokio::test]
async fn shard_reopens_db_and_blob_and_refuses_changes_to_its_supplier_group() {
    use canic::auth::install::{Install, LimitsPolicy};

    let (mut server, instance) = support::instance(&canic_qualification::server_directory()).await;
    let pic = instance.client();
    let root = pic.create_canister().await;
    let parent = pic.create_canister().await;
    let principal = |id: CanisterId| ic_auth_protocol_types::Principal::from_slice(id.as_slice());
    let input = Install {
        root: principal(root),
        issuer: principal(root),
        role: "shard".into(),
        network: [1; 32],
        audience: [2; 32],
        limits: LimitsPolicy {
            default_session_secs: 60,
            max_session_secs: 300,
            max_proof_ttl_ns: 60_000_000_000,
            max_sessions: Some(32),
            max_admitted_logins: Some(64),
        },
        application: canic::topology::Install {
            root: principal(root),
            parent: principal(parent),
            role: "shard".into(),
            application: vec![3; 16],
        }
        .encode()
        .unwrap(),
    }
    .encode()
    .unwrap();
    let blob = std::fs::read(artifacts().join("db-role-blob-receiver.wasm")).unwrap();
    let plain = std::fs::read(artifacts().join("db-role-receiver.wasm")).unwrap();
    for (original, changed) in [(&blob, &plain), (&plain, &blob)] {
        let id = pic.create_canister_with_settings(Some(root), None).await;
        pic.add_cycles(id, 3_000_000_000_000).await;
        pic.install_canister(id, original.clone(), input.clone(), Some(root))
            .await;
        let client = Client {
            pic,
            id,
            operator: parent,
        };
        client
            .update::<_, ()>(
                "seed",
                ic_wire::method_id("seed"),
                &(),
                canic::method::LIMITS,
            )
            .await;

        // Refuse both dropping and adding Blob before any new lease is admitted.
        let before = pic.get_stable_memory(id).await;
        let refusal =
            canic_qualification::upgrade_stopped(pic, id, changed.clone(), vec![], Some(root))
                .await
                .unwrap_err();
        assert_eq!(
            refusal.error_code,
            ic_testkit::pocket_ic::ErrorCode::CanisterTrapped
        );
        assert_eq!(pic.get_stable_memory(id).await, before);

        canic_qualification::upgrade_stopped(pic, id, original.clone(), vec![], Some(root))
            .await
            .unwrap();
        assert_eq!(
            user_directory_tests::query::<(u64, u64)>(&client, "values")
                .await
                .unwrap(),
            (7, 19)
        );
        assert_eq!(
            user_directory_tests::query::<(
                Option<ic_auth_protocol_types::Principal>,
                Option<String>
            )>(&client, "receiver_binding")
            .await
            .unwrap(),
            (Some(principal(parent)), Some("shard".into()))
        );
    }
    assert_eq!(
        instance.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}
