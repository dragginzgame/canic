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
        max_shards: 1,
        keys_per_shard: 1,
        creation_payer: Default::default(),
        creation_budget: if from_spare {
            0
        } else {
            management::CREATION_CYCLES
        },
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
    let place = async || {
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
    let route = async || -> Option<CanisterId> {
        query_service(pic, user, "route_shard", ("users", root.operator)).await
    };
    assert_eq!(route().await, Some(child.id));
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
    assert_eq!(route().await, Some(child.id));
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
    hub.update::<_, Result<(), String>>(
        "release_route",
        ic_wire::method_id("release_route"),
        &([1u8; 32], &target),
        canic::sharding::LIMITS,
    )
    .await
    .unwrap();
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
    assert_eq!(route().await, Some(child.id));
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
    assert_eq!(route().await, None, "returned shard remains discoverable");
    assert!(matches!(worker().await, Placement::Pending(_)));
    settle(pic).await;
    let Placement::Assigned(Allocation { generation, .. }, worker_id) = worker().await else {
        panic!("installed worker")
    };
    assert_eq!(worker_id, child.id.to_text());
    assert_eq!(route().await, None, "former shard now runs worker code");
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
    assert_eq!(advance(&input).await, Ok(P::Returned));
    assert!(
        matches!(inventory().await.as_slice(),[(original,Asset::Claimed(_,new))] if original==&origin && new.generation==generation)
    );
}
