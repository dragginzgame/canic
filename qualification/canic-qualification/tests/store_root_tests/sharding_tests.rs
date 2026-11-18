//! The hub is an independent installed application. Only Root controls the shards.
use super::*;
use canic::sharding::{self, Placement};

async fn user(pic: &PocketIc, hub: CanisterId, caller: CanisterId, method: &str) -> Placement {
    let reply = pic.update_call(hub, caller, method, vec![]).await.unwrap();
    ic_wire::decode(&reply, ic_wire::Encoding::Payload, sharding::LIMITS).unwrap()
}
#[tokio::test(flavor = "current_thread")]
async fn signup_retains_the_first_shard_across_reply_loss_growth_and_hub_upgrade() {
    exercise_pool(None).await;
    if let Some(wasm) = std::env::var_os("TOKO_USER_SHARD_WASM") {
        exercise_pool(Some(PathBuf::from(wasm))).await;
    }
}

async fn exercise_pool(toko_wasm: Option<PathBuf>) {
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
    let (root, supplier) = readiness_tests::fixture_with_root(pic, root_wasm.clone()).await;
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
[apps.toko.fleets.demo.shard_pool]
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
            2 * management::CREATION_CYCLES,
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
    assert!(text.contains("2 shards × 1 lifetime keys"));
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
                &[0_u8; 32],
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
    let slot = ChildId::ALL[a.slot as usize];
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
            &children::Request { slot, value: exact },
            stored::LIMITS,
        )
        .await;
    assert_eq!(recovered, journal);
    assert_eq!(
        a.slot, 1,
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
    for (id, wasm) in [(hub, hub_wasm), (root.id, root_wasm)] {
        canic_qualification::upgrade_stopped(pic, id, wasm, vec![], Some(root.operator))
            .await
            .unwrap();
    }
    assert_eq!(user(pic, hub, alice, "signup").await, original);
    assert_eq!(user(pic, hub, alice, "lookup").await, original);
    assert_eq!(user(pic, hub, eve, "signup").await, Placement::Full);
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}

async fn check_db_owner(pic: &PocketIc, id: CanisterId, hub: CanisterId, operator: CanisterId) {
    // The installed DB witness only opens its catalogue for the owner passed at init.
    use canic::db::{RPC_LIMITS, inspection};
    let frame = |direction| ic_wire::Encoding::Frame {
        method: inspection::INSPECT,
        direction,
    };
    let request = ic_wire::encode(
        &inspection::Request::Catalogue,
        frame(ic_wire::Direction::Request),
        RPC_LIMITS,
    )
    .unwrap();
    let bytes = pic
        .query_call(id, hub, "db_inspect", request.clone())
        .await
        .unwrap();
    let reply: inspection::Reply =
        ic_wire::decode(&bytes, frame(ic_wire::Direction::Response), RPC_LIMITS).unwrap();
    assert!(reply.is_ok());
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
        pool: "users".into(),
        max_shards: 2,
        keys_per_shard: 1,
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
    for slot in ChildId::ALL {
        root.update::<_, ()>(
            "inject_child_fault",
            children::INJECT,
            &children::Request { slot, value: 2_u8 },
            management::LIMITS,
        )
        .await;
    }
    let grant = stored::ShardPool {
        arguments: management::application::Arguments::new(vec![]).unwrap(),
        root: root.id.to_text(),
        hub: hub.to_text(),
        pool: "users".into(),
        max_shards: 2,
        keys_per_shard: 1,
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
    for slot in ChildId::ALL {
        let journal: management::Status = root
            .update("observe_child", children::STATUS, &slot, management::LIMITS)
            .await;
        if slot.index() == original.slot as usize {
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
                    &children::Request { slot, value: input },
                    stored::LIMITS,
                )
                .await;
            assert_eq!(observed, journal);
        } else {
            assert_eq!(journal.phase, management::Phase::Empty);
        }
    }
    assert_eq!(user(pic, hub, bob, "lookup").await, Placement::Unknown);
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}
