//! Installed worker contract with distinct simulated Root, pool and outsider principals.
use canic::{method::LIMITS, topology::Install};
use ic_testkit::{
    pocket_ic::{CanisterId, PocketIcBuilder},
    server::{Deletion, Limits, PreparedServer, Server},
};
use ic_wire::{Direction, Encoding};
use std::{path::Path, time::Duration};

#[tokio::test(flavor = "current_thread")]
async fn pool_recovers_exact_blocks_across_lost_reply_and_upgrade() {
    let wasm = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/examples/miner-pool/artifacts/miner.wasm"),
    )
    .expect("run make miner-test");
    let mut server = Server::start(
        PreparedServer::check(&canic_qualification::server_directory()).unwrap(),
        Limits {
            startup: Duration::from_secs(30),
            lifetime: Duration::from_secs(120),
            stdout_bytes: 1048576,
            stderr_bytes: 1048576,
        },
    )
    .unwrap();
    let owner = server
        .instance(
            PocketIcBuilder::new().with_application_subnet(),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    let pic = owner.client();
    let root = pic.create_canister().await;
    let pool = pic.create_canister().await;
    let miner = pic.create_canister_with_settings(Some(root), None).await;
    let args = Install {
        root: root.as_slice().try_into().unwrap(),
        parent: pool.as_slice().try_into().unwrap(),
        role: "miner".into(),
        application: vec![3; 32],
    }
    .encode()
    .unwrap();
    pic.install_canister(miner, wasm.clone(), args, Some(root))
        .await;

    let ready = async || {
        use canic::readiness::{METHOD, REQUEST_LIMITS, RESPONSE_LIMITS, State};
        let frame = |direction| Encoding::Frame {
            method: METHOD,
            direction,
        };
        let bytes = ic_wire::encode(&(), frame(Direction::Request), REQUEST_LIMITS).unwrap();
        let reply = pic
            .update_call(miner, CanisterId::anonymous(), "readiness", bytes)
            .await
            .unwrap();
        ic_wire::decode::<State>(&reply, frame(Direction::Response), RESPONSE_LIMITS).unwrap()
    };
    assert_eq!(ready().await, canic::readiness::State::Ready);
    let frame = |direction| Encoding::Frame {
        method: ic_wire::method_id("mine"),
        direction,
    };
    let encode = |round: u64, ordinal: u8| {
        ic_wire::encode(
            &(
                ic_wire::Bytes::<32>::new(vec![3; 32]).unwrap(),
                round,
                ordinal,
            ),
            frame(Direction::Request),
            LIMITS,
        )
        .unwrap()
    };
    let mine = async |caller, round, ordinal| {
        pic.update_call(miner, caller, "mine", encode(round, ordinal))
            .await
            .map(|bytes| {
                ic_wire::decode::<(u64, bool)>(&bytes, frame(Direction::Response), LIMITS).unwrap()
            })
    };
    let receipt = |attempt, round: u64, ordinal: u8| {
        use sha2::{Digest, Sha256};
        let hash = Sha256::new()
            .chain_update(miner.as_slice())
            .chain_update(round.to_be_bytes())
            .chain_update([ordinal])
            .finalize();
        (attempt, hash[0] < 64)
    };
    let stale = ic_wire::encode(
        &(ic_wire::Bytes::<32>::new(vec![2; 32]).unwrap(), 1u64, 1u8),
        frame(Direction::Request),
        LIMITS,
    )
    .unwrap();
    assert!(pic.update_call(miner, pool, "mine", stale).await.is_err());
    assert!(mine(root, 1, 1).await.is_err());
    assert!(mine(CanisterId::anonymous(), 1, 1).await.is_err());
    assert!(mine(pool, 1, 2).await.is_err());
    // Drop the ingress handle; the following batch can succeed only after the first committed.
    let _lost = pic
        .submit_call(miner, pool, "mine", encode(1, 1))
        .await
        .unwrap();
    pic.tick().await;
    assert_eq!(mine(pool, 1, 2).await.unwrap(), receipt(2, 1, 2));
    assert!(mine(pool, 1, 3).await.is_err());
    canic_qualification::upgrade_stopped(pic, miner, wasm.clone(), vec![], Some(root))
        .await
        .unwrap();
    assert!(mine(root, 1, 1).await.is_err());
    assert_eq!(ready().await, canic::readiness::State::Ready);
    assert_eq!(mine(pool, 1, 1).await.unwrap(), receipt(1, 1, 1));
    assert_eq!(mine(pool, 1, 2).await.unwrap(), receipt(2, 1, 2));
    assert_eq!(mine(pool, 4, 1).await.unwrap(), receipt(3, 4, 1));
    assert!(mine(pool, 1, 1).await.is_err());
    assert_eq!(mine(pool, 4, 2).await.unwrap(), receipt(4, 4, 2));
    // Reinstall on the same principal under the same parent. A batch already on the
    // wire for the original allocation must fail before it touches the new state.
    pic.uninstall_canister(miner, Some(root)).await.unwrap();
    let next = Install {
        root: root.as_slice().try_into().unwrap(),
        parent: pool.as_slice().try_into().unwrap(),
        role: "miner".into(),
        application: vec![4; 32],
    };
    pic.install_canister(miner, wasm, next.encode().unwrap(), Some(root))
        .await;
    assert!(mine(pool, 1, 1).await.is_err());
    let fresh = ic_wire::encode(
        &(ic_wire::Bytes::<32>::new(vec![4; 32]).unwrap(), 1u64, 1u8),
        frame(Direction::Request),
        LIMITS,
    )
    .unwrap();
    let bytes = pic.update_call(miner, pool, "mine", fresh).await.unwrap();
    assert_eq!(
        ic_wire::decode::<(u64, bool)>(&bytes, frame(Direction::Response), LIMITS).unwrap(),
        receipt(1, 1, 1)
    );
    assert_eq!(
        owner.close(Duration::from_secs(10)).await,
        Deletion::Attempted
    );
    assert_eq!(server.stop().unwrap().abandoned, 0);
}
