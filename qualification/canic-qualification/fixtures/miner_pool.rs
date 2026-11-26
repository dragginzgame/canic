//! Install the local fleet and prepare its bounded Host approvals for tests and the prompt.
use super::local_store::Client;
use canic_management_protocol::{
    self as management,
    children::{ChildId, Request},
    stored,
};
use ic_testkit::{pocket_ic::CanisterId, server::Instance};
use std::{path::Path, time::Duration};

pub(crate) struct Fleet<'a> {
    pub root: Client<'a>,
    pub pool: CanisterId,
    pub capacity: canic_host::PoolGrant<stored::SubnetCapacity>,
    pub standby: Option<canic_host::PoolGrant<stored::StandbyPolicy>>,
    pub workers: canic_host::PoolGrant<stored::InstancePool>,
}

pub(crate) async fn install<'a>(
    owner: &'a mut Instance,
    base: &Path,
    configure: impl FnOnce(&mut canic_host::FleetDeclaration),
) -> Fleet<'a> {
    let read = |name: &str| std::fs::read(base.join(name)).expect("built Miner Pool Wasm");
    let gateway = owner.client_mut().make_live(None).await;
    owner.client_mut().stop_live().await;
    let pic = owner.client();
    let (root, store) =
        super::local_store::fixture_with_code(pic, read("root.wasm"), read("wasm_store.wasm"))
            .await;
    let hub_manifest = store.publish_named("pool", "1", &read("pool.wasm")).await;
    let worker_manifest = store.publish_named("miner", "1", &read("miner.wasm")).await;
    pic.set_controllers(store.id, Some(root.operator), vec![root.id])
        .await
        .unwrap();
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let deadline = now + 1_800_000_000_000;
    let staged: Result<management::Status, management::Refusal> = root
        .update(
            "stage_child_creation",
            canic_management_protocol::application::STAGE_CHILD_CREATION,
            &Request {
                slot: ChildId::FIRST,
                value: canic_management_protocol::application::Deployment::<(), _>::Application(
                    management::application::Stage {
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
                ),
            },
            stored::LIMITS,
        )
        .await;
    staged.unwrap();
    for _ in 0..100 {
        pic.advance_time(Duration::from_millis(100)).await;
        pic.tick().await;
    }
    let status: management::Status = root.observe_creation(ChildId::FIRST).await;
    assert_eq!(status.phase, management::Phase::Ready, "{status:?}");
    let pool = CanisterId::from_text(status.worker.unwrap()).unwrap();
    pic.add_cycles(pool, 20_000_000_000_000).await;
    let config_path = base.with_file_name("canic.toml");
    let mut config = canic_host::load_config(&config_path).unwrap();
    let fleet = config
        .apps
        .get_mut("miner-pool")
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
    let worker_pool = fleet.worker_pools.first_mut().unwrap();
    worker_pool.expires_at_ns = Some(deadline);
    configure(fleet);
    let selected = config.select(None, None).unwrap();
    let plan = canic_host::plan_fleet(
        base,
        &config_path,
        &selected,
        &root.operator.to_text(),
        None,
        None,
    )
    .unwrap();
    let canic_host::FleetPlanning::PlacementPools {
        capacity: Some(capacity),
        standby,
        mut workers,
        instances: None,
        shards,
        ..
    } = plan
    else {
        panic!("expected independent worker grant");
    };
    assert!(shards.is_empty());
    assert_eq!(workers.len(), 1);
    let workers = workers.pop().unwrap();
    assert_eq!(workers.approval.reference.manifest, worker_manifest);
    Fleet {
        root,
        pool,
        capacity,
        standby,
        workers,
    }
}
