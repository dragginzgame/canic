//! Both permanent hubs and the placement winners are installed by one Root.
use super::*;
use canic::sharding::Placement;

pub(super) async fn place(
    hub: &Client<'_>,
    pool: Option<&str>,
    key: u8,
) -> Result<Placement, String> {
    hub.update(
        "place",
        ic_wire::method_id("place"),
        &(pool, key),
        canic::sharding::LIMITS,
    )
    .await
}

pub(super) async fn counts(root: &Client<'_>) -> stored::SpareCounts {
    let reply = root
        .pic
        .query_call(
            root.id,
            CanisterId::anonymous(),
            stored::ReadSpareCounts::NAME,
            stored::ReadSpareCounts::encode_request(&()).unwrap(),
        )
        .await
        .unwrap();
    stored::ReadSpareCounts::decode_reply(&reply)
        .unwrap()
        .unwrap()
}

pub(super) async fn approve(root: &Client<'_>, max_children: u8) {
    root.update::<_, Result<(), management::Refusal>>(
        "configure_capacity",
        stored::CONFIGURE_CAPACITY,
        &stored::SubnetCapacity {
            root: root.id.to_text(),
            max_children,
        },
        stored::LIMITS,
    )
    .await
    .unwrap();
}

pub(super) async fn ready_project(
    root: &Client<'_>,
    hub: CanisterId,
    source: &stored::Stage<management::Approval>,
) -> CanisterId {
    let install = canic::topology::Install {
        root: root.id.to_text().parse().unwrap(),
        parent: hub.to_text().parse().unwrap(),
        role: "project".into(),
        application: vec![],
    };
    let grant = stored::InstancePool {
        arguments: management::application::Arguments::new(install.encode().unwrap()).unwrap(),
        root: root.id.to_text(),
        hub: hub.to_text(),
        component: "project".into(),
        max_instances: 1,
        creation_payer: Default::default(),
        creation_budget: management::CREATION_CYCLES,
        expires_at_ns: source.approval.expires_at_ns,
        reference: source.reference.clone(),
    };
    root.update::<_, Result<(), management::Refusal>>(
        "configure_instances",
        stored::CONFIGURE_INSTANCES,
        &grant,
        stored::LIMITS,
    )
    .await
    .unwrap();
    let hub = Client {
        pic: root.pic,
        id: hub,
        operator: root.operator,
    };
    let place = async || self::place(&hub, None, 1).await.unwrap();
    assert!(matches!(place().await, Placement::Pending(_)));
    settle(root.pic).await;
    let Placement::Assigned(_, id) = place().await else {
        panic!("project must be ready")
    };
    CanisterId::from_text(id).unwrap()
}

