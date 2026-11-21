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
    let gateway = owner.client_mut().make_live(None).await;
    owner.client_mut().stop_live().await;
    let pic = owner.client();
    let (root, store) =
        super::readiness_tests::fixture_with_code(pic, read("root.wasm"), read("store.wasm")).await;
    let hub_manifest = store.publish_named("pool", "1", &read("pool.wasm")).await;
    let worker_manifest = store.publish_named("miner", "1", &read("miner.wasm")).await;
    pic.set_controllers(store.id, Some(root.operator), vec![root.id])
        .await
        .unwrap();
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let deadline = now + 1_800_000_000_000;
    let staged: Result<management::Status, management::Refusal> = root
        .update(
            "stage_application_child",
            management::application::STAGE,
            &Request {
                slot: ChildId::FIRST,
                value: management::application::Stage {
                    source: stored::Stage {
                        approval: management::Approval {
                            manager: root.id.to_text(),
                            module_hash: hub_manifest.hash,
                            creation_cycles: management::CREATION_CYCLES,
                            execute_at_ns: now,
                            expires_at_ns: now + 300_000_000_000,
                        },
                        reference: stored::Reference {
                            store: store.id.to_text(),
                            manifest: hub_manifest,
                        },
                    },
                    arguments: management::application::Arguments::new(
                        root.operator.as_slice().to_vec(),
                    )
                    .unwrap(),
                },
            },
            stored::LIMITS,
        )
        .await;
    staged.unwrap();
    settle(pic).await;
    let status: management::Status = root
        .update(
            "observe_child",
            children::STATUS,
            &ChildId::FIRST,
            management::LIMITS,
        )
        .await;
    assert_eq!(status.phase, management::Phase::Ready, "{status:?}");
    let pool = CanisterId::from_text(status.worker.unwrap()).unwrap();
    let config_path = base.with_file_name("canic.toml");
    let mut config = canic_host::load_config(&config_path).unwrap();
    let fleet = config
        .apps
        .get_mut("ex-miner-pool")
        .unwrap()
        .fleets
        .get_mut("local")
        .unwrap();
    fleet.network = canic_host::FleetNetwork::Bound(canic_host::Network::Local {
        url: gateway.to_string(),
        root_key: pic
            .root_key()
            .await
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        root_subnet: Some(pic.topology().await.get_nns().unwrap().to_text()),
        routing_canister: root.id.to_text(),
    });
    for (name, id) in [("root", root.id), ("store", store.id), ("pool", pool)] {
        fleet.canisters.insert(
            name.into(),
            canic_host::CanisterBinding {
                id: id.to_text(),
                profile_manager: false,
            },
        );
    }
    fleet.worker_pool.as_mut().unwrap().expires_at_ns = Some(deadline);
    let selected = config.select(None, None).unwrap();
    let plan = canic_host::plan_fleet(
        &base,
        &config_path,
        &selected,
        &root.operator.to_text(),
        None,
        None,
    )
    .unwrap();
    let canic_host::FleetPlanning::PlacementPools {
        capacity: Some(capacity),
        workers: Some(workers),
        instances: None,
        shards: None,
        ..
    } = plan
    else {
        panic!("expected independent worker grant");
    };
    assert_eq!(workers.approval.reference.manifest, worker_manifest);
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
            (report.round, report.goal, report.produced),
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
        assert!(ids.starts_with(&originals));
        originals = ids;
        println!(
            "Round {round}, goal {goal}, produced {}, miners {}",
            report.produced,
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
            assert_eq!(report.workers.iter().map(|w| w.blocks).sum::<u64>(), 21);
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
        (draining.round, draining.goal, draining.produced),
        (8, 0, 0)
    );
    assert_eq!(draining.workers[0].status, "draining");
    assert_eq!(draining.workers.iter().map(|w| w.blocks).sum::<u64>(), 21);
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
    assert_eq!((parked.round, parked.goal, parked.produced), (8, 0, 0));
    assert!(parked.workers.iter().all(|w| w.status == "parked"));
    assert_eq!(parked.workers.iter().map(|w| w.blocks).sum::<u64>(), 22);
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        ic_testkit::server::Deletion::Attempted
    );
    assert_eq!(server.stop().unwrap().abandoned, 0);
}
