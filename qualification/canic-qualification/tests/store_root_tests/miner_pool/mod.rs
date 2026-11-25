//! The actual pool calls Root for worker custody and its miners for each batch.
use super::*;
use canic::method::Contract;
use canic::sharding::standby::ReserveError;
use canic_host::application::codec;
use ex_miner_pool_hub::{PoolReport, Tick};

#[tokio::test(flavor = "current_thread")]
async fn growth_park_reuse_and_capacity() {
    let base =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/examples/miner-pool/artifacts");
    let read = |name: &str| std::fs::read(base.join(name)).expect("make miner-test");
    let (mut server, mut owner) = support::instance_with_lifetime(
        &canic_qualification::server_directory(),
        ic_testkit::pocket_ic::PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet(),
        Duration::from_secs(180),
    )
    .await;
    let crate::miner_pool_fixture::Fleet {
        root,
        pool,
        capacity,
        workers,
        ..
    } = crate::miner_pool_fixture::install(&mut owner, &base, |fleet| {
        fleet.worker_pools.first_mut().unwrap().creation_budget = 4 * management::CREATION_CYCLES
    })
    .await;
    let pic = root.pic;
    let tick = |round, goal| Tick::encode_request(&(round, goal)).unwrap();
    // The hub is installed but Root has not approved its grant yet.
    let refused = pic
        .update_call(pool, root.operator, Tick::NAME, tick(1, 2))
        .await
        .unwrap();
    assert_eq!(
        Tick::decode_reply(&refused).unwrap(),
        "worker 0: Root has no approved worker grant"
    );
    configure(&root, &capacity, &workers).await;
    let rejection = canic_qualification::upgrade_stopped(
        pic,
        root.id,
        std::fs::read(artifacts().join("management-root.wasm")).unwrap(),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap_err();
    assert!(
        rejection
            .reject_message
            .contains("retained Root approval requires child-creation"),
        "{rejection:?}"
    );
    let mut changed = workers.approval.clone();
    let mut topology = canic::topology::Install::decode(changed.arguments.as_slice()).unwrap();
    let floor = canic::topology::worker::minimum_reuse_cycles(&topology.application).unwrap();
    assert_eq!(floor.get(), 500_000_000_000);
    topology.application =
        canic::topology::worker::reuse_policy(floor.checked_add(1).unwrap()).unwrap();
    changed.arguments =
        management::application::Arguments::new(topology.encode().unwrap()).unwrap();
    let mut fewer_workers = workers.approval.clone();
    fewer_workers.max_instances -= 1;
    for changed in [changed, fewer_workers] {
        let conflict: Result<(), management::Refusal> = root
            .update(
                "approve_child_creation",
                stored::APPROVE_CHILD_CREATION,
                &stored::ChildCreationGrant::Worker(changed.clone()),
                stored::LIMITS,
            )
            .await;
        assert_eq!(conflict, Err(management::Refusal::Occupied));
    }
    let outsider = CanisterId::self_authenticating(b"other hub");
    crate::store_root_tests::assert_unauthorized(
        &pic.update_call(
            root.id,
            outsider,
            canic::sharding::payment::AcquirePlacement::NAME,
            crate::store_root_tests::placement_request(canic::sharding::payment::Demand::Worker(
                "miner".into(),
                0,
            )),
        )
        .await
        .unwrap(),
    );
    assert!(
        pic.update_call(
            pool,
            outsider,
            "tick",
            Tick::encode_request(&(1, 2)).unwrap()
        )
        .await
        .is_err()
    );
    use canic::sharding::standby::{Asset, ObserveStandby, Refill, RefillStandby};
    let refill = Refill {
        root: root.id.as_slice().try_into().unwrap(),
        sequence: 0,
        cycles: management::CREATION_CYCLES,
        expires_at_ns: pic.get_time().await.as_nanos_since_unix_epoch() + 300_000_000_000,
    };
    let request = RefillStandby::encode_request(&refill).unwrap();
    assert!(
        pic.update_call(root.id, outsider, RefillStandby::NAME, request.clone())
            .await
            .is_err()
    );
    let refill_once = async || {
        RefillStandby::decode_reply(
            &pic.update_call(root.id, root.operator, RefillStandby::NAME, request.clone())
                .await
                .unwrap(),
        )
        .unwrap()
        .unwrap()
    };
    let Asset::Empty(spare) = refill_once().await else {
        panic!("expected a new empty canister")
    };
    assert_eq!(refill_once().await, Asset::Empty(spare));
    let mut changed = refill.clone();
    changed.expires_at_ns -= 1;
    let reply = pic
        .update_call(
            root.id,
            root.operator,
            RefillStandby::NAME,
            RefillStandby::encode_request(&changed).unwrap(),
        )
        .await
        .unwrap();
    assert!(matches!(
        RefillStandby::decode_reply(&reply).unwrap().unwrap_err(),
        ReserveError::ApprovalConflict(_)
    ));
    let status = pic
        .canister_status(CanisterId::from_slice(spare.as_slice()), Some(root.id))
        .await
        .unwrap();
    assert!(status.module_hash.is_none());
    assert_eq!(status.settings.controllers, [root.id]);
    canic_qualification::upgrade_stopped(
        pic,
        root.id,
        read("root.wasm"),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
    let inventory = async || {
        ObserveStandby::decode_reply(
            &pic.update_call(
                root.id,
                root.operator,
                ObserveStandby::NAME,
                ObserveStandby::encode_request(&()).unwrap(),
            )
            .await
            .unwrap(),
        )
        .unwrap()
    };
    assert_eq!(
        inventory().await,
        vec![(
            canic::sharding::standby::Source::Refill(refill.clone()),
            Asset::Empty(spare)
        )]
    );
    let client = codec::output(&base.join("pool.wasm"));
    assert!(codec::encode(&client, &["tick".into(), "0".into(), "5".into()]).is_err());
    let mut originals = Vec::new();
    for (round, goal) in [(1u64, 2u8), (2, 5), (3, 1), (4, 5), (5, 10), (6, 0)] {
        let request = codec::encode(
            &client,
            &["tick".into(), round.to_string(), goal.to_string()],
        )
        .unwrap();
        let mut settled = false;
        let mut pending = false;
        let mut last = String::new();
        for _ in 0..100 {
            let bytes = pic
                .update_call(
                    pool,
                    root.operator,
                    &request.method,
                    request.arguments.clone(),
                )
                .await
                .unwrap();
            let decoded = codec::decode(&client, &bytes).unwrap();
            let result = decoded.as_str().unwrap();
            pending |= result == "worker 0: Root allocation pending";
            result.clone_into(&mut last);
            if result
                == if goal > 8 {
                    "capacity limit"
                } else {
                    "settled"
                }
            {
                settled = true;
                break;
            }
            assert_ne!(result, "growth refused");
            pic.advance_time(Duration::from_secs(1)).await;
            pic.tick().await;
        }
        if round == 1 {
            assert!(
                pending,
                "Root must admit the original ordinal after approval"
            );
        }
        assert!(settled, "round {round}: {last}");
        let bytes = pic
            .query_call(
                pool,
                outsider,
                PoolReport::NAME,
                PoolReport::encode_request(&()).unwrap(),
            )
            .await
            .unwrap();
        let report = PoolReport::decode_reply(&bytes).unwrap();
        assert_eq!(
            (report.round, report.goal, report.completed),
            (round, goal, goal.min(8))
        );
        assert_eq!(
            report
                .workers
                .iter()
                .filter(|w| w.status == "active")
                .count(),
            usize::from(goal.min(8).div_ceil(2))
        );
        let ids: Vec<_> = report.workers.iter().map(|w| w.principal).collect();
        assert_eq!(ids[0], spare);
        assert!(matches!(inventory().await.as_slice(), [(_, Asset::Claimed(id,_))] if *id==spare));
        assert!(ids.starts_with(&originals));
        originals = ids;
        println!(
            "Round {round}, goal {goal}, attempts {}, miners {}",
            report.completed,
            report.workers.len()
        );
        for worker in &report.workers {
            println!(
                "  {}  {:8}  {} blocks",
                worker.principal, worker.status, worker.blocks
            );
        }
        if round == 2 {
            canic_qualification::upgrade_stopped(
                pic,
                pool,
                read("pool.wasm"),
                vec![],
                Some(root.id),
            )
            .await
            .unwrap();
            canic_qualification::upgrade_stopped(
                pic,
                root.id,
                read("root.wasm"),
                vec![],
                Some(root.operator),
            )
            .await
            .unwrap();
        }
        if round == 6 {
            let finds = report.workers.iter().map(|w| w.blocks).sum::<u64>();
            assert!((1..21).contains(&finds), "expected both finds and misses");
            assert_eq!(report.workers.iter().map(|w| w.attempts).sum::<u64>(), 21);
        }
    }
    // Exercise Root’s own backstop even though the example tick already caps demand.
    let bytes = pic
        .update_call(
            root.id,
            pool,
            canic::sharding::payment::AcquirePlacement::NAME,
            crate::store_root_tests::placement_request(canic::sharding::payment::Demand::Worker(
                "miner".into(),
                4,
            )),
        )
        .await
        .unwrap();
    assert_eq!(
        crate::store_root_tests::placed(&bytes),
        canic::sharding::Placement::Full
    );
    // Lower demand while the worker is stopped. The reopened hub must drain the original batch.
    let worker = CanisterId::from_slice(originals[0].as_slice());
    pic.stop_canister(worker, Some(root.id)).await.unwrap();
    let refused = pic
        .update_call(pool, root.operator, Tick::NAME, tick(7, 1))
        .await
        .unwrap();
    assert_eq!(
        Tick::decode_reply(&refused).unwrap(),
        "worker 0, batch 7/1: call rejected"
    );
    canic_qualification::upgrade_stopped(pic, pool, read("pool.wasm"), vec![], Some(root.id))
        .await
        .unwrap();
    let refused = pic
        .update_call(pool, root.operator, Tick::NAME, tick(8, 0))
        .await
        .unwrap();
    assert_eq!(
        Tick::decode_reply(&refused).unwrap(),
        "worker 0, batch 7/1: call rejected"
    );
    let draining = pic
        .query_call(
            pool,
            outsider,
            PoolReport::NAME,
            PoolReport::encode_request(&()).unwrap(),
        )
        .await
        .unwrap();
    let draining = PoolReport::decode_reply(&draining).unwrap();
    assert_eq!(
        (draining.round, draining.goal, draining.completed),
        (8, 0, 0)
    );
    assert_eq!(draining.workers[0].status, "draining");
    assert_eq!(draining.workers.iter().map(|w| w.attempts).sum::<u64>(), 21);
    pic.start_canister(worker, Some(root.id)).await.unwrap();
    pic.submit_call(pool, root.operator, Tick::NAME, tick(8, 0))
        .await
        .unwrap();
    pic.tick().await;
    for _ in 0..2 {
        pic.update_call(pool, root.operator, "tick", tick(8, 0))
            .await
            .unwrap();
    }
    let reply = pic
        .query_call(
            pool,
            outsider,
            PoolReport::NAME,
            PoolReport::encode_request(&()).unwrap(),
        )
        .await
        .unwrap();
    let parked = PoolReport::decode_reply(&reply).unwrap();
    assert_eq!((parked.round, parked.goal, parked.completed), (8, 0, 0));
    assert!(parked.workers.iter().all(|w| w.status == "parked"));
    assert_eq!(parked.workers.iter().map(|w| w.attempts).sum::<u64>(), 22);
    use canic::sharding::{ShardTarget, standby::WorkerReleased};
    use ex_miner_pool_hub::Release;
    let last = parked.workers.last().unwrap();
    let target = ShardTarget {
        shard: last.principal,
        allocation: last.allocation.clone(),
    };
    let request = Release::encode_request(&target).unwrap();
    assert!(
        pic.update_call(pool, outsider, Release::NAME, request.clone())
            .await
            .is_err()
    );
    for _ in 0..2 {
        let bytes = pic
            .update_call(pool, root.operator, Release::NAME, request.clone())
            .await
            .unwrap();
        Release::decode_reply(&bytes).unwrap().unwrap();
    }
    canic_qualification::upgrade_stopped(pic, pool, read("pool.wasm"), vec![], Some(root.id))
        .await
        .unwrap();
    let released = |target: &ShardTarget| WorkerReleased::encode_request(target).unwrap();
    let bytes = pic
        .query_call(pool, root.id, WorkerReleased::NAME, released(&target))
        .await
        .unwrap();
    assert!(WorkerReleased::decode_reply(&bytes).unwrap());
    let mut stale = target;
    stale.allocation.generation[0] ^= 1;
    let bytes = pic
        .query_call(pool, root.id, WorkerReleased::NAME, released(&stale))
        .await
        .unwrap();
    assert!(!WorkerReleased::decode_reply(&bytes).unwrap());
    use canic::sharding::standby::ConfirmRelease;
    let spare_target = ShardTarget {
        shard: spare,
        allocation: parked.workers[0].allocation.clone(),
    };
    let confirm = async || {
        ConfirmRelease::decode_reply(
            &pic.update_call(
                root.id,
                root.operator,
                ConfirmRelease::NAME,
                ConfirmRelease::encode_request(&spare_target).unwrap(),
            )
            .await
            .unwrap(),
        )
        .unwrap()
    };
    assert!(confirm().await.is_err());
    for w in parked.workers.iter().rev() {
        let target = ShardTarget {
            shard: w.principal,
            allocation: w.allocation.clone(),
        };
        let reply = pic
            .update_call(
                pool,
                root.operator,
                Release::NAME,
                Release::encode_request(&target).unwrap(),
            )
            .await
            .unwrap();
        Release::decode_reply(&reply).unwrap().unwrap();
    }
    reserve::check_controller_handoff(&root, pool, &spare_target, &read("pool.wasm")).await;
    assert!(matches!(&inventory().await[..], [(_, Asset::Claimed(..))]));
    confirm().await.unwrap();
    confirm().await.unwrap();
    canic_qualification::upgrade_stopped(
        pic,
        root.id,
        read("root.wasm"),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
    assert_eq!(
        inventory().await,
        vec![(
            canic::sharding::standby::Source::Refill(refill.clone()),
            Asset::Released(spare, spare_target.allocation.clone())
        )]
    );
    let response = pic
        .update_call(
            root.id,
            pool,
            canic::sharding::LookupPlacement::NAME,
            canic::sharding::LookupPlacement::encode_request(&canic::sharding::Lookup::Worker(
                "miner".into(),
                0,
            ))
            .unwrap(),
        )
        .await
        .unwrap();
    assert!(matches!(
        canic::sharding::LookupPlacement::decode_reply(&response).unwrap(),
        canic::sharding::Placement::Unavailable(..)
    ));
    use canic::sharding::standby::{ResetPhase, ResetWorker};
    let deadline = pic.get_time().await.as_nanos_since_unix_epoch() + 300_000_000_000;
    let reset = async |target: &ShardTarget, deadline| {
        ResetWorker::decode_reply(
            &pic.update_call(
                root.id,
                root.operator,
                ResetWorker::NAME,
                ResetWorker::encode_request(&(target.clone(), deadline)).unwrap(),
            )
            .await
            .unwrap(),
        )
        .unwrap()
    };
    let mut wrong = spare_target.clone();
    wrong.allocation.generation[0] ^= 1;
    assert!(reset(&wrong, deadline).await.is_err());
    assert!(
        pic.update_call(
            root.id,
            outsider,
            ResetWorker::NAME,
            ResetWorker::encode_request(&(spare_target.clone(), deadline)).unwrap()
        )
        .await
        .is_err()
    );
    let id = CanisterId::from_slice(spare.as_slice());
    assert!(reset(&spare_target, 0).await.is_err());
    pic.set_controllers(id, Some(root.id), vec![root.id, outsider])
        .await
        .unwrap();
    assert!(matches!(
        reset(&spare_target, deadline).await.unwrap_err(),
        ReserveError::Unauthorized(_)
    ));
    pic.set_controllers(id, Some(root.id), vec![root.id])
        .await
        .unwrap();
    pic.stop_canister(id, Some(root.id)).await.unwrap();
    let snapshot = pic
        .take_canister_snapshot(id, Some(root.id), None)
        .await
        .unwrap();
    pic.start_canister(id, Some(root.id)).await.unwrap();
    assert!(matches!(
        reset(&spare_target, deadline).await.unwrap_err(),
        ReserveError::CustodyOccupied(_)
    ));
    assert!(
        pic.canister_status(id, Some(root.id))
            .await
            .unwrap()
            .module_hash
            .is_some()
    );
    pic.delete_canister_snapshot(id, Some(root.id), snapshot.id)
        .await
        .unwrap();
    assert_eq!(
        reset(&spare_target, deadline).await.unwrap(),
        ResetPhase::StopIssued
    );
    assert!(reset(&spare_target, deadline + 1).await.is_err());
    // Interrupt Root's callback after the management canister commits uninstall.
    pic.add_cycles(id, u128::from(floor.get())).await;
    canic_qualification::upgrade_stopped(
        pic,
        root.id,
        std::fs::read(artifacts().join("pool-faults.wasm")).unwrap(),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
    inject(&root, 26).await;
    let lost = pic
        .update_call(
            root.id,
            root.operator,
            ResetWorker::NAME,
            ResetWorker::encode_request(&(spare_target.clone(), deadline)).unwrap(),
        )
        .await
        .unwrap_err();
    assert!(
        lost.reject_message.contains("injected growth boundary"),
        "{lost:?}"
    );
    assert!(
        pic.canister_status(id, Some(root.id))
            .await
            .unwrap()
            .module_hash
            .is_none()
    );
    canic_qualification::upgrade_stopped(
        pic,
        root.id,
        read("root.wasm"),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
    assert_eq!(
        inventory().await,
        vec![(
            canic::sharding::standby::Source::Refill(refill.clone()),
            Asset::Reset(
                spare,
                spare_target.allocation.clone(),
                deadline,
                ResetPhase::UninstallIssued
            )
        )]
    );
    for phase in [
        ResetPhase::Uninstalled,
        ResetPhase::StartIssued,
        ResetPhase::Returned,
        ResetPhase::Returned,
    ] {
        assert_eq!(reset(&spare_target, deadline).await.unwrap(), phase);
    }
    let status = pic.canister_status(id, Some(root.id)).await.unwrap();
    assert!(status.module_hash.is_none());
    assert_eq!(status.settings.controllers, [root.id]);
    assert!(
        matches!(inventory().await.as_slice(), [(_, Asset::Available(id, balance))] if *id == spare && *balance >= u128::from(floor.get()))
    );
    let more = Refill {
        sequence: 1,
        ..changed
    };
    let response = pic
        .update_call(
            root.id,
            root.operator,
            RefillStandby::NAME,
            RefillStandby::encode_request(&more).unwrap(),
        )
        .await
        .unwrap();
    assert!(matches!(
        RefillStandby::decode_reply(&response).unwrap().unwrap_err(),
        ReserveError::Capacity(_)
    ));
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert_eq!(server.stop().unwrap().abandoned, 0);
}

async fn configure(
    root: &crate::local_store::Client<'_>,
    capacity: &canic_host::PoolGrant<stored::SubnetCapacity>,
    workers: &canic_host::PoolGrant<stored::InstancePool>,
) {
    let pic = root.pic;
    for (method, id, request) in [
        (
            "configure_capacity",
            stored::CONFIGURE_CAPACITY,
            &capacity.request,
        ),
        (
            "approve_child_creation",
            stored::APPROVE_CHILD_CREATION,
            &workers.request,
        ),
        (
            "approve_child_creation",
            stored::APPROVE_CHILD_CREATION,
            &workers.request,
        ),
    ] {
        let reply = pic
            .update_call(root.id, root.operator, method, request.clone())
            .await
            .unwrap();
        let configured: Result<(), management::Refusal> = ic_wire::decode(
            &reply,
            ic_wire::Encoding::Frame {
                method: id,
                direction: ic_wire::Direction::Response,
            },
            stored::LIMITS,
        )
        .unwrap();
        configured.unwrap();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_returned_worker_is_paid_again_under_its_fresh_allocation() {
    return_and_reuse(true).await;
}

#[tokio::test(flavor = "current_thread")]
async fn directly_created_worker_returns_without_refunding_creation() {
    return_and_reuse(false).await;
}

async fn return_and_reuse(prepaid: bool) {
    use canic::sharding::{
        LookupPlacement, Placement, ShardTarget,
        standby::{Asset, ConfirmRelease, Refill, RefillStandby, ResetPhase, ResetWorker},
    };
    use ex_miner_pool_hub::Release;
    let base =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/examples/miner-pool/artifacts");
    let (mut server, mut owner) = support::instance_with_lifetime(
        &canic_qualification::server_directory(),
        ic_testkit::pocket_ic::PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet(),
        Duration::from_secs(180),
    )
    .await;
    let fleet = crate::miner_pool_fixture::install(&mut owner, &base, |fleet| {
        fleet.subnet_child_limit = Some(if prepaid { 2 } else { 3 });
        let w = fleet.worker_pools.first_mut().unwrap();
        w.max_workers = 1;
        w.creation_budget = 5 * management::CREATION_CYCLES;
        w.minimum_reuse_cycles = std::num::NonZeroU64::new(2 * management::CREATION_CYCLES);
    })
    .await;
    let root = &fleet.root;
    let pic = root.pic;
    let hub = fleet.pool;
    install_miner_driver(root, hub).await;
    configure(root, &fleet.capacity, &fleet.workers).await;
    let tick = async |round, goal| {
        let bytes = pic
            .update_call(
                hub,
                root.operator,
                Tick::NAME,
                Tick::encode_request(&(round, goal)).unwrap(),
            )
            .await
            .unwrap();
        Tick::decode_reply(&bytes).unwrap()
    };
    let settle = async |round, goal| {
        for _ in 0..60 {
            let message = tick(round, goal).await;
            if message == "settled" {
                return;
            }
            assert_ne!(message, "growth refused");
            pic.advance_time(Duration::from_secs(1)).await;
            pic.tick().await;
        }
        panic!("round {round} did not settle")
    };
    let deadline = pic.get_time().await.as_nanos_since_unix_epoch() + 500_000_000_000;
    // A zero refill target lets the direct case exercise a miss and then a surplus return.
    if !prepaid {
        use canic_management_protocol::stored;
        root.update::<_, Result<(), ReserveError>>(
            "configure_standby",
            stored::CONFIGURE_STANDBY,
            &stored::StandbyPolicy {
                root: root.id.to_text(),
                minimum_available: 0,
                maximum_creations: 0,
                expires_at_ns: deadline,
                minimum_import_cycles: 1u64.try_into().unwrap(),
                imports: vec![],
            },
            stored::LIMITS,
        )
        .await
        .unwrap();
    }
    let reserve_logs = async || {
        use canic::logging::Logs;
        let bytes = pic
            .update_call(
                root.id,
                root.operator,
                Logs::NAME,
                Logs::encode_request(&None).unwrap(),
            )
            .await
            .unwrap();
        Logs::decode_reply(&bytes)
            .unwrap()
            .entries
            .into_iter()
            .filter(|row| row.source == "reserve")
            .map(|row| row.message)
            .collect::<Vec<_>>()
    };
    let prepaid_id = if prepaid {
        let bytes = pic
            .update_call(
                root.id,
                root.operator,
                RefillStandby::NAME,
                RefillStandby::encode_request(&Refill {
                    root: root.id.as_slice().try_into().unwrap(),
                    sequence: 0,
                    cycles: management::CREATION_CYCLES,
                    expires_at_ns: deadline,
                })
                .unwrap(),
            )
            .await
            .unwrap();
        let Asset::Empty(spare) = RefillStandby::decode_reply(&bytes).unwrap().unwrap() else {
            panic!("expected one prepaid spare")
        };
        Some(spare)
    } else {
        None
    };
    let lookup = async |ordinal| {
        LookupPlacement::decode_reply(
            &pic.update_call(
                root.id,
                hub,
                LookupPlacement::NAME,
                LookupPlacement::encode_request(&canic::sharding::Lookup::Worker(
                    "miner".into(),
                    ordinal,
                ))
                .unwrap(),
            )
            .await
            .unwrap(),
        )
        .unwrap()
    };
    let claim = async |ordinal| {
        acquire_fixture(
            root,
            hub,
            canic::sharding::payment::Demand::Worker("miner".into(), ordinal),
        )
        .await
    };
    let (first, second) = tokio::join!(claim(0), claim(1));
    let Placement::Pending(winner) = first else {
        panic!("expected paid worker: {first:?}")
    };
    assert!(matches!(second, Placement::Busy | Placement::Full));
    assert_eq!(lookup(1).await, Placement::Unknown);
    assert_eq!(claim(1).await, Placement::Full);
    settle(1, 1).await;
    let target = |p: Placement| {
        let (allocation, shard) = p.assigned_target().unwrap().unwrap();
        ShardTarget {
            shard,
            allocation: allocation.clone(),
        }
    };
    let original = target(lookup(0).await);
    let spare = original.shard;
    if let Some(id) = prepaid_id {
        assert_eq!(id, spare);
    }
    assert_eq!(original.allocation, winner);
    assert_eq!(pic.get_controllers(hub).await, [root.id]);
    assert_eq!(
        pic.get_controllers(CanisterId::from_slice(spare.as_slice()))
            .await,
        [root.id]
    );
    settle(2, 0).await;
    Release::decode_reply(
        &pic.update_call(
            hub,
            root.operator,
            Release::NAME,
            Release::encode_request(&original).unwrap(),
        )
        .await
        .unwrap(),
    )
    .unwrap()
    .unwrap();
    ConfirmRelease::decode_reply(
        &pic.update_call(
            root.id,
            root.operator,
            ConfirmRelease::NAME,
            ConfirmRelease::encode_request(&original).unwrap(),
        )
        .await
        .unwrap(),
    )
    .unwrap()
    .unwrap();
    let reopen = async || {
        canic_qualification::upgrade_stopped(
            pic,
            root.id,
            std::fs::read(base.join("root.wasm")).unwrap(),
            vec![],
            Some(root.operator),
        )
        .await
        .unwrap()
    };
    reopen().await;
    let reset = async || {
        ResetWorker::decode_reply(
            &pic.update_call(
                root.id,
                root.operator,
                ResetWorker::NAME,
                ResetWorker::encode_request(&(original.clone(), deadline)).unwrap(),
            )
            .await
            .unwrap(),
        )
        .unwrap()
    };
    for phase in [
        ResetPhase::StopIssued,
        ResetPhase::UninstallIssued,
        ResetPhase::Uninstalled,
    ] {
        assert_eq!(reset().await.unwrap(), phase);
    }
    assert!(matches!(
        reset().await,
        Err(ReserveError::BalanceBelowMinimum(_))
    ));
    pic.add_cycles(
        CanisterId::from_slice(spare.as_slice()),
        u128::from(2 * management::CREATION_CYCLES),
    )
    .await;
    assert_eq!(reset().await.unwrap(), ResetPhase::StartIssued);
    assert_eq!(reset().await.unwrap(), ResetPhase::Returned);
    if !prepaid {
        assert_eq!(reset().await.unwrap(), ResetPhase::Returned);
        assert_eq!(
            reserve_logs()
                .await
                .iter()
                .filter(|msg| msg.contains("surplus retained"))
                .count(),
            1
        );
    }
    reopen().await;
    // Duplicate growth requests must converge on the retained winner.
    for _ in 0..2 {
        pic.submit_call(
            hub,
            root.operator,
            Tick::NAME,
            Tick::encode_request(&(3, 1)).unwrap(),
        )
        .await
        .unwrap();
    }
    settle(3, 1).await;
    let next = target(lookup(1).await);
    assert_eq!(next.shard, spare);
    assert_eq!(next.allocation.slot, original.allocation.slot);
    assert_ne!(next.allocation.generation, original.allocation.generation);
    reopen().await;
    assert_eq!(target(lookup(1).await), next);
    if !prepaid {
        assert_eq!(claim(2).await, Placement::Full);
    }
    assert!(matches!(lookup(0).await, Placement::Unavailable(..)));
    assert_eq!(reset().await.unwrap(), ResetPhase::Returned);
    let stale = ic_wire::encode(
        &(
            ic_wire::Bytes::<32>::new(original.allocation.generation.to_vec()).unwrap(),
            4u64,
            1u8,
        ),
        ic_wire::Encoding::Frame {
            method: ic_wire::method_id("mine"),
            direction: ic_wire::Direction::Request,
        },
        canic::method::LIMITS,
    )
    .unwrap();
    assert!(
        pic.update_call(CanisterId::from_slice(spare.as_slice()), hub, "mine", stale)
            .await
            .is_err()
    );
    settle(4, 1).await;
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert_eq!(server.stop().unwrap().abandoned, 0);
}

mod reserve;

#[tokio::test(flavor = "current_thread")]
async fn one_hub_recovers_ordinal_zero_independently_in_two_worker_pools() {
    use canic::sharding::{LookupPlacement, Placement};

    let base =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/examples/miner-pool/artifacts");
    let (mut server, mut owner) = support::instance_with_lifetime(
        &canic_qualification::server_directory(),
        ic_testkit::pocket_ic::PocketIcBuilder::new()
            .with_nns_subnet()
            .with_application_subnet(),
        Duration::from_secs(180),
    )
    .await;
    let fleet = crate::miner_pool_fixture::install(&mut owner, &base, |fleet| {
        fleet.worker_pools[0].max_workers = 1;
    })
    .await;
    let root = fleet.root;
    install_miner_driver(&root, fleet.pool).await;
    configure(&root, &fleet.capacity, &fleet.workers).await;
    let mut other = fleet.workers.approval.clone();
    other.component = "auxiliary".into();
    other.creation_budget = management::CREATION_CYCLES;
    root.update::<_, Result<(), management::Refusal>>(
        "approve_child_creation",
        stored::APPROVE_CHILD_CREATION,
        &stored::ChildCreationGrant::Worker(other.clone()),
        stored::LIMITS,
    )
    .await
    .unwrap();
    let request = async |name: &str, ordinal, assign| {
        let input = (name.to_owned(), ordinal);
        if assign {
            acquire_fixture(
                &root,
                fleet.pool,
                canic::sharding::payment::Demand::Worker(name.into(), ordinal),
            )
            .await
        } else {
            let bytes = root
                .pic
                .update_call(
                    root.id,
                    fleet.pool,
                    LookupPlacement::NAME,
                    LookupPlacement::encode_request(&canic::sharding::Lookup::Worker(
                        input.0.clone(),
                        input.1,
                    ))
                    .unwrap(),
                )
                .await
                .unwrap();
            LookupPlacement::decode_reply(&bytes).unwrap()
        }
    };
    let mut allocations = Vec::new();
    for pool in ["miner", "auxiliary"] {
        assert_eq!(request(pool, 0, false).await, Placement::Unknown);
        let Placement::Pending(id) = request(pool, 0, true).await else {
            panic!("expected newly reserved worker")
        };
        allocations.push(id);
        assert_eq!(request(pool, 1, true).await, Placement::Full);
    }
    assert_ne!(allocations[0].slot, allocations[1].slot);
    for _ in 0..120 {
        root.pic.advance_time(Duration::from_millis(100)).await;
        root.pic.tick().await;
    }
    canic_qualification::upgrade_stopped(
        root.pic,
        root.id,
        std::fs::read(base.join("root.wasm")).unwrap(),
        vec![],
        Some(root.operator),
    )
    .await
    .unwrap();
    for (name, id) in ["miner", "auxiliary"].into_iter().zip(allocations) {
        let result = request(name, 0, false).await;
        assert!(
            matches!(&result, Placement::Assigned(original, _) if original == &id),
            "{result:?}"
        );
        assert_eq!(request(name, 0, true).await, result);
    }
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert_eq!(server.stop().unwrap().abandoned, 0);
}

async fn install_miner_driver(root: &Client<'_>, hub: CanisterId) {
    canic_qualification::upgrade_stopped(
        root.pic,
        hub,
        std::fs::read(artifacts().join("miner-payer.wasm")).unwrap(),
        vec![],
        Some(root.id),
    )
    .await
    .unwrap();
}
