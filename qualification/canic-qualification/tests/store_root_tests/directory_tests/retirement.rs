//! Same-principal handover from a proof-bearing shared shard to a worker.
use super::*;
use canic::sharding::{
    Allocation, AssignWorker, BeginShardDrain, Placement, ShardTarget,
    standby::{Asset, InspectStandby, Source},
};
use canic::topology::attestation::{CloseRoleIssuance, Proof};
use canic_management_protocol::stored::RetirementPhase as P;

pub(super) async fn exercise(
    root: &Client<'_>,
    user: CanisterId,
    pool: CanisterId,
    source: &stored::Stage<management::Approval>,
    dormant: store::Manifest,
    miner: store::Manifest,
    from_spare: bool,
) {
    let pic = root.pic;
    let args = |parent: CanisterId, role: &str, application: Vec<u8>| {
        management::application::Arguments::new(
            canic::topology::Install {
                root: root.id.to_text().parse().unwrap(),
                parent: parent.to_text().parse().unwrap(),
                role: role.into(),
                application,
            }
            .encode()
            .unwrap(),
        )
        .unwrap()
    };
    let reference = |manifest| stored::Reference {
        store: source.reference.store.clone(),
        manifest,
    };
    let hub = Client {
        pic,
        id: user,
        operator: root.operator,
    };
    let refill = canic::sharding::standby::Refill {
        root: root.id.to_text().parse().unwrap(),
        sequence: 0,
        cycles: management::CREATION_CYCLES,
        expires_at_ns: source.approval.expires_at_ns,
    };
    if from_spare {
        use canic::sharding::standby::RefillStandby;
        let asset = root
            .update::<_, Result<Asset, String>>(
                RefillStandby::NAME,
                RefillStandby::ID,
                &refill,
                RefillStandby::REPLY,
            )
            .await
            .unwrap();
        assert!(matches!(asset, Asset::Empty(_)));
    }
    let shard_pool = stored::ShardPool {
        arguments: args(user, "user_shard", vec![]),
        root: root.id.to_text(),
        hub: user.to_text(),
        pool: "users".into(),
        max_shards: if from_spare { 2 } else { 1 },
        keys_per_shard: if from_spare { 2 } else { 1 },
        creation_payer: Default::default(),
        creation_budget: management::CREATION_CYCLES,
        expires_at_ns: source.approval.expires_at_ns,
        reference: source.reference.clone(),
    };
    root.update::<_, Result<(), management::Refusal>>(
        "configure_shards",
        stored::CONFIGURE_SHARDS,
        &shard_pool,
        stored::LIMITS,
    )
    .await
    .unwrap();
    let local_place = async |owner| {
        Client {
            pic,
            id: user,
            operator: owner,
        }
        .update::<_, Placement>(
            "local_place",
            ic_wire::method_id("local_place"),
            &(),
            canic::sharding::LIMITS,
        )
        .await
    };
    let alice = CanisterId::self_authenticating(b"alice");
    let place = async || {
        if from_spare {
            return local_place(alice).await;
        }
        super::capacity::place(&hub, Some("users"), 1)
            .await
            .unwrap()
    };
    assert!(matches!(place().await, Placement::Pending(_)));
    settle(pic).await;
    let Placement::Assigned(allocation, id) = place().await else {
        panic!("installed shard")
    };
    let target = ShardTarget {
        allocation: allocation.clone(),
        shard: id.parse().unwrap(),
    };
    let child = Client {
        pic,
        id: CanisterId::from_text(&id).unwrap(),
        operator: root.id,
    };
    let members =
        async || -> Vec<CanisterId> { query_service(pic, user, "pool_children", "users").await };
    assert_eq!(members().await, [child.id]);
    use canic::topology::directory::WithdrawChild;
    let withdrawal = WithdrawChild::encode_request(&(target.shard, allocation.generation)).unwrap();
    let reply = pic
        .update_call(user, root.operator, WithdrawChild::NAME, withdrawal)
        .await
        .unwrap();
    assert_eq!(
        WithdrawChild::decode_reply(&reply).unwrap(),
        Err("unexpected topology sender".into())
    );
    assert_eq!(members().await, [child.id]);
    let expiry = child
        .update::<_, Result<u64, String>>(
            "renew_role",
            ic_wire::method_id("renew_role"),
            &(root.id, user),
            canic::method::LIMITS,
        )
        .await
        .unwrap();
    let frame = |direction| ic_wire::Encoding::Frame {
        method: ic_wire::method_id("retained_role"),
        direction,
    };
    let proof: Option<Proof> = ic_wire::decode(
        &pic.query_call(
            child.id,
            root.id,
            "retained_role",
            ic_wire::encode(
                &(),
                frame(ic_wire::Direction::Request),
                canic::method::LIMITS,
            )
            .unwrap(),
        )
        .await
        .unwrap(),
        frame(ic_wire::Direction::Response),
        canic::method::LIMITS,
    )
    .unwrap();
    let proof = proof.unwrap();
    let use_proof = async || {
        Client {
            pic,
            id: user,
            operator: child.id,
        }
        .update::<_, Result<(), String>>(
            "use_role",
            ic_wire::method_id("use_role"),
            &proof,
            canic::method::LIMITS,
        )
        .await
    };
    assert_eq!(
        root.update::<_, Result<u32, String>>(
            BeginShardDrain::NAME,
            BeginShardDrain::ID,
            &target,
            canic::sharding::LIMITS
        )
        .await,
        Ok(1)
    );
    if from_spare {
        assert_eq!(
            place().await.assigned_target().unwrap().unwrap().1,
            target.shard
        );
        let writer = Client {
            pic,
            id: child.id,
            operator: alice,
        };
        let write = async |value: u64| {
            writer
                .update::<_, Result<(), String>>(
                    "write_value",
                    ic_wire::method_id("write_value"),
                    &value,
                    canic::method::LIMITS,
                )
                .await
        };
        write(42).await.unwrap();
        assert!(
            root.update::<_, Result<u64, String>>(
                CloseRoleIssuance::NAME,
                CloseRoleIssuance::ID,
                &(child.id, proof.0.generation.clone()),
                canic::method::LIMITS
            )
            .await
            .is_err()
        );
        let bob = CanisterId::self_authenticating(b"bob");
        assert!(matches!(local_place(bob).await, Placement::Pending(_)));
        settle(pic).await;
        let bob_route = local_place(bob).await;
        let (allocation, shard) = bob_route.assigned_target().unwrap().unwrap();
        assert_ne!(shard, target.shard);
        let destination = ShardTarget {
            allocation: allocation.clone(),
            shard,
        };
        let arguments = (target.clone(), destination);
        let shard_id = CanisterId::from_slice(shard.as_slice());
        let name = "move_account";
        let frame = ic_wire::Encoding::Frame {
            method: ic_wire::method_id(name),
            direction: ic_wire::Direction::Request,
        };
        let _lost = pic
            .submit_call(
                user,
                alice,
                name,
                ic_wire::encode(&arguments, frame, canic::sharding::LIMITS).unwrap(),
            )
            .await
            .unwrap();
        settle(pic).await;
        assert_eq!(place().await, bob_route);
        let read_value = async || {
            let name = "read_value";
            let frame = |direction| ic_wire::Encoding::Frame {
                method: ic_wire::method_id(name),
                direction,
            };
            let input = ic_wire::encode(
                &(),
                frame(ic_wire::Direction::Request),
                canic::method::LIMITS,
            )
            .unwrap();
            let output = pic.query_call(shard_id, alice, name, input).await.unwrap();
            ic_wire::decode::<Option<u64>>(
                &output,
                frame(ic_wire::Direction::Response),
                canic::method::LIMITS,
            )
            .unwrap()
        };
        assert_eq!(read_value().await, Some(42));
        let destination_writer = Client {
            pic,
            id: shard_id,
            operator: alice,
        };
        destination_writer
            .update::<_, Result<(), String>>(
                "write_value",
                ic_wire::method_id("write_value"),
                &43_u64,
                canic::method::LIMITS,
            )
            .await
            .unwrap();
        let mover = Client {
            pic,
            id: user,
            operator: alice,
        };
        mover
            .update::<_, Result<(), String>>(
                name,
                ic_wire::method_id(name),
                &arguments,
                canic::sharding::LIMITS,
            )
            .await
            .unwrap();
        assert_eq!(
            read_value().await,
            Some(43),
            "retry preserved the later destination edit"
        );
        assert_eq!(write(43).await, Err("account fenced".into()));
        assert_eq!(place().await, bob_route);
        assert_eq!(
            root.update::<_, Result<u32, String>>(
                BeginShardDrain::NAME,
                BeginShardDrain::ID,
                &target,
                canic::sharding::LIMITS
            )
            .await,
            Ok(0)
        );
    } else {
        Client {
            operator: root.id,
            ..hub
        }
        .update::<_, Result<(), String>>(
            "release_route",
            ic_wire::method_id("release_route"),
            &([1u8; 32], &target),
            canic::sharding::LIMITS,
        )
        .await
        .unwrap();
    }
    assert_eq!(
        root.update::<_, Result<u32, String>>(
            BeginShardDrain::NAME,
            BeginShardDrain::ID,
            &target,
            canic::sharding::LIMITS
        )
        .await,
        Ok(0)
    );
    let deadline = root
        .update::<_, Result<u64, String>>(
            CloseRoleIssuance::NAME,
            CloseRoleIssuance::ID,
            &(child.id, proof.0.generation.clone()),
            canic::method::LIMITS,
        )
        .await
        .unwrap();
    assert!(deadline >= expiry);
    let expires = deadline + 300_000_000_000;
    let input = stored::Stage {
        reference: reference(dormant),
        approval: stored::ShardRetirement {
            root: root.id.to_text(),
            shard: id,
            slot: allocation.slot,
            generation: allocation.generation,
            minimum_reuse_cycles: 1_000_000_u64.try_into().unwrap(),
            expires_at_ns: expires,
        },
    };
    let advance = async |approval: &stored::Stage<stored::ShardRetirement>| {
        root.update::<_, Result<P, String>>(
            "retire_shard",
            ic_wire::method_id("retire_shard"),
            approval,
            stored::LIMITS,
        )
        .await
    };
    let reopen = async |phase: &str| {
        canic_qualification::upgrade_stopped(
            pic,
            root.id,
            std::fs::read(artifacts().join("role-root.wasm")).unwrap(),
            vec![],
            Some(root.operator),
        )
        .await
        .unwrap_or_else(|error| panic!("Root reopen at {phase}: {error:?}"))
    };
    let inventory = async || {
        root.update::<_, <InspectStandby as Contract>::Output>(
            InspectStandby::NAME,
            InspectStandby::ID,
            &(),
            InspectStandby::REPLY,
        )
        .await
    };
    let codec = artifacts().join("role-root.client");
    let command = [
        "retire-shard".into(),
        serde_json::to_string(&input).unwrap(),
    ];
    let request = canic_host::application::codec::encode(&codec, &command).unwrap();
    assert!(
        pic.update_call(root.id, user, &request.method, request.arguments.clone())
            .await
            .is_err()
    );
    let reply = pic
        .update_call(root.id, root.operator, &request.method, request.arguments)
        .await
        .unwrap();
    assert_eq!(
        canic_host::application::codec::decode(&codec, &reply).unwrap()["retirement"],
        "Pinned"
    );
    let mut changed = input.clone();
    changed.approval.minimum_reuse_cycles = 1u64.try_into().unwrap();
    assert_eq!(
        advance(&changed).await.unwrap_err(),
        "retirement approval differs"
    );
    for phase in [P::CacheIssued, P::Cached] {
        assert_eq!(advance(&input).await, Ok(phase));
    }
    pic.stop_canister(user, Some(root.id)).await.unwrap();
    assert!(
        advance(&input)
            .await
            .unwrap_err()
            .contains("hub withdrawal unconfirmed")
    );
    pic.start_canister(user, Some(root.id)).await.unwrap();
    assert!(members().await.contains(&child.id));
    for phase in [P::InstallIssued, P::Dormant] {
        assert_eq!(advance(&input).await, Ok(phase));
    }
    reopen("dormant").await;
    assert_eq!(use_proof().await, Ok(()));
    let origin = if from_spare {
        assert!(
            matches!(inventory().await.as_slice(),[(_,Asset::Claimed(id,a))] if *id==target.shard && *a==target.allocation)
        );
        Source::Refill(refill)
    } else {
        assert!(inventory().await.is_empty());
        Source::Shard(target.clone())
    };
    assert_eq!(
        advance(&input).await.unwrap_err(),
        "original role proof window remains open"
    );
    // Exercise both independent hubs and co-owned worker and shard grants.
    let pool = if from_spare { user } else { pool };
    let worker_pool = stored::InstancePool {
        root: root.id.to_text(),
        hub: pool.to_text(),
        component: "miner".into(),
        max_instances: 1,
        creation_payer: Default::default(),
        creation_budget: 0,
        expires_at_ns: expires,
        reference: reference(miner),
        arguments: args(pool, "miner", canic::topology::WORKER_GENERATION.to_vec()),
    };
    root.update::<_, Result<(), management::Refusal>>(
        "configure_workers",
        stored::CONFIGURE_WORKERS,
        &worker_pool,
        stored::LIMITS,
    )
    .await
    .unwrap();
    if !from_spare {
        let auxiliary = stored::InstancePool {
            hub: user.to_text(),
            component: "auxiliary".into(),
            arguments: args(
                user,
                "auxiliary",
                canic::topology::WORKER_GENERATION.to_vec(),
            ),
            ..worker_pool.clone()
        };
        root.update::<_, Result<(), management::Refusal>>(
            "configure_workers",
            stored::CONFIGURE_WORKERS,
            &auxiliary,
            stored::LIMITS,
        )
        .await
        .unwrap();
        let input = ("miner".to_string(), 0u8);
        use canic::sharding::LookupWorker;
        for (method, request) in [
            (
                AssignWorker::NAME,
                AssignWorker::encode_request(&input).unwrap(),
            ),
            (
                LookupWorker::NAME,
                LookupWorker::encode_request(&input).unwrap(),
            ),
        ] {
            assert!(
                pic.update_call(root.id, user, method, request)
                    .await
                    .is_err(),
                "{method} admitted another pool's hub"
            );
        }
    }
    let worker = async || {
        Client {
            pic,
            id: root.id,
            operator: pool,
        }
        .update::<_, Placement>(
            AssignWorker::NAME,
            AssignWorker::ID,
            &("miner".to_string(), 0u8),
            canic::sharding::LIMITS,
        )
        .await
    };
    assert_eq!(worker().await, Placement::Full);
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    pic.advance_time(Duration::from_nanos(deadline - now)).await;
    for phase in [
        P::StopIssued,
        P::UninstallIssued,
        P::Uninstalled,
        P::StartIssued,
        P::Returned,
    ] {
        assert_eq!(advance(&input).await, Ok(phase));
    }
    assert!(
        matches!(inventory().await.as_slice(), [(original,Asset::Available(id,_))] if original==&origin && *id==target.shard)
    );
    reopen("available").await;
    assert!(
        !members().await.contains(&child.id),
        "returned shard remains discoverable"
    );
    assert!(matches!(worker().await, Placement::Pending(_)));
    settle(pic).await;
    let Placement::Assigned(Allocation { generation, .. }, worker_id) = worker().await else {
        panic!("installed worker")
    };
    assert_eq!(worker_id, child.id.to_text());
    assert!(
        !members().await.contains(&child.id),
        "former shard now runs worker code"
    );
    assert_ne!(generation, allocation.generation);
    let parent = Client {
        pic,
        id: child.id,
        operator: pool,
    };
    let (attempt, _): (u64, bool) = parent
        .update(
            "mine",
            ic_wire::method_id("mine"),
            &(
                ic_wire::Bytes::<32>::new(generation.to_vec()).unwrap(),
                1u64,
                1u8,
            ),
            canic::method::LIMITS,
        )
        .await;
    assert_eq!(attempt, 1);
    assert_eq!(use_proof().await, Err("role refused".into()));
    reopen("reassigned").await;
    if from_spare {
        use canic::sharding::GrowShard;
        let original = Client {
            pic,
            id: root.id,
            operator: user,
        }
        .update::<_, (Placement, u32)>(
            GrowShard::NAME,
            GrowShard::ID,
            &("users", 0_u8),
            GrowShard::REPLY,
        )
        .await;
        assert_eq!(
            original.0,
            Placement::Unavailable(target.allocation.clone(), Some(target.shard.to_text()))
        );
    }
    assert_eq!(advance(&input).await, Ok(P::Returned));
    assert!(
        matches!(inventory().await.as_slice(),[(original,Asset::Claimed(_,new))] if original==&origin && new.generation==generation)
    );
}
