//! Both permanent hubs and the placement winners are installed by one Root.
use super::*;
use canic::sharding::Placement;

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
    let place = async || {
        hub.update::<_, Placement>(
            "place",
            ic_wire::method_id("place"),
            &(true, 1u8),
            canic::sharding::LIMITS,
        )
        .await
    };
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
    let shard = stored::ShardPool {
        arguments: args(hub),
        root: root.id.to_text(),
        hub: hub.to_text(),
        pool: "users".into(),
        max_shards: 1,
        keys_per_shard: 1,
        creation_budget: management::CREATION_CYCLES,
        expires_at_ns: source.approval.expires_at_ns,
        reference: source.reference.clone(),
    };
    let instances = stored::InstancePool {
        arguments: args(project),
        root: root.id.to_text(),
        hub: project.to_text(),
        component: "project".into(),
        max_instances: 2,
        creation_budget: 2 * management::CREATION_CYCLES,
        expires_at_ns: source.approval.expires_at_ns,
        reference: source.reference.clone(),
    };
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
        Client {
            pic: root.pic,
            id: if dedicated { project } else { hub },
            operator: root.operator,
        }
        .update::<_, Placement>(
            "place",
            ic_wire::method_id("place"),
            &(dedicated, key),
            canic::sharding::LIMITS,
        )
        .await
    };
    let (users, projects) = if limit == 3 {
        tokio::join!(place(false, 1), place(true, 1))
    } else {
        let users = place(false, 1).await;
        settle(root.pic).await;
        assert_eq!(place(false, 2).await, Placement::Full);
        (users, place(true, 1).await)
    };
    let outcomes = [(false, users), (true, projects)];
    assert_eq!(
        outcomes
            .iter()
            .filter(|(_, p)| matches!(p, Placement::Pending(_)))
            .count(),
        usize::from(limit - 2)
    );
    settle(root.pic).await;
    canic_qualification::upgrade_stopped(
        root.pic,
        root.id,
        std::fs::read(artifacts().join("pool-root.wasm")).unwrap(),
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
                assert_eq!(
                    resolve(root.pic, child).await,
                    Some((2, vec![hub, project]))
                );
            }
            Placement::Full => assert_eq!(place(dedicated, 1).await, Placement::Full),
            other => panic!("unexpected placement {other:?}"),
        }
    }
    assert_eq!(place(true, 2).await, Placement::Full);
}
