//! Actual Store retirement: exact intent replay, retained preparation and bounded clearing.

use super::{pull, restart};
use candid::Principal;
use canic::{
    Error,
    dto::fixture_provisioning::{FixtureChunkRead, FixtureStoreError},
    protocol::{CANIC_WASM_STORE_CATALOG, CANIC_WASM_STORE_COMMAND},
};
use canic_control_plane::dto::template::{
    StoreCatalogRequest, StoreCatalogResponse, StoreCommand, StoreCommandResponse,
    WasmStoreGcRequest, WasmStoreGcTarget, WasmStoreStatusResponse,
};
use canic_control_plane::ids::WasmStoreGcMode;
use ic_testkit::pic::{CandidCallExt, PocketIc};
use std::time::Duration;

pub(super) fn qualify(
    pic: &PocketIc,
    store: Principal,
    root: Principal,
    controller: Principal,
    wasm: &[u8],
    target: Principal,
    read: FixtureChunkRead,
) {
    let operation = [0xc1; 32];
    let before = status(pic, store, root);
    assert!(before.occupied_store_bytes > 0);
    assert!(command(pic, store, root, operation, WasmStoreGcTarget::Complete).is_err());
    assert!(
        command(
            pic,
            store,
            controller,
            operation,
            WasmStoreGcTarget::Prepared
        )
        .is_err()
    );
    assert_eq!(status(pic, store, root), before);
    command(pic, store, root, operation, WasmStoreGcTarget::Prepared).unwrap();
    let prepared = status(pic, store, root);
    assert_eq!(prepared.gc.mode, WasmStoreGcMode::Prepared);
    assert_eq!(prepared.occupied_store_bytes, before.occupied_store_bytes);
    // Discarding and replaying preparation receipts must never advance to deletion.
    for _ in 0..2 {
        command(pic, store, root, operation, WasmStoreGcTarget::Prepared).unwrap();
        pic.tick();
        assert_eq!(status(pic, store, root), prepared);
    }
    assert_eq!(
        pull(pic, target, store, read.clone()),
        Err(FixtureStoreError::Authority)
    );
    restart(pic, store, controller, wasm);
    command(pic, store, root, operation, WasmStoreGcTarget::Prepared).unwrap();
    assert_eq!(status(pic, store, root), prepared);
    command(pic, store, root, operation, WasmStoreGcTarget::Complete).unwrap();
    for _ in 0..10 {
        pic.advance_time(Duration::from_secs(1));
        pic.tick();
        if status(pic, store, root).gc.mode == WasmStoreGcMode::Clearing {
            break;
        }
    }
    let clearing = status(pic, store, root);
    assert_eq!(clearing.gc.mode, WasmStoreGcMode::Clearing);
    assert!(clearing.occupied_store_bytes > 0);
    assert!(clearing.occupied_store_bytes < prepared.occupied_store_bytes);
    restart(pic, store, controller, wasm);
    command(pic, store, root, operation, WasmStoreGcTarget::Prepared).unwrap();
    assert_eq!(status(pic, store, root), clearing);
    for _ in 0..20 {
        if status(pic, store, root).gc.mode == WasmStoreGcMode::Complete {
            break;
        }
        command(pic, store, root, operation, WasmStoreGcTarget::Complete).unwrap();
        pic.advance_time(Duration::from_secs(1));
        pic.tick();
    }
    let complete = status(pic, store, root);
    assert_eq!(complete.gc.mode, WasmStoreGcMode::Complete);
    assert_eq!(complete.gc.runs_completed, 1);
    assert_eq!(complete.occupied_store_bytes, 0);
    assert_eq!(
        pull(pic, target, store, read),
        Err(FixtureStoreError::Authority)
    );
    for target in [WasmStoreGcTarget::Prepared, WasmStoreGcTarget::Complete] {
        command(pic, store, root, operation, target).unwrap();
        assert!(command(pic, store, root, [0xc2; 32], target).is_err());
        pic.tick();
        assert_eq!(status(pic, store, root), complete);
    }
}

fn command(
    pic: &PocketIc,
    store: Principal,
    caller: Principal,
    operation_id: [u8; 32],
    target: WasmStoreGcTarget,
) -> Result<(), Error> {
    let result: Result<StoreCommandResponse, Error> = pic
        .update_candid_as(
            store,
            caller,
            CANIC_WASM_STORE_COMMAND,
            (StoreCommand::RunGc(WasmStoreGcRequest {
                operation_id,
                target,
            }),),
        )
        .unwrap();
    let StoreCommandResponse::OperationAccepted(receipt) = result? else {
        panic!("GC receipt");
    };
    assert_eq!(receipt.operation_id, operation_id);
    Ok(())
}

fn status(pic: &PocketIc, store: Principal, root: Principal) -> WasmStoreStatusResponse {
    let result: Result<StoreCatalogResponse, Error> = pic
        .update_candid_as(
            store,
            root,
            CANIC_WASM_STORE_CATALOG,
            (StoreCatalogRequest::Storage,),
        )
        .unwrap();
    let StoreCatalogResponse::Storage(status) = result.unwrap() else {
        panic!("Store storage response");
    };
    status
}
