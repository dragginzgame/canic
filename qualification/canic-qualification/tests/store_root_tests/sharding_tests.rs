//! The hub is an independent installed application. Only Root controls the shards.
use super::*;
use canic::sharding::standby::StandbyPoolError;
use canic::sharding::{self, Placement};
use canic_profile_protocol::ProfileError;

type Registration = Result<Placement, ProfileError>;

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
async fn large_subnet_requires_exact_controller_opt_in() {
    let (mut server, owner) = support::instance(&canic_qualification::server_directory()).await;
    let pic = owner.client();
    let wasm = std::fs::read(artifacts().join("root.wasm")).unwrap();
    let (root, _) = readiness_tests::fixture_with_root(pic, wasm.clone()).await;
    let grant = |max_children, allow_large_subnet| stored::SubnetCapacity {
        root: root.id.to_text(),
        max_children,
        allow_large_subnet,
    };
    let large = grant(40_001, true);
    let request = ic_wire::encode(
        &large,
        ic_wire::Encoding::Frame {
            method: stored::CONFIGURE_SUBNET_CAPACITY,
            direction: ic_wire::Direction::Request,
        },
        stored::LIMITS,
    )
    .unwrap();
    assert!(
        pic.update_call(
            root.id,
            pic.create_canister().await,
            "configure_subnet_capacity",
            request
        )
        .await
        .is_err()
    );
    for (approval, expected) in [
        (grant(40_000, false), Ok(())),
        (grant(40_001, false), Err(management::Refusal::Binding)),
        (large.clone(), Ok(())),
        (large, Ok(())),
        (grant(40_000, false), Err(management::Refusal::Occupied)),
    ] {
        assert_eq!(
            root.update::<_, Result<(), management::Refusal>>(
                "configure_subnet_capacity",
                stored::CONFIGURE_SUBNET_CAPACITY,
                &approval,
                stored::LIMITS
            )
            .await,
            expected
        );
    }
    check_occupied_placement_endpoints(&root).await;
    pic.upgrade_canister(root.id, wasm, vec![], Some(root.operator))
        .await
        .unwrap();
    assert_eq!(
        root.update::<_, Result<(), management::Refusal>>(
            "configure_subnet_capacity",
            stored::CONFIGURE_SUBNET_CAPACITY,
            &grant(40_000, false),
            stored::LIMITS,
        )
        .await,
        Err(management::Refusal::Occupied)
    );
    assert_eq!(
        owner.close(std::time::Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}

async fn check_occupied_placement_endpoints(root: &Client<'_>) {
    fn frame<T: serde::Serialize>(method: u64, value: &T) -> Vec<u8> {
        ic_wire::encode(
            value,
            ic_wire::Encoding::Frame {
                method,
                direction: ic_wire::Direction::Request,
            },
            management::LIMITS,
        )
        .unwrap()
    }
    let pic = root.pic;
    // Malformed input distinguishes admission from decoding without issuing effects.
    for (method, fenced) in [
        ("observe_root_custody", true),
        ("change_store_admission", true),
        ("remove_store_artifact", true),
        ("maintain_store", true),
        ("recover_store_maintenance", true),
        ("observe_child_status", false),
        ("observe_store_maintenance", false),
        ("observe_store", false),
        ("observe_child_deployments", false),
        ("publish_store_artifact", false),
        ("stage_child_creation", false),
        ("reconcile_child_creation", false),
        ("stage_child_upgrade", false),
        ("reconcile_child_upgrade", false),
    ] {
        let reject = pic
            .update_call(root.id, root.operator, method, vec![])
            .await
            .unwrap_err();
        assert!(
            reject.reject_message.contains("management endpoint"),
            "{method}: {reject:?}"
        );
        assert_eq!(
            reject
                .reject_message
                .contains("shard pool custody required"),
            fenced,
            "{method}: {reject:?}"
        );
    }
    let read = management::publication::ObservationRequest::Admission(
        management::publication::InfoRequest {
            store: root.id.to_text(),
            expires_at_ns: 0,
        },
    );
    let denied = pic
        .update_call(
            root.id,
            root.operator,
            "observe_store",
            frame(management::publication::OBSERVATION, &read),
        )
        .await
        .unwrap_err();
    assert!(
        denied
            .reject_message
            .contains("shard pool custody required"),
        "{denied:?}"
    );
    for (name, args) in [
        (
            "child_status",
            frame(
                children::READ_STATUS,
                &children::StatusRequest::Creation(ChildId::FIRST),
            ),
        ),
        (
            "child_status",
            frame(
                children::READ_STATUS,
                &children::StatusRequest::Upgrade(children::Request {
                    slot: ChildId::FIRST,
                    value: None,
                }),
            ),
        ),
        (
            "store_maintenance_status",
            frame(store_maintenance::STATUS, &()),
        ),
    ] {
        pic.query_call(root.id, CanisterId::anonymous(), name, args)
            .await
            .unwrap();
    }
    let status = frame(
        children::READ_STATUS,
        &children::StatusRequest::Creation(ChildId::FIRST),
    );
    let outsider = CanisterId::anonymous();
    let public = pic
        .query_call(root.id, outsider, "child_status", status.clone())
        .await
        .unwrap();
    let replicated = root.observe_creation(ChildId::FIRST).await;
    let children::ObservedStatus::Creation(public) = ic_wire::decode(
        &public,
        ic_wire::Encoding::Frame {
            method: children::READ_STATUS,
            direction: ic_wire::Direction::Response,
        },
        management::LIMITS,
    )
    .unwrap() else {
        panic!("expected creation status")
    };
    assert_eq!(public, replicated);
    let status = frame(
        children::OBSERVE_STATUS,
        &children::ObserveStatus::Creation(ChildId::FIRST),
    );
    let denied = pic
        .update_call(root.id, outsider, "observe_child_status", status)
        .await
        .unwrap_err();
    assert!(
        denied.reject_message.contains("controller required"),
        "{denied:?}"
    );
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
    let root_wasm = std::fs::read(artifacts().join("root.wasm")).unwrap();
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
        crate::store_root_tests::hub_args(&root),
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
    assert!(text.contains("Creation limit: 2 shards") && text.contains("Hub policy: 1 live keys"));
    let result: serde_json::Value = serde_json::from_slice(&plan(true)).unwrap();
    assert_eq!(result["action"], "shard_pool");
    let bytes: Vec<u8> = serde_json::from_value(result["request"].clone()).unwrap();
    let frame = |direction| ic_wire::Encoding::Frame {
        method: stored::APPROVE_CHILD_CREATION,
        direction,
    };
    let stored::ChildCreationGrant::Shared(approval) =
        ic_wire::decode(&bytes, frame(ic_wire::Direction::Request), stored::LIMITS).unwrap()
    else {
        panic!("expected shared creation grant")
    };
    assert_eq!(result["approval"], serde_json::to_value(&approval).unwrap());
    assert_eq!(approval.reference.manifest, manifest);
    assert_eq!(approval.arguments.as_slice(), &init);
    let reply = pic
        .update_call(root.id, root.operator, "approve_child_creation", bytes)
        .await
        .unwrap();
    let configured: Result<(), management::Refusal> =
        ic_wire::decode(&reply, frame(ic_wire::Direction::Response), stored::LIMITS).unwrap();
    configured.unwrap();
    super::configure_routes(&root, hub, "user_shard", 1)
        .await
        .unwrap();
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
    let original = user(pic, hub, alice, "signup").await;
    let Placement::Assigned(a, first_pid) = &original else {
        panic!("{original:?}")
    };
    assert_eq!(*a, allocation);
    if inline.is_some() {
        let refusal = canic_qualification::upgrade_stopped(
            pic,
            root.id,
            root_wasm,
            vec![],
            Some(root.operator),
        )
        .await
        .unwrap_err();
        assert!(refusal.reject_message.contains("trapped"), "{refusal:?}");
        configure(&root, &approval).await.unwrap();
        assert_eq!(user(pic, hub, alice, "signup").await, original);
        assert_eq!(
            owner.close(Duration::from_secs(10)).await,
            ic_testkit::server::Deletion::Attempted
        );
        assert!(server.stop().unwrap().process.cleanup.is_empty());
        return;
    }
    for removed in ["assign_shard", "lookup_shard", "release_shard"] {
        assert!(
            pic.update_call(root.id, hub, removed, vec![])
                .await
                .is_err()
        );
    }
    // The same hub code, Root address, operator and account key confer no pool authority.
    let unrelated = pic
        .create_canister_with_settings(Some(root.operator), None)
        .await;
    pic.install_canister(
        unrelated,
        hub_wasm.clone(),
        crate::store_root_tests::hub_args(&root),
        Some(root.operator),
    )
    .await;
    super::configure_routes(&root, unrelated, "user_shard", 1)
        .await
        .unwrap();
    let retained = pic.get_stable_memory(root.id).await;
    assert_eq!(
        user(pic, unrelated, alice, "lookup").await,
        Placement::Unknown
    );
    {
        let method = "signup";
        let refusal = pic
            .update_call(unrelated, alice, method, vec![])
            .await
            .unwrap_err();
        assert!(
            refusal
                .reject_message
                .contains("shard capacity reply unavailable"),
            "{refusal:?}"
        );
    }
    assert!(pic.get_stable_memory(root.id).await == retained);
    assert_eq!(user(pic, hub, alice, "lookup").await, original);
    let slot = ChildId::placement(a.slot as usize).unwrap();
    let journal: management::Status = root.observe_creation(slot).await;
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
            "reconcile_child_creation",
            canic_management_protocol::application::RECONCILE_CHILD_CREATION,
            &children::AllocationRequest {
                slot,
                value: canic_management_protocol::application::Deployment::<(), _>::Application(
                    exact,
                ),
            },
            stored::LIMITS,
        )
        .await;
    assert_eq!(recovered, journal);
    let _discarded = pic.submit_call(hub, alice, "signup", vec![]).await.unwrap();
    assert!(matches!(
        user(pic, hub, bob, "signup").await,
        Placement::Pending(_)
    ));
    settle(pic).await;
    let Placement::Assigned(b, second_pid) = user(pic, hub, bob, "signup").await else {
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
    for (id, wasm) in [(hub, hub_wasm.clone()), (root.id, root_wasm.clone())] {
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
    assert!(text.contains("Creation limit: 2 shards") && text.contains("Hub policy: 1 live keys"));
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
        pic.update_call(root.id, alice, "approve_child_creation", source.clone())
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
        .update_call(root.id, root.operator, "approve_child_creation", source)
        .await
        .unwrap();
    ic_wire::decode::<Result<(), management::Refusal>>(
        &configured,
        ic_wire::Encoding::Frame {
            method: stored::APPROVE_CHILD_CREATION,
            direction: ic_wire::Direction::Response,
        },
        stored::LIMITS,
    )
    .unwrap()
    .unwrap();
    for method in ["project", "lookup_project"] {
        let refusal = pic
            .update_call(hub, alice, method, vec![1; 32])
            .await
            .unwrap_err();
        assert!(
            refusal.reject_message.contains(if method == "project" {
                "hub: Refused(Unauthorized)"
            } else {
                "hub: Rejected("
            }),
            "{method}: {refusal:?}"
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
    // Completed projects must stay resolvable on a reopened registry with its creator offline.
    pic.stop_canister(root.id, Some(root.operator))
        .await
        .unwrap();
    canic_qualification::upgrade_stopped(
        pic,
        unrelated,
        hub_wasm.clone(),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
    assert_eq!(
        project(pic, unrelated, alice, "lookup_project", 1).await,
        project_a
    );
    assert_eq!(
        project(pic, unrelated, alice, "project", 2).await,
        project_b
    );
    pic.start_canister(root.id, Some(root.operator))
        .await
        .unwrap();
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
            "[apps.toko.fleets.demo]\nsubnet_child_limit = 9",
        )
        .replace("max_shards=2", "max_shards=6\nincrease_from=2")
        .replace("max_instances = 2", "max_instances = 6\nincrease_from=2");
    std::fs::write(&config, text).unwrap();
    let expanded: serde_json::Value = serde_json::from_slice(&plan(true)).unwrap();
    let cap: stored::SubnetCapacity =
        serde_json::from_value(expanded["capacity"]["approval"].clone()).unwrap();
    assert_eq!(cap.max_children, 9);
    let changes = expanded["increases"].as_array().unwrap();
    assert_eq!(changes.len(), 2);
    let old_request = |name| ic_wire::Encoding::Frame {
        method: ic_wire::method_id(name),
        direction: ic_wire::Direction::Request,
    };
    for (endpoint, bytes) in [
        (
            "configure_subnet_capacity",
            ic_wire::encode(
                &cap,
                old_request("canic.placement.configure_capacity.v1"),
                stored::LIMITS,
            )
            .unwrap(),
        ),
        (
            "increase_pool_limit",
            ic_wire::encode(
                &stored::IncreasePoolLimit::Instances(projects.clone(), 6),
                old_request("canic.placement.increase_pool.v1"),
                stored::LIMITS,
            )
            .unwrap(),
        ),
    ] {
        assert!(
            pic.update_call(root.id, root.operator, endpoint, bytes)
                .await
                .is_err()
        );
    }
    for (item, endpoint, method) in std::iter::once((
        &expanded["capacity"],
        "configure_subnet_capacity",
        stored::CONFIGURE_SUBNET_CAPACITY,
    ))
    .chain(
        changes
            .iter()
            .map(|item| (item, "increase_pool_limit", stored::INCREASE_POOL_LIMIT)),
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
                "increase_pool_limit",
                stored::INCREASE_POOL_LIMIT,
                &stored::IncreasePoolLimit::Instances(forged, 6),
                stored::LIMITS,
            )
            .await;
        assert_eq!(result, Err(management::Refusal::Binding));
    }
    let stale: Result<(), management::Refusal> = root
        .update(
            "increase_pool_limit",
            stored::INCREASE_POOL_LIMIT,
            &stored::IncreasePoolLimit::Instances(projects.clone(), 7),
            stored::LIMITS,
        )
        .await;
    assert_eq!(stale, Err(management::Refusal::Occupied));
    for change in [
        stored::IncreasePoolLimit::Instances(projects.clone(), 9),
        stored::IncreasePoolLimit::Shards(approval.clone(), 9),
    ] {
        let result: Result<(), management::Refusal> = root
            .update(
                "increase_pool_limit",
                stored::INCREASE_POOL_LIMIT,
                &change,
                stored::LIMITS,
            )
            .await;
        assert_eq!(result, Err(management::Refusal::Binding));
    }
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
    let third_shard = user(pic, hub, eve, "signup").await;
    let third_project = project(pic, unrelated, alice, "lookup_project", 3).await;
    assert!(matches!(third_shard, Placement::Assigned(..)));
    assert!(matches!(third_project, Placement::Assigned(..)));
    assert!(matches!(
        project(pic, unrelated, alice, "project", 4).await,
        Placement::Pending(_)
    ));
    settle(pic).await;

    assert!(matches!(
        project(pic, unrelated, alice, "project", 5).await,
        Placement::Pending(_)
    ));
    settle(pic).await;
    // Two hubs contend for the ninth slot. Drop both responses and resolve by their original keys.
    let fourth_user = CanisterId::self_authenticating(b"fourth");
    let _shard_call = pic
        .submit_call(hub, fourth_user, "signup", vec![])
        .await
        .unwrap();
    let _project_call = pic
        .submit_call(unrelated, alice, "project", vec![6; 32])
        .await
        .unwrap();
    settle(pic).await;
    canic_qualification::upgrade_stopped(pic, root.id, root_wasm, vec![], Some(root.operator))
        .await
        .unwrap();
    let shard = user(pic, hub, fourth_user, "signup").await;
    let project_retry = project(pic, unrelated, alice, "project", 6).await;
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
    if let Some(path) = std::env::var_os("CANIC_PREDECESSOR_SHARD_ROOT_WASM") {
        pic.advance_time(Duration::from_secs(600)).await;
        pic.tick().await;
        let refusal = canic_qualification::upgrade_stopped(
            pic,
            root.id,
            std::fs::read(path).unwrap(),
            vec![],
            Some(root.operator),
        )
        .await
        .unwrap_err();
        assert!(refusal.reject_message.contains("trapped"), "{refusal:?}");
        configure_project(&root, &projects).await.unwrap();
    }

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
    use canic::method::Contract;
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let (root, store) = readiness_tests::fixture_with_root(
        pic,
        std::fs::read(artifacts().join("root.wasm")).unwrap(),
    )
    .await;
    let hub = pic.create_canister().await;
    pic.install_canister(
        hub,
        std::fs::read(artifacts().join("hub.wasm")).unwrap(),
        crate::store_root_tests::hub_args(&root),
        None,
    )
    .await;
    super::configure_routes(&root, hub, "user_shard", 100_000)
        .await
        .unwrap();
    let manifest = store.publish_bytes("one", b"\0asm\x01\0\0\0").await;
    let approval = stored::ShardPool {
        arguments: management::application::Arguments::new(vec![]).unwrap(),
        root: root.id.to_text(),
        hub: hub.to_text(),
        pool: "user_shard".into(),
        max_shards: 2,
        creation_budget: 0,
        expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 600_000_000_000,
        reference: stored::Reference {
            store: store.id.to_text(),
            manifest,
        },
    };
    let caller = CanisterId::self_authenticating(b"user");
    let encoded = ic_wire::encode(
        &stored::ChildCreationGrant::Shared(approval.clone()),
        ic_wire::Encoding::Frame {
            method: stored::APPROVE_CHILD_CREATION,
            direction: ic_wire::Direction::Request,
        },
        stored::LIMITS,
    )
    .unwrap();
    assert!(
        pic.update_call(root.id, caller, "approve_child_creation", encoded)
            .await
            .is_err()
    );
    let project_hub = pic.create_canister().await;
    pic.install_canister(
        project_hub,
        std::fs::read(artifacts().join("hub.wasm")).unwrap(),
        crate::store_root_tests::hub_args(&root),
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
    // The subnet backstop does not increase the addressable shard family.
    assert_eq!(
        configure(
            &root,
            &stored::ShardPool {
                max_shards: stored::MAX_SHARDS + 1,
                ..approval.clone()
            }
        )
        .await,
        Err(management::Refusal::Binding)
    );
    configure(&root, &approval).await.unwrap();
    // A large backstop alone cannot authorize spending on either pool.
    root.update::<_, Result<(), management::Refusal>>(
        "configure_subnet_capacity",
        stored::CONFIGURE_SUBNET_CAPACITY,
        &stored::SubnetCapacity {
            allow_large_subnet: false,
            root: root.id.to_text(),
            max_children: 20_010,
        },
        stored::LIMITS,
    )
    .await
    .unwrap();
    assert_eq!(
        user(pic, hub, caller, "signup").await,
        Placement::GrowthRefused
    );
    assert_eq!(user(pic, hub, caller, "lookup").await, Placement::Unknown);
    let spare = root
        .update::<_, Result<canic::sharding::standby::Asset, StandbyPoolError>>(
            canic::sharding::standby::RefillStandby::NAME,
            canic::sharding::standby::RefillStandby::ID,
            &canic::sharding::standby::Refill {
                root: root.id.as_slice().try_into().unwrap(),
                sequence: 0,
                cycles: management::CREATION_CYCLES,
                expires_at_ns: approval.expires_at_ns,
            },
            canic::sharding::standby::RefillStandby::REPLY,
        )
        .await
        .unwrap();
    assert!(matches!(spare, canic::sharding::standby::Asset::Empty(_)));
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
    for slot in ChildId::HISTORICAL_SERVICES {
        let journal: management::Status = root.observe_creation(slot).await;
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
        "approve_child_creation",
        stored::APPROVE_CHILD_CREATION,
        &stored::ChildCreationGrant::Shared(a.clone()),
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
        creation_budget: 6 * management::CREATION_CYCLES,
        expires_at_ns,
    }
}

async fn configure_project(
    root: &Client<'_>,
    grant: &stored::InstancePool,
) -> Result<(), management::Refusal> {
    root.update(
        "approve_child_creation",
        stored::APPROVE_CHILD_CREATION,
        &stored::ChildCreationGrant::Dedicated(grant.clone()),
        stored::LIMITS,
    )
    .await
}

#[tokio::test(flavor = "current_thread")]
async fn lost_create_result_keeps_the_original_claim_and_blocks_another_creation() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let wasm = std::fs::read(artifacts().join("root-faults.wasm")).unwrap();
    let (root, supplier) = readiness_tests::fixture_with_root(pic, wasm.clone()).await;
    let hub = pic.create_canister().await;
    pic.install_canister(
        hub,
        std::fs::read(artifacts().join("hub.wasm")).unwrap(),
        crate::store_root_tests::hub_args(&root),
        None,
    )
    .await;
    super::configure_routes(&root, hub, "user_shard", 1)
        .await
        .unwrap();
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
        allow_large_subnet: false,
        root: root.id.to_text(),
        max_children: 2,
    };
    root.update::<_, Result<(), management::Refusal>>(
        "configure_subnet_capacity",
        stored::CONFIGURE_SUBNET_CAPACITY,
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
    assert_eq!(
        user(pic, hub, bob, "signup").await,
        Placement::Pending(original.clone())
    );
    {
        let slot = ChildId::placement(original.slot as usize).unwrap();
        let journal: management::Status = root.observe_creation(slot).await;

        assert_eq!(journal.phase, management::Phase::CreateIssued);
        assert_eq!(journal.attempts, [1, 0, 0]);
        assert!(journal.worker.is_none());
        let input = stored::Stage {
            approval: journal.approval.clone().unwrap(),
            reference: grant.reference.clone(),
        };
        let observed: management::Status = root
            .update(
                "reconcile_child_creation",
                canic_management_protocol::application::RECONCILE_CHILD_CREATION,
                &children::AllocationRequest {
                    slot,
                    value: canic_management_protocol::application::Deployment::<_, ()>::Stored(
                        input,
                    ),
                },
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
        crate::store_root_tests::hub_args(&root),
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

#[tokio::test(flavor = "current_thread")]
async fn full_fleet_upgrades_a_store_backed_service_with_child_only_evidence() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let (root, supplier, mut input, second) = fixture_with_module(pic, "root.wasm").await;
    root.update::<_, Result<(), management::Refusal>>(
        "configure_subnet_capacity",
        stored::CONFIGURE_SUBNET_CAPACITY,
        &stored::SubnetCapacity {
            allow_large_subnet: false,
            root: root.id.to_text(),
            max_children: 1,
        },
        stored::LIMITS,
    )
    .await
    .unwrap();
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    input.approval.execute_at_ns = now;
    stage(&root, &input).await.unwrap();
    settle(pic).await;
    let evidence = async || {
        root.update::<_, management::application::Custody>(
            "observe_child_deployments",
            management::application::CHILD_EVIDENCE,
            &(),
            management::application::LIMITS,
        )
        .await
    };
    let prior = evidence().await.root.current.stored;
    let creation = &prior.children[0].creation;
    assert_eq!(creation.phase, management::Phase::Ready);
    let child = creation.worker.clone().unwrap();
    let change = stored::Stage {
        approval: upgrade::Approval {
            manager: root.id.to_text(),
            child: child.clone(),
            previous_module: input.approval.module_hash,
            module_hash: second.hash,
            execute_at_ns: pic.get_time().await.as_nanos_since_unix_epoch(),
            expires_at_ns: input.approval.expires_at_ns,
        },
        reference: stored::Reference {
            store: supplier.id.to_text(),
            manifest: second,
        },
    };
    let req = Request {
        slot: ChildId::FIRST,
        value: &change,
    };
    root.update::<_, Result<upgrade::Status, management::Refusal>>(
        "stage_child_upgrade",
        canic_management_protocol::application::STAGE_CHILD_UPGRADE,
        &canic_management_protocol::children::Request {
            slot: req.slot,
            value: canic_management_protocol::application::Deployment::<_, ()>::Stored(&req.value),
        },
        stored::LIMITS,
    )
    .await
    .unwrap();
    settle(pic).await;
    let after = evidence().await.root.current.stored;
    let status = root
        .update::<_, upgrade::Status>(
            "reconcile_child_upgrade",
            canic_management_protocol::application::RECONCILE_CHILD_UPGRADE,
            &canic_management_protocol::children::Request {
                slot: req.slot,
                value: canic_management_protocol::application::Deployment::<_, ()>::Stored(
                    &req.value,
                ),
            },
            stored::LIMITS,
        )
        .await;
    assert_eq!(status.phase, upgrade::Phase::Ready);
    assert_eq!(after.children[0].creation, *creation);
    assert_eq!(after.children[0].upgrades, [status]);
    let delivery = &after.sources[0].as_ref().unwrap().upgrades[0];
    assert_eq!(delivery.reference, change.reference);
    assert_eq!(delivery.state, stored::PinState::Retained);
    assert_eq!(
        pic.canister_status(CanisterId::from_text(child).unwrap(), Some(root.id))
            .await
            .unwrap()
            .module_hash,
        Some(change.approval.module_hash.to_vec())
    );
    let frame = ic_wire::encode(
        &(),
        ic_wire::Encoding::Frame {
            method: management::application::CUSTODY,
            direction: ic_wire::Direction::Request,
        },
        stored::LIMITS,
    )
    .unwrap();
    let refused = pic
        .update_call(root.id, root.operator, "observe_root_custody", frame)
        .await
        .unwrap_err();
    assert!(
        refused
            .reject_message
            .contains("shard pool custody required")
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
        crate::store_root_tests::hub_args(root),
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
async fn approved_hub_recovers_its_first_growth_ordinal() {
    use canic::method::Contract;

    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance(&binary).await;
    let pic = owner.client();
    let wasm = std::fs::read(artifacts().join("root.wasm")).unwrap();
    let (root, store) = readiness_tests::fixture_with_root(pic, wasm).await;
    let (hub, mut grant) = fresh_user_shard_grant(&root, &store).await;
    grant.max_shards = 1;
    root.update::<_, Result<(), management::Refusal>>(
        "configure_subnet_capacity",
        stored::CONFIGURE_SUBNET_CAPACITY,
        &stored::SubnetCapacity {
            allow_large_subnet: false,
            root: root.id.to_text(),
            max_children: 1,
        },
        stored::LIMITS,
    )
    .await
    .unwrap();
    configure(&root, &grant).await.unwrap();

    // Occupied pools admit both read-only Store variants, but only for the controller.
    use management::publication::{self, Observation, ObservationRequest};
    for request in [
        ObservationRequest::Info(publication::InfoRequest {
            store: store.id.to_text(),
            expires_at_ns: grant.expires_at_ns - 400_000_000_000,
        }),
        ObservationRequest::Custody(publication::CustodyRequest {
            store: store.id.to_text(),
            expires_at_ns: grant.expires_at_ns - 400_000_000_000,
            release: grant.reference.manifest.release.clone(),
        }),
    ] {
        let args = ic_wire::encode(
            &request,
            ic_wire::Encoding::Frame {
                method: publication::OBSERVATION,
                direction: ic_wire::Direction::Request,
            },
            management::LIMITS,
        )
        .unwrap();
        assert!(
            pic.update_call(root.id, hub, "observe_store", args)
                .await
                .is_err()
        );
        let evidence = root
            .update::<_, publication::ObservationReply>(
                "observe_store",
                publication::OBSERVATION,
                &request,
                publication::RESPONSE_LIMITS,
            )
            .await
            .unwrap();
        match (request, evidence) {
            (ObservationRequest::Info(_), Observation::Info(info)) => assert_eq!(info.releases, 1),
            (ObservationRequest::Custody(_), Observation::Custody(custody)) => {
                assert_eq!(custody.status.manifest, grant.reference.manifest)
            }
            _ => panic!("unexpected Store evidence"),
        }
    }

    let request = crate::store_root_tests::placement_request(
        canic::sharding::payment::Demand::ShardCapacity("user_shard".into(), 0),
    );
    let assign = async || {
        acquire_fixture(
            &root,
            hub,
            canic::sharding::payment::Demand::ShardCapacity("user_shard".into(), 0),
        )
        .await
    };
    let unpaid = pic
        .update_call(
            root.id,
            hub,
            canic::sharding::payment::AcquirePlacement::NAME,
            request,
        )
        .await
        .unwrap();
    assert!(matches!(
        canic::sharding::payment::AcquirePlacement::decode_reply(&unpaid).unwrap(),
        Ok(canic::sharding::payment::Payment::Quoted(_))
    ));
    let Placement::Pending(original) = assign().await else {
        panic!("first key must reserve a shard")
    };
    settle(pic).await;
    let assigned = assign().await;
    let (allocation, _) = assigned.assigned_target().unwrap().expect("ready shard");
    assert_eq!(allocation, &original);
    // Publication through Root must still work when the only allocation slot is occupied.
    let custody = pic.get_stable_memory(root.id).await;
    publication_tests::publish_shard_successor(&root, &store).await;
    assert_eq!(pic.get_stable_memory(root.id).await, custody);
    assert_eq!(assign().await, assigned);
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert!(server.stop().unwrap().process.cleanup.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn ten_thousand_keys_per_shard_survive_rollover_and_hub_reopen() {
    let (_, binary) = support::inputs();
    let (mut server, owner) = support::instance_with_lifetime(
        &binary,
        ic_testkit::pocket_ic::PocketIcBuilder::new().with_application_subnet(),
        Duration::from_secs(900),
    )
    .await;
    let pic = owner.client();
    let wasm = std::fs::read(artifacts().join("root.wasm")).unwrap();
    let (root, store) = readiness_tests::fixture_with_root(pic, wasm.clone()).await;
    let (hub, approval) = fresh_user_shard_grant(&root, &store).await;

    assert_eq!(
        super::configure_routes(&root, hub, "user_shard", 100_001).await,
        Err(canic::sharding::ShardError::InvalidPolicy)
    );
    super::configure_routes(&root, hub, "user_shard", 10_000)
        .await
        .unwrap();
    configure(&root, &approval).await.unwrap();
    let account = |n: u32| CanisterId::self_authenticating(n.to_le_bytes());
    let mut assignments = Vec::new();
    for first in [1, 10_001] {
        assert!(matches!(
            user(pic, hub, account(first), "signup").await,
            Placement::Pending(_)
        ));
        settle(pic).await;
        let assigned = user(pic, hub, account(first), "signup").await;
        assert!(matches!(assigned, Placement::Assigned(..)));
        pic.stop_canister(root.id, Some(root.operator))
            .await
            .unwrap();
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
        pic.start_canister(root.id, Some(root.operator))
            .await
            .unwrap();
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
    canic_qualification::upgrade_stopped(
        pic,
        hub,
        std::fs::read(artifacts().join("hub.wasm")).unwrap(),
        vec![],
        None,
    )
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
    let wasm = std::fs::read(artifacts().join("root.wasm")).unwrap();
    let (root, store) = readiness_tests::fixture_with_root(pic, wasm.clone()).await;
    let (hub, shared) = fresh_user_shard_grant(&root, &store).await;
    let mut grant = instance_grant(root.id, hub, shared.reference, shared.expires_at_ns);
    grant.max_instances = 3;
    root.update::<_, Result<(), management::Refusal>>(
        "configure_subnet_capacity",
        stored::CONFIGURE_SUBNET_CAPACITY,
        &stored::SubnetCapacity {
            allow_large_subnet: false,
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
        ic_wire::decode::<Result<Payment, canic::sharding::payment::Refusal>>(
            &reply,
            frame(ic_wire::Direction::Response),
            canic::sharding::LIMITS,
        )
        .unwrap()
    };
    let denied = root
        .update::<_, Result<Payment, canic::sharding::payment::Refusal>>(
            canic::sharding::payment::AcquirePlacement::NAME,
            canic::sharding::payment::AcquirePlacement::ID,
            &(Demand::Instance([1; 32]), maximum),
            canic::sharding::LIMITS,
        )
        .await;
    assert_eq!(
        denied.unwrap_err(),
        canic::sharding::payment::Refusal::Unauthorized
    );
    for id in [1, 2] {
        let spare = if id == 1 {
            let asset = root
                .update::<_, Result<Asset, StandbyPoolError>>(
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
            assert_eq!(
                pay(2, 0).await,
                Err(canic::sharding::payment::Refusal::QuoteConflict)
            );
        }
        assert_eq!(quote.spare, spare);
        let charge = quote.creation_fee + quote.working_cycles;
        assert!(quote.creation_fee > 0 && quote.working_cycles > 0 && charge <= maximum);
        assert_eq!(
            pay(id, charge - 1).await,
            Err(canic::sharding::payment::Refusal::InsufficientPayment)
        );
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
    root.update::<_, Result<(), StandbyPoolError>>(
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
                stored::ReadStandbyStatus::NAME,
                stored::ReadStandbyStatus::encode_request(&()).unwrap(),
            )
            .await
            .unwrap();
        stored::ReadStandbyStatus::decode_reply(&reply)
            .unwrap()
            .unwrap()
            .ready
    };
    root.update::<_, Result<Asset, StandbyPoolError>>(
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
        ic_wire::decode::<Result<Payment, canic::sharding::payment::Refusal>>(
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
    let wasm = std::fs::read(artifacts().join("root.wasm")).unwrap();
    let (root, store) = readiness_tests::fixture_with_root(pic, wasm.clone()).await;
    let (hub, grant) = grant_with_child(&root, &store, "hub.wasm").await;
    use canic::{
        method::Contract,
        sharding::standby::{Asset, Refill, RefillStandby},
    };
    root.update::<_, Result<(), management::Refusal>>(
        "configure_subnet_capacity",
        stored::CONFIGURE_SUBNET_CAPACITY,
        &stored::SubnetCapacity {
            allow_large_subnet: false,
            root: root.id.to_text(),
            max_children: 2,
        },
        stored::LIMITS,
    )
    .await
    .unwrap();
    let Asset::Empty(spare) = root
        .update::<_, Result<Asset, StandbyPoolError>>(
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
    super::configure_routes(&root, hub, "user_shard", 2)
        .await
        .unwrap();
    configure(&root, &grant).await.unwrap();
    let grow = async |caller, ordinal| {
        pic.update_call(
            root.id,
            caller,
            canic::sharding::payment::AcquirePlacement::NAME,
            crate::store_root_tests::placement_request(
                canic::sharding::payment::Demand::ShardCapacity("user_shard".into(), ordinal),
            ),
        )
        .await
    };
    crate::store_root_tests::assert_unauthorized(&grow(root.operator, 0).await.unwrap());
    let skipped = grow(hub, 1).await.unwrap();
    assert_eq!(
        crate::store_root_tests::placed(&skipped),
        Placement::GrowthRefused
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
                ic_wire::decode::<Registration>(&raw, response("register_local"), sharding::LIMITS)
                    .unwrap()
                    .unwrap();
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
    // Obtain Root's actual first allocation, then control when its successful reply arrives.
    assert!(matches!(
        acquire_fixture(
            &root,
            hub,
            sharding::payment::Demand::ShardCapacity("user_shard".into(), 0)
        )
        .await,
        Placement::Pending(_)
    ));
    settle(pic).await;
    let ready = grow(hub, 0).await.unwrap();
    let original = crate::store_root_tests::placed(&ready);
    assert_eq!(original.assigned_target().unwrap().unwrap().1, spare);
    revoked_growth(&root, hub, alice, &ready, &wasm, &request("register_local")).await;
    let _lost = pic
        .submit_call(hub, alice, "register_local", request("register_local"))
        .await
        .unwrap();
    let first = register(alice).await;
    assert_eq!(first, original);
    let first_id = first.assigned_target().unwrap().unwrap().1;
    assert_eq!(first_id, spare);
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
        ic_wire::decode::<Registration>(&pending, response("register_local"), sharding::LIMITS)
            .unwrap()
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

// Deliver the captured successful Root reply only after this subject's permission is revoked.
async fn revoked_growth(
    root: &Client<'_>,
    hub: CanisterId,
    subject: CanisterId,
    ready: &[u8],
    root_wasm: &[u8],
    register: &[u8],
) {
    use canic::method::Contract;
    use ic_testkit::held::{Catalog, Fixture, Limits, Reply};

    let pic = root.pic;
    let assigned = crate::store_root_tests::placed(ready);
    let shard = CanisterId::from_slice(assigned.assigned_target().unwrap().unwrap().1.as_slice());
    let hub_memory = pic.get_stable_memory(hub).await;
    let shard_memory = pic.get_stable_memory(shard).await;
    let catalog = Catalog::new(
        &[Reply {
            method: canic::sharding::payment::AcquirePlacement::NAME,
            bytes: ready,
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
    canic_qualification::upgrade_stopped(
        pic,
        root.id,
        held.wasm().to_vec(),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
    let disable = async |value: Option<CanisterId>| {
        Client {
            pic,
            id: hub,
            operator: CanisterId::anonymous(),
        }
        .update::<_, ()>(
            "disable_registration",
            ic_wire::method_id("disable_registration"),
            &value,
            canic::method::LIMITS,
        )
        .await;
    };
    let refused = |raw: &[u8]| {
        let frame = ic_wire::Encoding::Frame {
            method: ic_wire::method_id("register_local"),
            direction: ic_wire::Direction::Response,
        };
        assert_eq!(
            ic_wire::decode::<Registration>(raw, frame, sharding::LIMITS).unwrap(),
            Err(ProfileError::Forbidden)
        );
    };
    disable(Some(subject)).await;
    refused(
        &pic.update_call(hub, subject, "register_local", register.to_vec())
            .await
            .unwrap(),
    );
    assert!(held.pending(pic).await.unwrap().is_empty());
    disable(None).await;
    let attempt = pic
        .submit_call(hub, subject, "register_local", register.to_vec())
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
    let token = pending.pop().expect("hub suspended waiting for growth");
    disable(Some(subject)).await;
    held.release(pic, &token).await.unwrap();
    refused(&pic.await_call(attempt).await.unwrap());
    assert!(
        pic.get_stable_memory(hub).await == hub_memory,
        "capacity or route changed"
    );
    assert!(
        pic.get_stable_memory(shard).await == shard_memory,
        "provisioning continued"
    );
    canic_qualification::upgrade_stopped(
        pic,
        root.id,
        root_wasm.to_vec(),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
    disable(None).await;
}
