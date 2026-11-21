//! The actual pool calls Root for worker custody and its miners for each batch.
use super::*;
use canic::method::Contract;
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
    } = crate::miner_pool_fixture::install(&mut owner, &base).await;
    let pic = root.pic;
    let tick = |round, goal| Tick::encode_request(&(round, goal)).unwrap();
    // The hub is installed but Root has not approved its grant yet.
    let refused = pic
        .update_call(pool, root.operator, Tick::NAME, tick(1, 2))
        .await
        .unwrap();
    assert_eq!(
        Tick::decode_reply(&refused).unwrap(),
        "worker 0: Root call rejected"
    );
    for (method, id, request) in [
        (
            "configure_capacity",
            stored::CONFIGURE_CAPACITY,
            &capacity.request,
        ),
        (
            "configure_workers",
            stored::CONFIGURE_WORKERS,
            &workers.request,
        ),
        (
            "configure_workers",
            stored::CONFIGURE_WORKERS,
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
    let mut changed = workers.approval.clone();
    changed.max_instances -= 1;
    let conflict: Result<(), management::Refusal> = root
        .update(
            "configure_workers",
            stored::CONFIGURE_WORKERS,
            &changed,
            stored::LIMITS,
        )
        .await;
    assert_eq!(conflict, Err(management::Refusal::Occupied));
    let outsider = CanisterId::self_authenticating(b"other hub");
    assert!(
        pic.update_call(
            root.id,
            outsider,
            "assign_worker",
            canic::sharding::AssignWorker::encode_request(&0).unwrap()
        )
        .await
        .is_err()
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
    use canic::sharding::standby::{Asset, InspectStandby, Refill, RefillStandby};
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
    assert_eq!(
        RefillStandby::decode_reply(&reply).unwrap().unwrap_err(),
        "refill approval differs"
    );
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
        InspectStandby::decode_reply(
            &pic.update_call(
                root.id,
                root.operator,
                InspectStandby::NAME,
                InspectStandby::encode_request(&()).unwrap(),
            )
            .await
            .unwrap(),
        )
        .unwrap()
    };
    assert_eq!(
        inventory().await,
        vec![(refill.clone(), Asset::Empty(spare))]
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
            canic::sharding::AssignWorker::NAME,
            canic::sharding::AssignWorker::encode_request(&4).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        canic::sharding::AssignWorker::decode_reply(&bytes).unwrap(),
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
        vec![(refill, Asset::Released(spare, spare_target.allocation))]
    );
    let response = pic
        .update_call(
            root.id,
            pool,
            canic::sharding::LookupWorker::NAME,
            canic::sharding::LookupWorker::encode_request(&0).unwrap(),
        )
        .await
        .unwrap();
    assert!(matches!(
        canic::sharding::LookupWorker::decode_reply(&response).unwrap(),
        canic::sharding::Placement::Unavailable(..)
    ));
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
    assert_eq!(
        RefillStandby::decode_reply(&response).unwrap().unwrap_err(),
        "no approved subnet capacity"
    );
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert_eq!(server.stop().unwrap().abandoned, 0);
}