pub(super) async fn exercise(
    root: &Client<'_>,
    source: &stored::Stage<management::Approval>,
    hub: CanisterId,
    project: CanisterId,
    limit: u8,
    module: &str,
) {
    let args = |parent: CanisterId| {
        management::application::Arguments::new(
            canic::topology::Install {
                role: (if parent == hub {
                    "user_shard"
                } else {
                    "project"
                })
                .into(),
                root: root.id.to_text().parse().unwrap(),
                parent: parent.to_text().parse().unwrap(),
                application: vec![],
            }
            .encode()
            .unwrap(),
        )
        .unwrap()
    };
    let from_reserve = limit == 4 && module == "pool-root.wasm";
    let shard = stored::ShardPool {
        arguments: args(hub),
        root: root.id.to_text(),
        hub: hub.to_text(),
        pool: "users".into(),
        max_shards: 1,
        keys_per_shard: if module == "role-root.wasm" { 3 } else { 1 },
        creation_payer: Default::default(),
        creation_budget: if from_reserve {
            0
        } else {
            management::CREATION_CYCLES
        },
        expires_at_ns: source.approval.expires_at_ns,
        reference: source.reference.clone(),
    };
    let instances = stored::InstancePool {
        arguments: args(project),
        root: root.id.to_text(),
        hub: project.to_text(),
        component: "project".into(),
        max_instances: 2,
        creation_payer: Default::default(),
        creation_budget: if from_reserve {
            0
        } else {
            2 * management::CREATION_CYCLES
        },
        expires_at_ns: source.approval.expires_at_ns,
        reference: source.reference.clone(),
    };
    // A different ready hub or Root cannot substitute for the approved binding.
    for (issuer, parent) in [(root.id, project), (project, hub)] {
        let wrong = canic::topology::Install {
            root: issuer.to_text().parse().unwrap(),
            parent: parent.to_text().parse().unwrap(),
            role: "user_shard".into(),
            application: vec![],
        };
        let mut mismatched = shard.clone();
        mismatched.arguments =
            management::application::Arguments::new(wrong.encode().unwrap()).unwrap();
        assert_eq!(
            root.update::<_, Result<(), management::Refusal>>(
                "configure_shards",
                stored::CONFIGURE_SHARDS,
                &mismatched,
                stored::LIMITS
            )
            .await,
            Err(management::Refusal::Binding)
        );
    }
    let mut truncated = shard.clone();
    let bytes = shard.arguments.as_slice();
    truncated.arguments =
        management::application::Arguments::new(bytes[..bytes.len() - 1].to_vec()).unwrap();
    assert_eq!(
        root.update::<_, Result<(), management::Refusal>>(
            "configure_shards",
            stored::CONFIGURE_SHARDS,
            &truncated,
            stored::LIMITS
        )
        .await,
        Err(management::Refusal::Binding)
    );
    root.update::<_, Result<(), management::Refusal>>(
        "configure_shards",
        stored::CONFIGURE_SHARDS,
        &shard,
        stored::LIMITS,
    )
    .await
    .unwrap();
    root.update::<_, Result<(), management::Refusal>>(
        "configure_instances",
        stored::CONFIGURE_INSTANCES,
        &instances,
        stored::LIMITS,
    )
    .await
    .unwrap();
    let place = async |dedicated: bool, key: u8| {
        let caller = Client {
            pic: root.pic,
            id: if dedicated { project } else { hub },
            operator: root.operator,
        };
        self::place(&caller, (!dedicated).then_some("users"), key)
            .await
            .unwrap()
    };
    let (users, projects) = if limit == 3 {
        tokio::join!(place(false, 1), place(true, 1))
    } else {
        let users = place(false, 1).await;
        settle(root.pic).await;
        if shard.keys_per_shard == 1 {
            assert_eq!(place(false, 2).await, Placement::Full);
        } else {
            assert!(matches!(place(false, 2).await, Placement::Assigned(..)));
        }
        (users, place(true, 1).await)
    };
    let outcomes = [(false, users), (true, projects)];
    assert_eq!(
        outcomes
            .iter()
            .filter(|(_, p)| matches!(p, Placement::Pending(_)))
            .count(),
        usize::from(limit - 2).min(2)
    );
    settle(root.pic).await;
    canic_qualification::upgrade_stopped(
        root.pic,
        root.id,
        std::fs::read(artifacts().join(module)).unwrap(),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
    for (dedicated, outcome) in outcomes {
        match outcome {
            Placement::Pending(original) => {
                let Placement::Assigned(after, id) = place(dedicated, 1).await else {
                    panic!("original placement must survive reopen")
                };
                assert_eq!(after, original);
                // A child receives the directory without appearing among the named hubs.
                let parent = if dedicated { project } else { hub };
                let child = CanisterId::from_text(id).unwrap();
                let status: (String, Option<Option<u64>>) =
                    query_service(root.pic, parent, "topology_status", child).await;
                assert_eq!(status.1, Some(Some(2)));
                let children: Vec<CanisterId> =
                    query_service(root.pic, parent, "pool_children", "users").await;
                assert_eq!(
                    children,
                    (!dedicated)
                        .then_some(child)
                        .into_iter()
                        .collect::<Vec<_>>()
                );
                assert_eq!(
                    resolve(root.pic, child).await,
                    Some((2, vec![hub, project]))
                );
            }
            Placement::Full => assert_eq!(place(dedicated, 1).await, Placement::Full),
            other => panic!("unexpected placement {other:?}"),
        }
    }
    if limit == 5 {
        let second = stored::ShardPool {
            pool: "profiles".into(),
            ..shard
        };
        root.update::<_, Result<(), management::Refusal>>(
            "configure_shards",
            stored::CONFIGURE_SHARDS,
            &second,
            stored::LIMITS,
        )
        .await
        .unwrap();
        use canic::sharding::{AssignShard, LookupShard};
        let request = ("profiles".to_string(), [1u8; 32]);
        let assign = AssignShard::encode_request(&request).unwrap();
        assert!(
            root.pic
                .update_call(root.id, project, AssignShard::NAME, assign.clone())
                .await
                .is_err()
        );
        let bytes = root
            .pic
            .update_call(root.id, hub, AssignShard::NAME, assign)
            .await
            .unwrap();
        let Placement::Pending(new) = AssignShard::decode_reply(&bytes).unwrap() else {
            panic!("second pool should reserve its own shard")
        };
        settle(root.pic).await;
        canic_qualification::upgrade_stopped(
            root.pic,
            root.id,
            std::fs::read(artifacts().join(module)).unwrap(),
            vec![],
            Some(root.operator),
        )
        .await
        .unwrap();
        let bytes = root
            .pic
            .update_call(
                root.id,
                hub,
                LookupShard::NAME,
                LookupShard::encode_request(&request).unwrap(),
            )
            .await
            .unwrap();
        let Placement::Assigned(reopened, id) = LookupShard::decode_reply(&bytes).unwrap() else {
            panic!("missing second pool assignment")
        };
        assert_eq!(new, reopened);
        let Placement::Assigned(first, other) = place(false, 1).await else {
            panic!("missing first pool assignment")
        };
        assert_ne!(first, reopened);
        assert_ne!(id, other);
    }
    assert_eq!(place(true, 2).await, Placement::Full);
}

pub(super) async fn separate_hub_pools(
    root: &Client<'_>,
    source: &stored::Stage<management::Approval>,
    users: CanisterId,
    discovery: CanisterId,
) {
    use canic::sharding::LookupShard;
    let mut children = std::collections::BTreeSet::new();
    for (pool, parent, other) in [
        ("user_shard", users, discovery),
        ("discovery_shard", discovery, users),
    ] {
        let install = canic::topology::Install {
            root: root.id.to_text().parse().unwrap(),
            parent: parent.to_text().parse().unwrap(),
            role: pool.into(),
            application: vec![],
        };
        let grant = stored::ShardPool {
            root: root.id.to_text(),
            hub: parent.to_text(),
            pool: pool.into(),
            arguments: management::application::Arguments::new(install.encode().unwrap()).unwrap(),
            keys_per_shard: 1,
            max_shards: 1,
            creation_payer: Default::default(),
            creation_budget: management::CREATION_CYCLES,
            expires_at_ns: source.approval.expires_at_ns,
            reference: source.reference.clone(),
        };
        root.update::<_, Result<(), management::Refusal>>(
            "configure_shards",
            stored::CONFIGURE_SHARDS,
            &grant,
            stored::LIMITS,
        )
        .await
        .unwrap();
        let caller = Client {
            pic: root.pic,
            id: parent,
            operator: root.operator,
        };
        let assign = async || self::place(&caller, Some(pool), 42).await;
        root.pic
            .stop_canister(root.id, Some(root.operator))
            .await
            .unwrap();
        assert_eq!(
            assign().await,
            Err("placement reply unavailable".to_owned())
        );
        root.pic
            .start_canister(root.id, Some(root.operator))
            .await
            .unwrap();
        let reply = root
            .pic
            .update_call(
                root.id,
                parent,
                LookupShard::NAME,
                LookupShard::encode_request(&(pool.to_owned(), [42; 32])).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            LookupShard::decode_reply(&reply).unwrap(),
            Placement::Unknown
        );
        let Placement::Pending(original) = assign().await.unwrap() else {
            panic!("expected first allocation")
        };
        settle(root.pic).await;
        let Placement::Assigned(allocation, id) = assign().await.unwrap() else {
            panic!("ready named shard")
        };
        assert_eq!(allocation, original);
        let id = CanisterId::from_text(id).unwrap();
        assert!(children.insert(id));
        assert_eq!(root.pic.get_controllers(parent).await, [root.id]);
        assert_eq!(root.pic.get_controllers(id).await, [root.id]);
        let members = async |hub| -> Vec<CanisterId> {
            query_service(root.pic, hub, "pool_children", pool).await
        };
        assert_eq!(members(parent).await, [id]);
        assert!(members(other).await.is_empty());
        root.pic
            .stop_canister(root.id, Some(root.operator))
            .await
            .unwrap();
        assert_eq!(members(parent).await, [id]);
        assert!(members(other).await.is_empty());
        root.pic
            .start_canister(root.id, Some(root.operator))
            .await
            .unwrap();
    }
}
