//! The hub is an independent installed application. Only Root controls the shards.
use super::*;
use canic::sharding::{self, Placement};

async fn user(pic: &PocketIc, hub: CanisterId, caller: CanisterId, method: &str) -> Placement {
    let reply = pic.update_call(hub, caller, method, vec![]).await.unwrap();
    ic_wire::decode(&reply, ic_wire::Encoding::Payload, sharding::LIMITS).unwrap()
}

async fn project(
    pic: &PocketIc,
    hub: CanisterId,
    caller: CanisterId,
    method: &str,
    key: u8,
) -> Placement {
    let reply = pic
        .update_call(hub, caller, method, vec![key; 32])
        .await
        .unwrap();
    ic_wire::decode(&reply, ic_wire::Encoding::Payload, sharding::LIMITS).unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn signup_retains_the_first_shard_across_reply_loss_growth_and_hub_upgrade() {
    exercise_pool(None, None).await;
    if let Some(old) = std::env::var_os("CANIC_PREDECESSOR_SHARD_ROOT_WASM") {
        exercise_pool(None, Some(PathBuf::from(old))).await;
    }
    if let Some(wasm) = std::env::var_os("TOKO_USER_SHARD_WASM") {
        exercise_pool(Some(PathBuf::from(wasm)), None).await;
    }
}

async fn exercise_pool(toko_wasm: Option<PathBuf>, inline: Option<PathBuf>) {
    let (_, binary) = support::inputs();
    let (mut server, mut owner) = support::instance_with(
        &binary,
        ic_testkit::pocket_ic::PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet(),
    )
    .await;
    let gateway = owner.client_mut().make_live(None).await;
    // Advance replica time explicitly once the fleet has its gateway address.
    owner.client_mut().stop_live().await;
    let pic = owner.client();
    let root_wasm = std::fs::read(artifacts().join("pool-root.wasm")).unwrap();
    let initial = inline
        .as_ref()
        .map_or_else(|| root_wasm.clone(), |old| std::fs::read(old).unwrap());
    let (root, supplier) = readiness_tests::fixture_with_root(pic, initial).await;
    let hub = pic
        .create_canister_with_settings(Some(root.operator), None)
        .await;
    let hub_wasm = std::fs::read(artifacts().join("hub.wasm")).unwrap();
    pic.install_canister(
        hub,
        hub_wasm.clone(),
        root.id.as_slice().to_vec(),
        Some(root.operator),
    )
    .await;
    let shard = std::fs::read(
        toko_wasm
            .clone()
            .unwrap_or_else(|| artifacts().join("pool-db.wasm")),
    )
    .unwrap();
    let manifest = supplier
        .publish_named("user_shard", "shard-v1", &shard)
        .await;
    let project_wasm = std::fs::read(artifacts().join("pool-db.wasm")).unwrap();
    let project_release = supplier
        .publish_named("project", "project-v1", &project_wasm)
        .await;
    pic.set_controllers(supplier.id, Some(root.operator), vec![root.id])
        .await
        .unwrap();
    let fixture = if toko_wasm.is_some() { "toko" } else { "db" };
    let demo = artifacts().join(format!(
        "local-{fixture}-fleet-{}-{}",
        std::process::id(),
        pic.get_time().await.as_nanos_since_unix_epoch()
    ));
    std::fs::create_dir(&demo).unwrap();
    let config = demo.join("canic.toml");
    std::fs::write(demo.join("project.wasm"), project_wasm).unwrap();
    let mut init = Vec::new();
    if toko_wasm.is_some() {
        // Toko's fresh installation nonce precedes the captured hub principal.
        init.extend_from_slice(&[7; 16]);
    }
    init.extend_from_slice(hub.as_slice());
    std::fs::write(demo.join("shard-init.bin"), &init).unwrap();
    std::fs::write(demo.join("user_shard.wasm"), &shard).unwrap();
    std::fs::write(
        &config,
        format!(
            r#"
[apps.toko.canisters.user_hub]
[apps.toko.canisters.user_shard]
parent="user_hub"
wasm="user_shard.wasm"
[apps.toko.fleets.demo]
network={{kind="Local",url="{gateway}",root_key="{}",root_subnet="{}",routing_canister="{}"}}
[apps.toko.fleets.demo.canisters]
root={{id="{}"}}
store={{id="{}"}}
user_hub={{id="{}"}}
[[apps.toko.fleets.demo.shard_pools]]
component="user_shard"
release={{name="user_shard",version="shard-v1"}}
install_args="shard-init.bin"
root="root"
store="store"
max_shards=2
keys_per_shard=1
creation_budget={}
expires_at_ns={}
"#,
            pic.root_key()
                .await
                .unwrap()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            pic.topology().await.get_nns().unwrap(),
            root.id,
            root.id,
            supplier.id,
            hub,
            6 * management::CREATION_CYCLES,
            pic.get_time().await.as_nanos_since_unix_epoch() + 600_000_000_000
        ),
    )
    .unwrap();
    let cli = std::env::var_os("CANIC_CLI").unwrap();
    let plan = |json: bool| {
        let mut command = std::process::Command::new(&cli);
        command
            .args(["fleet", "plan", "--config"])
            .arg(&config)
            .args(["--controller", &root.operator.to_text()]);
        if json {
            command.arg("--json");
        }
        let result = command.output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        result.stdout
    };
    let text = String::from_utf8(plan(false)).unwrap();
    assert!(text.contains("2 shards × 1 live keys"));
    let result: serde_json::Value = serde_json::from_slice(&plan(true)).unwrap();
    assert_eq!(result["action"], "shard_pool");
    let bytes: Vec<u8> = serde_json::from_value(result["request"].clone()).unwrap();
    let frame = |direction| ic_wire::Encoding::Frame {
        method: stored::CONFIGURE_SHARDS,
        direction,
    };
    let approval: stored::ShardPool =
        ic_wire::decode(&bytes, frame(ic_wire::Direction::Request), stored::LIMITS).unwrap();
    assert_eq!(result["approval"], serde_json::to_value(&approval).unwrap());
    assert_eq!(approval.reference.manifest, manifest);
    assert_eq!(approval.arguments.as_slice(), &init);
    let reply = pic
        .update_call(root.id, root.operator, "configure_shards", bytes)
        .await
        .unwrap();
    let configured: Result<(), management::Refusal> =
        ic_wire::decode(&reply, frame(ic_wire::Direction::Response), stored::LIMITS).unwrap();
    configured.unwrap();
    let mut changed = approval.clone();
    changed.arguments =
        management::application::Arguments::new(root.operator.as_slice().to_vec()).unwrap();
    assert_eq!(
        configure(&root, &changed).await,
        Err(management::Refusal::Occupied)
    );
    let alice = CanisterId::self_authenticating(b"alice");
    let bob = CanisterId::self_authenticating(b"bob");
    let eve = CanisterId::self_authenticating(b"eve");
    // Even a Root controller cannot use the hub's account assignment method.
    assert!(
        pic.update_call(
            root.id,
            root.operator,
            "assign_shard",
            ic_wire::encode(
                &("user_shard", [0_u8; 32]),
                ic_wire::Encoding::Frame {
                    method: sharding::ASSIGN,
                    direction: ic_wire::Direction::Request
                },
                sharding::LIMITS
            )
            .unwrap()
        )
        .await
        .is_err()
    );
    assert_eq!(user(pic, hub, alice, "lookup").await, Placement::Unknown);
    let first = pic.submit_call(hub, alice, "signup", vec![]).await.unwrap();
    let second = pic.submit_call(hub, alice, "signup", vec![]).await.unwrap();
    let reply = pic.await_call(second).await.unwrap();
    let duplicate: Placement =
        ic_wire::decode(&reply, ic_wire::Encoding::Payload, sharding::LIMITS).unwrap();
    let allocation = match duplicate {
        Placement::Pending(a) | Placement::Assigned(a, _) => a,
        other => panic!("{other:?}"),
    };
    // Lose the first reply at the caller. The original request is never reissued to management.
    let _lost = first;
    settle(pic).await;
    let original = user(pic, hub, alice, "lookup").await;
    let Placement::Assigned(a, first_pid) = &original else {
        panic!("{original:?}")
    };
    assert_eq!(*a, allocation);
    if inline.is_some() {
        canic_qualification::upgrade_stopped(pic, root.id, root_wasm, vec![], Some(root.operator))
            .await
            .unwrap_err();
        assert_eq!(user(pic, hub, alice, "signup").await, original);
        assert_eq!(
            owner.close(Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        assert!(server.stop().unwrap().process.cleanup.is_empty());
        return;
    }
    // The same hub code, Root address, operator and account key confer no pool authority.
    let unrelated = pic
        .create_canister_with_settings(Some(root.operator), None)
        .await;
    pic.install_canister(
        unrelated,
        hub_wasm.clone(),
        root.id.as_slice().to_vec(),
        Some(root.operator),
    )
    .await;
    let retained = pic.get_stable_memory(root.id).await;
    for method in ["lookup", "signup"] {
        let refusal = pic
            .update_call(unrelated, alice, method, vec![])
            .await
            .unwrap_err();
        assert!(
            refusal
                .reject_message
                .contains("placement reply unavailable"),
            "{refusal:?}"
        );
    }
    assert!(pic.get_stable_memory(root.id).await == retained);
    assert_eq!(user(pic, hub, alice, "lookup").await, original);
    let slot = ChildId::placement(a.slot as usize).unwrap();
    let journal: management::Status = root
        .update("observe_child", children::STATUS, &slot, management::LIMITS)
        .await;
    let exact = management::application::Stage {
        source: stored::Stage {
            approval: journal.approval.clone().unwrap(),
            reference: approval.reference.clone(),
        },
        arguments: approval.arguments.clone(),
    };
    assert_eq!(
        a.generation,
        exact.pin(management::application::STAGE, slot).operation
    );
    let recovered: management::Status = root
        .update(
            "reconcile_application_child",
            management::application::RECONCILE,
            &children::AllocationRequest { slot, value: exact },
            stored::LIMITS,
        )
        .await;
    assert_eq!(recovered, journal);
    assert_eq!(
        slot,
        ChildId::SHARDS[1],
        "legacy HRW vector for alice and the user_shard pool"
    );
    let _discarded = pic.submit_call(hub, alice, "signup", vec![]).await.unwrap();
    assert!(matches!(
        user(pic, hub, bob, "signup").await,
        Placement::Pending(_)
    ));
    settle(pic).await;
    let Placement::Assigned(b, second_pid) = user(pic, hub, bob, "lookup").await else {
        panic!("second shard")
    };
    assert_ne!(b.slot, a.slot);
    assert_ne!(b.generation, a.generation);
    assert_ne!(first_pid, &second_pid);
    for pid in [first_pid, &second_pid] {
        let id = CanisterId::from_text(pid).unwrap();
        if toko_wasm.is_some() {
            let refusal = pic
                .update_call(id, root.operator, "provision_user", vec![])
                .await
                .unwrap_err();
            assert!(
                refusal.reject_message.contains("unauthorized hub"),
                "{refusal:?}"
            );
        } else {
            check_db_owner(pic, id, hub, root.operator).await;
        }
        assert_eq!(
            pic.canister_status(CanisterId::from_text(pid).unwrap(), Some(root.id))
                .await
                .unwrap()
                .settings
                .controllers,
            [root.id]
        );
    }
    assert_eq!(user(pic, hub, eve, "signup").await, Placement::Full);
    assert_eq!(user(pic, hub, eve, "lookup").await, Placement::Unknown);
    for (id, wasm) in [(hub, hub_wasm), (root.id, root_wasm.clone())] {
        canic_qualification::upgrade_stopped(pic, id, wasm, vec![], Some(root.operator))
            .await
            .unwrap();
    }
    assert_eq!(user(pic, hub, alice, "signup").await, original);
    assert_eq!(user(pic, hub, alice, "lookup").await, original);
    assert_eq!(user(pic, hub, eve, "signup").await, Placement::Full);
    let projects = instance_grant(
        root.id,
        unrelated,
        stored::Reference {
            store: supplier.id.to_text(),
            manifest: project_release,
        },
        approval.expires_at_ns,
    );
    std::fs::write(demo.join("project-init.bin"), [8; 16]).unwrap();
    let original_config = std::fs::read_to_string(&config).unwrap();
    std::fs::write(
        &config,
        format!(
            r#"{original_config}
[apps.toko.canisters.project_hub]
[apps.toko.canisters.project]
role = "project"
parent = "project_hub"
wasm = "project.wasm"
[apps.toko.fleets.demo.canisters.project_hub]
id = "{unrelated}"
[apps.toko.fleets.demo.registry]
component = "project"
root = "root"
store = "store"
release = {{ name = "project", version = "project-v1" }}
install_args = "project-init.bin"
max_instances = 2
creation_budget = {}
expires_at_ns = {}
"#,
            projects.creation_budget, projects.expires_at_ns
        ),
    )
    .unwrap();
    let text = String::from_utf8(plan(false)).unwrap();
    assert!(text.contains("Registry · project"));
    assert!(text.contains("2 shards × 1 live keys"));
    let both: serde_json::Value = serde_json::from_slice(&plan(true)).unwrap();
    assert_eq!(both["action"], "placement_pools");
    assert_eq!(both["shards"][0]["request"], result["request"]);
    assert_eq!(
        both["instances"]["approval"],
        serde_json::to_value(&projects).unwrap()
    );
    let combined_config = std::fs::read_to_string(&config).unwrap();
    let mut instance_only: toml::Table = toml::from_str(&combined_config).unwrap();
    instance_only["apps"]["toko"]["fleets"]["demo"]
        .as_table_mut()
        .unwrap()
        .remove("shard_pools")
        .unwrap();
    std::fs::write(&config, toml::to_string(&instance_only).unwrap()).unwrap();
    let single: serde_json::Value = serde_json::from_slice(&plan(true)).unwrap();
    assert_eq!(single["instances"], both["instances"]);
    assert!(single.get("shards").is_none());
    std::fs::write(&config, combined_config).unwrap();
    let source: Vec<u8> = serde_json::from_value(both["instances"]["request"].clone()).unwrap();
    assert!(
        pic.update_call(root.id, alice, "configure_instances", source.clone())
            .await
            .is_err()
    );
    assert_eq!(
        configure_project(
            &root,
            &stored::InstancePool {
                hub: approval.hub.clone(),
                ..projects.clone()
            }
        )
        .await,
        Err(management::Refusal::Binding)
    );
    assert_eq!(
        configure_project(
            &root,
            &stored::InstancePool {
                arguments: management::application::Arguments::new(vec![]).unwrap(),
                ..projects.clone()
            }
        )
        .await,
        Err(management::Refusal::Binding)
    );
    let configured = pic
        .update_call(root.id, root.operator, "configure_instances", source)
        .await
        .unwrap();
    ic_wire::decode::<Result<(), management::Refusal>>(
        &configured,
        ic_wire::Encoding::Frame {
            method: stored::CONFIGURE_INSTANCES,
            direction: ic_wire::Direction::Response,
        },
        stored::LIMITS,
    )
    .unwrap()
    .unwrap();
    for method in ["project", "lookup_project"] {
        assert!(
            pic.update_call(hub, alice, method, vec![1; 32])
                .await
                .is_err(),
            "{method} admitted the shard hub"
        );
    }
    assert_eq!(
        project(pic, unrelated, alice, "lookup_project", 1).await,
        Placement::Unknown
    );
    let lost = pic
        .submit_call(unrelated, alice, "project", vec![1; 32])
        .await
        .unwrap();
    let duplicate = project(pic, unrelated, alice, "project", 1).await;
    let (Placement::Pending(allocation) | Placement::Assigned(allocation, _)) = duplicate else {
        panic!("{duplicate:?}")
    };
    let _lost = lost;
    let second = project(pic, unrelated, alice, "project", 2).await;
    assert!(matches!(
        second,
        Placement::Pending(_) | Placement::Assigned(..)
    ));
    settle(pic).await;
    let project_a = project(pic, unrelated, alice, "lookup_project", 1).await;
    let project_b = project(pic, unrelated, alice, "lookup_project", 2).await;
    let Placement::Assigned(a, pid_a) = &project_a else {
        panic!("{project_a:?}")
    };
    let Placement::Assigned(b, pid_b) = &project_b else {
        panic!("{project_b:?}")
    };
    assert_eq!(a, &allocation);
    assert_ne!(a, b);
    assert_ne!(pid_a, pid_b);
    for id in [pid_a, pid_b] {
        check_db_owner(pic, id.parse().unwrap(), unrelated, root.operator).await;
    }
    assert_eq!(
        project(pic, unrelated, alice, "project", 3).await,
        Placement::Full
    );
    if let Some(path) = std::env::var_os("CANIC_PREDECESSOR_SHARD_ROOT_WASM") {
        canic_qualification::upgrade_stopped(
            pic,
            root.id,
            std::fs::read(path).unwrap(),
            vec![],
            Some(root.operator),
        )
        .await
        .unwrap_err();
    }
    canic_qualification::upgrade_stopped(
        pic,
        root.id,
        root_wasm.clone(),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
    configure_project(&root, &projects).await.unwrap();
    assert_eq!(
        project(pic, unrelated, alice, "project", 1).await,
        project_a
    );
    assert_eq!(
        project(pic, unrelated, alice, "lookup_project", 2).await,
        project_b
    );
    assert_eq!(
        configure_project(
            &root,
            &stored::InstancePool {
                creation_budget: 0,
                ..projects.clone()
            }
        )
        .await,
        Err(management::Refusal::Occupied)
    );
    configure(&root, &approval).await.unwrap();
    assert_eq!(user(pic, hub, alice, "lookup").await, original);
    assert!(
        pic.update_call(unrelated, alice, "signup", vec![])
            .await
            .is_err()
    );
    // The two old grants have spare funding but cannot grow until the controller approves a count change.
    let text = std::fs::read_to_string(&config)
        .unwrap()
        .replace(
            "[apps.toko.fleets.demo]",
            "[apps.toko.fleets.demo]\nsubnet_child_limit = 8",
        )
        .replace("max_shards=2", "max_shards=6\nincrease_from=2")
        .replace("max_instances = 2", "max_instances = 6\nincrease_from=2");
    std::fs::write(&config, text).unwrap();
    let expanded: serde_json::Value = serde_json::from_slice(&plan(true)).unwrap();
    let cap: stored::SubnetCapacity =
        serde_json::from_value(expanded["capacity"]["approval"].clone()).unwrap();
    assert_eq!(cap.max_children, 8);
    let changes = expanded["increases"].as_array().unwrap();
    assert_eq!(changes.len(), 2);
    for (item, endpoint, method) in std::iter::once((
        &expanded["capacity"],
        "configure_capacity",
        stored::CONFIGURE_CAPACITY,
    ))
    .chain(
        changes
            .iter()
            .map(|item| (item, "increase_pool", stored::INCREASE_POOL)),
    ) {
        let req: Vec<u8> = serde_json::from_value(item["request"].clone()).unwrap();
        assert!(
            pic.update_call(root.id, alice, endpoint, req.clone())
                .await
                .is_err()
        );
        for _ in 0..2 {
            let bytes = pic
                .update_call(root.id, root.operator, endpoint, req.clone())
                .await
                .unwrap();
            ic_wire::decode::<Result<(), management::Refusal>>(
                &bytes,
                ic_wire::Encoding::Frame {
                    method,
                    direction: ic_wire::Direction::Response,
                },
                stored::LIMITS,
            )
            .unwrap()
            .unwrap();
        }
    }
    for forged in [
        stored::InstancePool {
            creation_budget: projects.creation_budget + 1,
            ..projects.clone()
        },
        stored::InstancePool {
            expires_at_ns: projects.expires_at_ns + 1,
            ..projects.clone()
        },
    ] {
        let result: Result<(), management::Refusal> = root
            .update(
                "increase_pool",
                stored::INCREASE_POOL,
                &stored::IncreasePool::Instances(forged, 6),
                stored::LIMITS,
            )
            .await;
        assert_eq!(result, Err(management::Refusal::Binding));
    }
    let stale: Result<(), management::Refusal> = root
        .update(
            "increase_pool",
            stored::INCREASE_POOL,
            &stored::IncreasePool::Instances(projects.clone(), 7),
            stored::LIMITS,
        )
        .await;
    assert_eq!(stale, Err(management::Refusal::Occupied));
    pic.add_cycles(root.id, 12_000_000_000_000).await;
    assert!(matches!(
        user(pic, hub, eve, "signup").await,
        Placement::Pending(_)
    ));
    assert!(matches!(
        project(pic, unrelated, alice, "project", 3).await,
        Placement::Pending(_)
    ));
    settle(pic).await;
    let third_shard = user(pic, hub, eve, "lookup").await;
    let third_project = project(pic, unrelated, alice, "lookup_project", 3).await;
    assert!(matches!(third_shard, Placement::Assigned(..)));
    assert!(matches!(third_project, Placement::Assigned(..)));
    assert!(matches!(
        project(pic, unrelated, alice, "project", 4).await,
        Placement::Pending(_)
    ));
    settle(pic).await;

    // Two hubs contend for the eighth slot. Drop both responses and resolve by their original keys.
    let fourth_user = CanisterId::self_authenticating(b"fourth");
    let _shard_call = pic
        .submit_call(hub, fourth_user, "signup", vec![])
        .await
        .unwrap();
    let _project_call = pic
        .submit_call(unrelated, alice, "project", vec![5; 32])
        .await
        .unwrap();
    settle(pic).await;
    canic_qualification::upgrade_stopped(pic, root.id, root_wasm, vec![], Some(root.operator))
        .await
        .unwrap();
    let shard = user(pic, hub, fourth_user, "signup").await;
    let project_retry = project(pic, unrelated, alice, "project", 5).await;
    assert!(
        matches!(
            (&shard, &project_retry),
            (Placement::Assigned(..), Placement::Full) | (Placement::Full, Placement::Assigned(..))
        ),
        "{shard:?} / {project_retry:?}"
    );
    assert_eq!(user(pic, hub, eve, "lookup").await, third_shard);
    assert_eq!(
        project(pic, unrelated, alice, "lookup_project", 3).await,
        third_project
    );
    assert_eq!(user(pic, hub, alice, "lookup").await, original);
    assert_eq!(
        project(pic, unrelated, alice, "lookup_project", 1).await,
        project_a
    );
    configure(&root, &approval).await.unwrap();
    configure_project(&root, &projects).await.unwrap();
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}

async fn check_db_owner(pic: &PocketIc, id: CanisterId, hub: CanisterId, operator: CanisterId) {
    // The installed DB witness only exposes its admission for the owner passed at init.
    use canic::db::{RPC_LIMITS, inspection};
    let frame = |direction| ic_wire::Encoding::Frame {
        method: inspection::INSPECT,
        direction,
    };
    let request = ic_wire::encode(&(), frame(ic_wire::Direction::Request), RPC_LIMITS).unwrap();
    let bytes = pic
        .query_call(id, hub, "db_inspect", request.clone())
        .await
        .unwrap();
    let reply: inspection::Response =
        ic_wire::decode(&bytes, frame(ic_wire::Direction::Response), RPC_LIMITS).unwrap();
    assert!(matches!(
        reply.admission,
        inspection::Admission::Readable { .. }
    ));
    assert!(
        pic.query_call(id, operator, "db_inspect", request)
            .await
            .is_err()
    );
}

#[tokio::test(flavor = "current_thread")]
async fn exhausted_creation_authority_never_claims_a_key_or_creates_a_shard() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let (root, store) = readiness_tests::fixture_with_root(
        pic,
        std::fs::read(artifacts().join("pool-root.wasm")).unwrap(),
    )
    .await;
    let hub = pic.create_canister().await;
    pic.install_canister(
        hub,
        std::fs::read(artifacts().join("hub.wasm")).unwrap(),
        root.id.as_slice().to_vec(),
        None,
    )
    .await;
    let manifest = store.publish_bytes("one", b"\0asm\x01\0\0\0").await;
    let approval = stored::ShardPool {
        arguments: management::application::Arguments::new(vec![]).unwrap(),
        root: root.id.to_text(),
        hub: hub.to_text(),
        pool: "user_shard".into(),
        max_shards: 2,
        keys_per_shard: 100_000,
        creation_payer: Default::default(),
        creation_budget: 0,
        expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 600_000_000_000,
        reference: stored::Reference {
            store: store.id.to_text(),
            manifest,
        },
    };
    let caller = CanisterId::self_authenticating(b"user");
    let encoded = ic_wire::encode(
        &approval,
        ic_wire::Encoding::Frame {
            method: stored::CONFIGURE_SHARDS,
            direction: ic_wire::Direction::Request,
        },
        stored::LIMITS,
    )
    .unwrap();
    assert!(
        pic.update_call(root.id, caller, "configure_shards", encoded)
            .await
            .is_err()
    );
    let project_hub = pic.create_canister().await;
    pic.install_canister(
        project_hub,
        std::fs::read(artifacts().join("hub.wasm")).unwrap(),
        root.id.as_slice().to_vec(),
        None,
    )
    .await;
    let mut projects = instance_grant(
        root.id,
        project_hub,
        approval.reference.clone(),
        approval.expires_at_ns,
    );
    projects.creation_budget = 0;
    configure_project(&root, &projects).await.unwrap();
    assert_eq!(
        project(pic, project_hub, caller, "project", 1).await,
        Placement::GrowthRefused
    );
    assert_eq!(
        project(pic, project_hub, caller, "lookup_project", 1).await,
        Placement::Unknown
    );
    assert_eq!(
        configure(
            &root,
            &stored::ShardPool {
                hub: project_hub.to_text(),
                ..approval.clone()
            }
        )
        .await,
        Err(management::Refusal::Binding)
    );
    // A shared pool cannot acquire future project capacity via a larger grant.
    assert_eq!(
        configure(
            &root,
            &stored::ShardPool {
                max_shards: 3,
                ..approval.clone()
            }
        )
        .await,
        Err(management::Refusal::Binding)
    );
    configure(&root, &approval).await.unwrap();
    assert_eq!(
        user(pic, hub, caller, "signup").await,
        Placement::GrowthRefused
    );
    assert_eq!(user(pic, hub, caller, "lookup").await, Placement::Unknown);
    assert_eq!(
        configure(
            &root,
            &stored::ShardPool {
                creation_budget: 2 * management::CREATION_CYCLES,
                ..approval
            }
        )
        .await,
        Err(management::Refusal::Occupied)
    );
    for slot in ChildId::ALL {
        let journal: management::Status = root
            .update("observe_child", children::STATUS, &slot, management::LIMITS)
            .await;
        assert_eq!(journal.phase, management::Phase::Empty);
        assert_eq!(journal.attempts, [0; 3]);
    }
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}

async fn configure(root: &Client<'_>, a: &stored::ShardPool) -> Result<(), management::Refusal> {
    root.update(
        "configure_shards",
        stored::CONFIGURE_SHARDS,
        a,
        stored::LIMITS,
    )
    .await
}

fn instance_grant(
    root: CanisterId,
    hub: CanisterId,
    reference: stored::Reference,
    expires_at_ns: u64,
) -> stored::InstancePool {
    stored::InstancePool {
        root: root.to_text(),
        hub: hub.to_text(),
        component: "project".into(),
        reference,
        arguments: management::application::Arguments::new(
            canic::topology::Install {
                role: "project".into(),
                root: root.to_text().parse().unwrap(),
                parent: hub.to_text().parse().unwrap(),
                application: vec![8; 16],
            }
            .encode()
            .unwrap(),
        )
        .unwrap(),
        max_instances: 2,
        creation_payer: Default::default(),
        creation_budget: 6 * management::CREATION_CYCLES,
        expires_at_ns,
    }
}

async fn configure_project(
    root: &Client<'_>,
    grant: &stored::InstancePool,
) -> Result<(), management::Refusal> {
    root.update(
        "configure_instances",
        stored::CONFIGURE_INSTANCES,
        grant,
        stored::LIMITS,
    )
    .await
}

#[tokio::test(flavor = "current_thread")]
async fn lost_create_result_keeps_the_original_claim_and_blocks_another_creation() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let wasm = std::fs::read(artifacts().join("pool-faults.wasm")).unwrap();
    let (root, supplier) = readiness_tests::fixture_with_root(pic, wasm.clone()).await;
    let hub = pic.create_canister().await;
    pic.install_canister(
        hub,
        std::fs::read(artifacts().join("hub.wasm")).unwrap(),
        root.id.as_slice().to_vec(),
        None,
    )
    .await;
    let manifest = supplier
        .publish_bytes(
            "shards",
            &std::fs::read(artifacts().join("app.wasm")).unwrap(),
        )
        .await;
    pic.set_controllers(supplier.id, Some(root.operator), vec![root.id])
        .await
        .unwrap();
    for slot in ChildId::SHARDS {
        root.update::<_, ()>(
            "inject_child_fault",
            children::INJECT,
            &children::AllocationRequest { slot, value: 2_u8 },
            management::LIMITS,
        )
        .await;
    }
    let ceiling = stored::SubnetCapacity {
        root: root.id.to_text(),
        max_children: 2,
    };
    root.update::<_, Result<(), management::Refusal>>(
        "configure_capacity",
        stored::CONFIGURE_CAPACITY,
        &ceiling,
        stored::LIMITS,
    )
    .await
    .unwrap();
    let grant = stored::ShardPool {
        arguments: management::application::Arguments::new(vec![]).unwrap(),
        root: root.id.to_text(),
        hub: hub.to_text(),
        pool: "user_shard".into(),
        max_shards: 2,
        keys_per_shard: 1,
        creation_payer: Default::default(),
        creation_budget: 2 * management::CREATION_CYCLES,
        expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 600_000_000_000,
        reference: stored::Reference {
            store: supplier.id.to_text(),
            manifest,
        },
    };
    configure(&root, &grant).await.unwrap();
    let alice = CanisterId::self_authenticating(b"alice");
    let bob = CanisterId::self_authenticating(b"bob");
    let Placement::Pending(original) = user(pic, hub, alice, "signup").await else {
        panic!("expected reservation")
    };
    settle(pic).await;
    canic_qualification::upgrade_stopped(pic, root.id, wasm, vec![], Some(root.operator))
        .await
        .unwrap();
    assert_eq!(
        user(pic, hub, alice, "signup").await,
        Placement::Pending(original.clone())
    );
    assert_eq!(user(pic, hub, bob, "signup").await, Placement::Busy);
    {
        let slot = ChildId::placement(original.slot as usize).unwrap();
        let journal: management::Status = root
            .update("observe_child", children::STATUS, &slot, management::LIMITS)
            .await;

        assert_eq!(journal.phase, management::Phase::CreateIssued);
        assert_eq!(journal.attempts, [1, 0, 0]);
        assert!(journal.worker.is_none());
        let input = stored::Stage {
            approval: journal.approval.clone().unwrap(),
            reference: grant.reference.clone(),
        };
        let observed: management::Status = root
            .update(
                "reconcile_stored_child",
                stored::RECONCILE,
                &children::AllocationRequest { slot, value: input },
                stored::LIMITS,
            )
            .await;
        assert_eq!(observed, journal);
    }
    assert_eq!(user(pic, hub, bob, "lookup").await, Placement::Unknown);
    // The unknown shard creation still consumes one of this subnet's two slots.
    let peer = pic.create_canister().await;
    pic.install_canister(
        peer,
        std::fs::read(artifacts().join("hub.wasm")).unwrap(),
        root.id.as_slice().to_vec(),
        None,
    )
    .await;
    configure_project(
        &root,
        &instance_grant(root.id, peer, grant.reference.clone(), grant.expires_at_ns),
    )
    .await
    .unwrap();
    assert!(matches!(
        project(pic, peer, bob, "project", 1).await,
        Placement::Pending(_)
    ));
    assert_eq!(project(pic, peer, bob, "project", 2).await, Placement::Full);
    assert_eq!(
        project(pic, peer, bob, "lookup_project", 2).await,
        Placement::Unknown
    );
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}

async fn fresh_user_shard_grant(
    root: &Client<'_>,
    store: &Client<'_>,
) -> (CanisterId, stored::ShardPool) {
    grant_with_child(root, store, "pool-db.wasm").await
}

async fn grant_with_child(
    root: &Client<'_>,
    store: &Client<'_>,
    child_wasm: &str,
) -> (CanisterId, stored::ShardPool) {
    let pic = root.pic;
    let hub = pic.create_canister().await;
    pic.add_cycles(hub, 50_000_000_000_000).await;
    pic.add_cycles(root.id, 50_000_000_000_000).await;
    pic.install_canister(
        hub,
        std::fs::read(artifacts().join("hub.wasm")).unwrap(),
        root.id.as_slice().to_vec(),
        None,
    )
    .await;
    let child = std::fs::read(artifacts().join(child_wasm)).unwrap();
    let manifest = store.publish_named("user_shard", "capacity", &child).await;
    pic.set_controllers(store.id, Some(root.operator), vec![root.id])
        .await
        .unwrap();
    let approval = stored::ShardPool {
        arguments: management::application::Arguments::new(hub.as_slice().to_vec()).unwrap(),
        root: root.id.to_text(),
        hub: hub.to_text(),
        pool: "user_shard".into(),
        max_shards: 2,
        keys_per_shard: 10_000,
        creation_payer: Default::default(),
        creation_budget: 2 * management::CREATION_CYCLES,
        expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 600_000_000_000,
        reference: stored::Reference {
            store: store.id.to_text(),
            manifest,
        },
    };
    (hub, approval)
}

// Call Root directly as the approved hub, before any application provisioning.
#[tokio::test(flavor = "current_thread")]
async fn approved_hub_retries_first_key_in_fresh_named_pool() {
    use canic::{
        method::Contract,
        sharding::{AssignShard, LookupShard},
    };

    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let wasm = std::fs::read(artifacts().join("pool-root.wasm")).unwrap();
    let (root, store) = readiness_tests::fixture_with_root(pic, wasm).await;
    let (hub, mut grant) = fresh_user_shard_grant(&root, &store).await;
    grant.max_shards = 1;
    configure(&root, &grant).await.unwrap();

    let input = ("user_shard".to_owned(), [0xa5; 32]);
    let request = AssignShard::encode_request(&input).unwrap();
    let assign = async || {
        let reply = pic
            .update_call(root.id, hub, AssignShard::NAME, request.clone())
            .await
            .expect("approved hub assignment");
        AssignShard::decode_reply(&reply).unwrap()
    };
    let lookup = async || {
        let reply = pic
            .update_call(
                root.id,
                hub,
                LookupShard::NAME,
                LookupShard::encode_request(&input).unwrap(),
            )
            .await
            .unwrap();
        LookupShard::decode_reply(&reply).unwrap()
    };
    assert_eq!(lookup().await, Placement::Unknown);
    let Placement::Pending(original) = assign().await else {
        panic!("first key must reserve a shard")
    };
    settle(pic).await;
    let assigned = assign().await;
    let (allocation, _) = assigned.assigned_target().unwrap().expect("ready shard");
    assert_eq!(allocation, &original);
    assert_eq!(lookup().await, assigned);
    assert_eq!(assign().await, assigned);
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn ten_thousand_keys_per_shard_survive_rollover_and_root_reopen() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance_with_lifetime(
        &binary,
        ic_testkit::pocket_ic::PocketIcBuilder::new().with_application_subnet(),
        Duration::from_secs(900),
    )
    .await;
    let pic = owner.client();
    let wasm = std::fs::read(artifacts().join("pool-root.wasm")).unwrap();
    let (root, store) = readiness_tests::fixture_with_root(pic, wasm.clone()).await;
    let (hub, mut approval) = fresh_user_shard_grant(&root, &store).await;
    approval.keys_per_shard = 100_001;
    assert_eq!(
        configure(&root, &approval).await,
        Err(management::Refusal::Binding)
    );
    approval.keys_per_shard = 10_000;
    configure(&root, &approval).await.unwrap();
    let account = |n: u32| CanisterId::self_authenticating(n.to_le_bytes());
    let mut assignments = Vec::new();
    for first in [1, 10_001] {
        assert!(matches!(
            user(pic, hub, account(first), "signup").await,
            Placement::Pending(_)
        ));
        settle(pic).await;
        let assigned = user(pic, hub, account(first), "lookup").await;
        assert!(matches!(assigned, Placement::Assigned(..)));
        for start in (first + 1..first + 10_000).step_by(100) {
            let mut requests = Vec::new();
            for key in start..(start + 100).min(first + 10_000) {
                requests.push(
                    pic.submit_call(hub, account(key), "signup", vec![])
                        .await
                        .unwrap(),
                );
            }
            for request in requests {
                let bytes = pic.await_call(request).await.unwrap();
                let result: Placement =
                    ic_wire::decode(&bytes, ic_wire::Encoding::Payload, sharding::LIMITS).unwrap();
                assert_eq!(result, assigned);
            }
        }
        assignments.push(assigned);
    }
    assert_ne!(assignments[0], assignments[1]);
    assert_eq!(
        user(pic, hub, account(20_001), "signup").await,
        Placement::Full
    );
    canic_qualification::upgrade_stopped(pic, root.id, wasm, vec![], Some(root.operator))
        .await
        .unwrap();
    for (key, expected) in [(1, 0), (10_000, 0), (10_001, 1), (20_000, 1)] {
        for method in ["lookup", "signup"] {
            assert_eq!(
                user(pic, hub, account(key), method).await,
                assignments[expected]
            );
        }
    }
    assert_eq!(
        user(pic, hub, account(20_001), "signup").await,
        Placement::Full
    );
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn registry_reimburses_root_once_on_a_reserve_hit_and_a_miss() {
    use canic::{
        method::Contract,
        sharding::{
            payment::{Demand, Payment},
            standby::{Asset, Refill, RefillStandby},
        },
    };
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let wasm = std::fs::read(artifacts().join("pool-root.wasm")).unwrap();
    let (root, store) = readiness_tests::fixture_with_root(pic, wasm.clone()).await;
    let (hub, shared) = fresh_user_shard_grant(&root, &store).await;
    let mut grant = instance_grant(root.id, hub, shared.reference, shared.expires_at_ns);
    grant.max_instances = 3;
    grant.creation_payer = stored::CreationPayer::Requester;
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
    configure_project(&root, &grant).await.unwrap();
    let maximum = 2 * u128::from(management::CREATION_CYCLES);
    let method = ic_wire::method_id("pay");
    let frame = |direction| ic_wire::Encoding::Frame { method, direction };
    let request = |id: u8, limit: u128, attached: u128| {
        ic_wire::encode(
            &(Demand::Instance([id; 32]), limit, attached),
            frame(ic_wire::Direction::Request),
            canic::sharding::LIMITS,
        )
        .unwrap()
    };
    let pay = async |id: u8, attached: u128| {
        let reply = pic
            .update_call(
                hub,
                CanisterId::anonymous(),
                "pay",
                request(id, maximum, attached),
            )
            .await
            .unwrap();
        ic_wire::decode::<Result<Payment, String>>(
            &reply,
            frame(ic_wire::Direction::Response),
            canic::sharding::LIMITS,
        )
        .unwrap()
    };
    let denied = root
        .update::<_, Result<Payment, String>>(
            canic::sharding::payment::FundPlacement::NAME,
            canic::sharding::payment::FundPlacement::ID,
            &(Demand::Instance([1; 32]), maximum),
            canic::sharding::LIMITS,
        )
        .await;
    assert!(denied.unwrap_err().contains("calling hub"));
    for id in [1, 2] {
        let spare = if id == 1 {
            let asset = root
                .update::<_, Result<Asset, String>>(
                    RefillStandby::NAME,
                    RefillStandby::ID,
                    &Refill {
                        root: root.id.to_text().parse().unwrap(),
                        sequence: 0,
                        cycles: management::CREATION_CYCLES,
                        expires_at_ns: grant.expires_at_ns,
                    },
                    RefillStandby::REPLY,
                )
                .await
                .unwrap();
            let Asset::Empty(spare) = asset else {
                panic!("ready spare")
            };
            Some(spare)
        } else {
            None
        };
        let Payment::Quoted(quote) = pay(id, 0).await.unwrap() else {
            panic!("quote required")
        };
        if id == 1 {
            // The held spare and its exact balance quote survive an ordinary Root upgrade.
            canic_qualification::upgrade_stopped(
                pic,
                root.id,
                wasm.clone(),
                vec![],
                Some(root.operator),
            )
            .await
            .unwrap();
            assert_eq!(pay(id, 0).await.unwrap(), Payment::Quoted(quote.clone()));
            assert!(pay(2, 0).await.is_err());
        }
        assert_eq!(quote.spare, spare);
        let charge = quote.creation_fee + quote.working_cycles;
        assert!(quote.creation_fee > 0 && quote.working_cycles > 0 && charge <= maximum);
        assert!(pay(id, charge - 1).await.is_err());
        let before = pic.cycle_balance(hub).await;
        // Discard the first settlement reply. Recover using only the original demand.
        let _discarded = pic
            .submit_call(
                hub,
                CanisterId::anonymous(),
                "pay",
                request(id, maximum, maximum),
            )
            .await
            .unwrap();
        settle(pic).await;
        let Payment::Placed(Placement::Assigned(original, original_target), Some(receipt)) =
            pay(id, 0).await.unwrap()
        else {
            panic!("recover paid admission after lost reply")
        };
        assert_eq!(quote, receipt);
        let child_balance = pic.cycle_balance(original_target.parse().unwrap()).await;
        assert!(child_balance > charge.saturating_sub(quote.creation_fee) / 2);
        assert!(before - pic.cycle_balance(hub).await >= charge);
        settle(pic).await;
        canic_qualification::upgrade_stopped(
            pic,
            root.id,
            wasm.clone(),
            vec![],
            Some(root.operator),
        )
        .await
        .unwrap();
        let before = pic.cycle_balance(hub).await;
        let Payment::Placed(Placement::Assigned(recovered, target), Some(replayed)) =
            pay(id, maximum).await.unwrap()
        else {
            panic!("recover original paid child")
        };
        assert_eq!(original, recovered);
        assert_eq!(original_target, target);
        assert_eq!(quote, replayed);
        if let Some(spare) = spare {
            assert_eq!(target, spare.to_text());
        }
        assert!(before - pic.cycle_balance(hub).await < charge / 100);
    }
    // An unpaid reservation excludes other claims only until its original deadline.
    root.update::<_, Result<(), String>>(
        "configure_standby",
        stored::CONFIGURE_STANDBY,
        &stored::StandbyPolicy {
            root: root.id.to_text(),
            minimum_available: 0,
            maximum_creations: 0,
            expires_at_ns: grant.expires_at_ns,
            minimum_import_cycles: 1_u64.try_into().unwrap(),
            imports: vec![],
        },
        stored::LIMITS,
    )
    .await
    .unwrap();
    let counts = async || {
        let reply = pic
            .query_call(
                root.id,
                root.operator,
                stored::ReadSpareCounts::NAME,
                stored::ReadSpareCounts::encode_request(&()).unwrap(),
            )
            .await
            .unwrap();
        stored::ReadSpareCounts::decode_reply(&reply)
            .unwrap()
            .unwrap()
            .ready
    };
    root.update::<_, Result<Asset, String>>(
        RefillStandby::NAME,
        RefillStandby::ID,
        &Refill {
            root: grant.root.parse().unwrap(),
            sequence: 1,
            cycles: management::CREATION_CYCLES,
            expires_at_ns: grant.expires_at_ns,
        },
        RefillStandby::REPLY,
    )
    .await
    .unwrap();
    assert_eq!(counts().await, 1);
    let low = pic
        .update_call(hub, CanisterId::anonymous(), "pay", request(3, 1, 0))
        .await
        .unwrap();
    assert!(
        ic_wire::decode::<Result<Payment, String>>(
            &low,
            frame(ic_wire::Direction::Response),
            canic::sharding::LIMITS
        )
        .unwrap()
        .is_err()
    );
    assert_eq!(counts().await, 1);
    assert!(matches!(pay(3, 0).await.unwrap(), Payment::Quoted(_)));
    assert_eq!(counts().await, 0);
    pic.advance_time(Duration::from_secs(601)).await;
    pic.tick().await;
    assert_eq!(counts().await, 1);
    assert!(pay(3, maximum).await.is_err());
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn hub_commits_routes_locally_and_requests_capacity_once_per_ordinal() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let wasm = std::fs::read(artifacts().join("pool-root.wasm")).unwrap();
    let (root, store) = readiness_tests::fixture_with_root(pic, wasm.clone()).await;
    let (hub, mut grant) = grant_with_child(&root, &store, "hub.wasm").await;
    use canic::{
        method::Contract,
        sharding::standby::{Asset, Refill, RefillStandby},
    };
    root.update::<_, Result<(), management::Refusal>>(
        "configure_capacity",
        stored::CONFIGURE_CAPACITY,
        &stored::SubnetCapacity {
            root: root.id.to_text(),
            max_children: 2,
        },
        stored::LIMITS,
    )
    .await
    .unwrap();
    let Asset::Empty(spare) = root
        .update::<_, Result<Asset, String>>(
            RefillStandby::NAME,
            RefillStandby::ID,
            &Refill {
                root: root.id.to_text().parse().unwrap(),
                sequence: 0,
                cycles: management::CREATION_CYCLES,
                expires_at_ns: grant.expires_at_ns,
            },
            RefillStandby::REPLY,
        )
        .await
        .unwrap()
    else {
        panic!("expected available spare")
    };
    grant.keys_per_shard = 2;
    configure(&root, &grant).await.unwrap();
    let grow = async |caller, ordinal| {
        pic.update_call(
            root.id,
            caller,
            sharding::GrowShard::NAME,
            sharding::GrowShard::encode_request(&("user_shard".into(), ordinal)).unwrap(),
        )
        .await
    };
    assert!(grow(root.operator, 0).await.is_err());
    let skipped = grow(hub, 1).await.unwrap();
    assert_eq!(
        sharding::GrowShard::decode_reply(&skipped).unwrap(),
        (Placement::GrowthRefused, 2)
    );
    let request = |name: &str| {
        ic_wire::encode(
            &(),
            ic_wire::Encoding::Frame {
                method: ic_wire::method_id(name),
                direction: ic_wire::Direction::Request,
            },
            sharding::LIMITS,
        )
        .unwrap()
    };
    let response = |name: &str| ic_wire::Encoding::Frame {
        method: ic_wire::method_id(name),
        direction: ic_wire::Direction::Response,
    };
    let register = async |caller| {
        for _ in 0..5 {
            let raw = pic
                .update_call(hub, caller, "register_local", request("register_local"))
                .await
                .unwrap();
            let result: Placement =
                ic_wire::decode(&raw, response("register_local"), sharding::LIMITS).unwrap();
            if !matches!(result, Placement::Pending(_) | Placement::Busy) {
                return result;
            }
            settle(pic).await;
        }
        panic!("growth remained pending");
    };
    let alice = CanisterId::self_authenticating(b"alice");
    let bob = CanisterId::self_authenticating(b"bob");
    let carol = CanisterId::self_authenticating(b"carol");
    let _lost = pic
        .submit_call(hub, alice, "register_local", request("register_local"))
        .await
        .unwrap();
    let first = register(alice).await;
    let first_id = first.assigned_target().unwrap().unwrap().1;
    assert_eq!(first_id, spare);
    // The first capacity ordinal has the same bytes as an all-zero account key.
    let target = sharding::ShardTarget {
        allocation: first.assigned_target().unwrap().unwrap().0.clone(),
        shard: first_id,
    };
    let released = pic
        .update_call(
            root.id,
            hub,
            sharding::ReleaseShard::NAME,
            sharding::ReleaseShard::encode_request(&([0; 32], target)).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        sharding::ReleaseShard::decode_reply(&released).unwrap(),
        Err("account releases belong to the hub for this pool".into())
    );
    pic.stop_canister(root.id, Some(root.operator))
        .await
        .unwrap();
    assert_eq!(register(bob).await, first);
    pic.start_canister(root.id, Some(root.operator))
        .await
        .unwrap();
    let pending = pic
        .update_call(hub, carol, "register_local", request("register_local"))
        .await
        .unwrap();
    assert!(matches!(
        ic_wire::decode::<Placement>(&pending, response("register_local"), sharding::LIMITS)
            .unwrap(),
        Placement::Pending(_)
    ));
    settle(pic).await;
    // Both requests may deliver to the installed shard. Lose one reply after delivery.
    let _lost_write = pic
        .submit_call(hub, carol, "register_local", request("register_local"))
        .await
        .unwrap();
    let second = register(carol).await;
    let second_id = second.assigned_target().unwrap().unwrap().1;
    assert_eq!(register(carol).await, second);
    assert!(
        pic.update_call(
            hub,
            CanisterId::anonymous(),
            "register_local",
            request("register_local")
        )
        .await
        .is_err()
    );
    assert_ne!(first_id, second_id);
    assert_eq!(
        register(CanisterId::self_authenticating(b"dave")).await,
        second
    );
    assert_eq!(
        register(CanisterId::self_authenticating(b"eve")).await,
        Placement::Full
    );
    for shard in [first_id, second_id] {
        let raw = pic
            .query_call(
                CanisterId::from_slice(shard.as_slice()),
                alice,
                "record_count",
                request("record_count"),
            )
            .await
            .unwrap();
        assert_eq!(
            ic_wire::decode::<u32>(&raw, response("record_count"), sharding::LIMITS).unwrap(),
            2
        );
    }
    canic_qualification::upgrade_stopped(pic, root.id, wasm, vec![], Some(root.operator))
        .await
        .unwrap();
    assert_eq!(register(carol).await, second);
    pic.stop_canister(root.id, Some(root.operator))
        .await
        .unwrap();
    canic_qualification::upgrade_stopped(
        pic,
        hub,
        std::fs::read(artifacts().join("hub.wasm")).unwrap(),
        vec![],
        None,
    )
    .await
    .unwrap();
    assert_eq!(register(alice).await, first);
    let raw = pic
        .query_call(hub, alice, "local_route", request("local_route"))
        .await
        .unwrap();
    assert_eq!(
        ic_wire::decode::<Option<_>>(&raw, response("local_route"), sharding::LIMITS).unwrap(),
        Some(first_id)
    );
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}
