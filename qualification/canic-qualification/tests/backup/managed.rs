//! The operator controls Root. Only Root controls the admitted counter.
use super::*;
use canic_management_protocol::{
    Approval, CREATION_CYCLES, LIMITS, Phase, STAGE, STAGE_LIMITS, STATUS, Stage, Status, digest,
};
use ic_testkit::pocket_ic::nonblocking::PocketIc;

pub(super) async fn install(
    pic: &PocketIc,
    caller: CanisterId,
) -> (CanisterId, CanisterId, Vec<u8>, Vec<u8>) {
    let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/backup");
    let root_wasm = fs::read(assets.join("managed-root.wasm")).unwrap();
    let child_wasm = fs::read(assets.join("managed-counter.wasm")).unwrap();
    let root = pic.create_canister().await;
    pic.add_cycles(root, 10_000_000_000_000).await;
    pic.install_canister(root, root_wasm.clone(), vec![], None)
        .await;
    pic.set_controllers(root, None, vec![caller]).await.unwrap();
    let now = pic.get_time().await.as_nanos_since_unix_epoch();
    let stage = Stage {
        approval: Approval {
            manager: root.to_text(),
            module_hash: digest(&child_wasm),
            creation_cycles: CREATION_CYCLES,
            execute_at_ns: now,
            expires_at_ns: now + 120_000_000_000,
        },
        wasm: child_wasm.clone(),
    };
    let request = |method| ic_wire::Encoding::Frame {
        method,
        direction: ic_wire::Direction::Request,
    };
    pic.update_call(
        root,
        caller,
        "stage",
        ic_wire::encode(&stage, request(STAGE), STAGE_LIMITS).unwrap(),
    )
    .await
    .unwrap();
    for _ in 0..30 {
        pic.advance_time(Duration::from_millis(100)).await;
        pic.tick().await;
    }
    let reply = pic
        .query_call(
            root,
            caller,
            "status",
            ic_wire::encode(&(), request(STATUS), LIMITS).unwrap(),
        )
        .await
        .unwrap();
    let status: Status = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: STATUS,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .unwrap();
    assert_eq!(status.phase, Phase::Ready);
    let child = CanisterId::from_text(status.worker.unwrap()).unwrap();
    assert_eq!(
        pic.canister_status(child, Some(root))
            .await
            .unwrap()
            .settings
            .controllers,
        [root]
    );
    (root, child, root_wasm, child_wasm)
}
