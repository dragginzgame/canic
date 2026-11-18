//! Install both behaviors and reopen the same hub under another application role.
#[allow(
    dead_code,
    reason = "The shared Toko harness also supports predecessor selection in other tests."
)]
mod support;

use canic::auth::install::{Install, LimitsPolicy};
use ic_db::Principal;
use ic_testkit::pocket_ic::{CanisterId, PocketIcBuilder};
use ic_wire::{Direction, Encoding};

#[tokio::test(flavor = "current_thread")]
async fn display_names_preserve_behavior_and_storage() {
    let (server, instance) = support::start(PocketIcBuilder::new().with_application_subnet()).await;
    let pic = instance.client();
    let root = pic.create_canister().await;
    let hub = pic.create_canister().await;
    let shard = pic.create_canister().await;
    let principal = |id: CanisterId| Principal::from_slice(id.as_slice());
    let wasm = |name| {
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../apps/toko/artifacts")
                .join(name),
        )
        .unwrap()
    };
    let args = |application| {
        Install {
            root: principal(root),
            issuer: principal(root),
            network: [1; 32],
            audience: [2; 32],
            role: "user".into(),
            limits: LimitsPolicy {
                default_session_secs: 60,
                max_session_secs: 1800,
                max_proof_ttl_ns: 600_000_000_000,
                max_sessions: None,
            },
            application,
        }
        .encode()
        .unwrap()
    };
    let metadata = async |target| {
        let frame = |direction| Encoding::Frame {
            method: ic_wire::method_id("role_metadata"),
            direction,
        };
        let bytes = ic_wire::encode(&(), frame(Direction::Request), canic::method::LIMITS).unwrap();
        let reply = pic
            .query_call(target, CanisterId::anonymous(), "role_metadata", bytes)
            .await
            .unwrap();
        ic_wire::decode::<(String, String)>(
            &reply,
            frame(Direction::Response),
            canic::method::LIMITS,
        )
        .unwrap()
    };
    for id in [hub, shard] {
        pic.add_cycles(id, 10_000_000_000_000).await;
    }
    pic.install_canister(hub, wasm("users.wasm"), args(vec![5; 16]), None)
        .await;
    let original = metadata(hub).await;
    assert_eq!(
        original,
        ("user_hub".into(), "toko.user.hub.principal.v1".into())
    );
    let before = pic.get_stable_memory(hub).await;
    // Allow the simulator to replenish the install instruction allowance.
    for _ in 0..100 {
        pic.tick().await;
    }
    pic.upgrade_canister(hub, wasm("projects.wasm"), vec![], None)
        .await
        .unwrap();
    assert_eq!(metadata(hub).await, ("project_hub".into(), original.1));
    assert!(pic.get_stable_memory(hub).await == before);
    pic.set_controllers(shard, None, vec![root]).await.unwrap();
    let topology = canic::topology::Install {
        root: principal(root),
        parent: principal(hub),
        application: vec![7; 16],
    };
    pic.install_canister(
        shard,
        wasm("archive.wasm"),
        args(topology.encode().unwrap()),
        Some(root),
    )
    .await;
    assert_eq!(
        metadata(shard).await,
        ("account_archive".into(), "toko.user.shard.user.v1".into())
    );
    support::finish(server, instance).await;
}
