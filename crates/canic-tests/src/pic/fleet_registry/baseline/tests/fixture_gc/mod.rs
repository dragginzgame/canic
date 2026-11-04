//! Module: pic::fleet_registry::baseline::tests::fixture_gc
//!
//! Responsibility: measure bounded Store cleanup and qualify interrupted replay.
//! Boundary: real maximum-size fixture chunks use the existing Store GC owner.

use super::*;
use canic::dto::template::{WasmStoreGcRequest, WasmStoreGcTarget, WasmStoreStatusResponse};
use canic_contracts::ids::WasmStoreGcMode;
use ic_testkit::pic::{WireCallSpec, query_wire, update_wire};

const OPERATION: [u8; 32] = [0x96; 32];
const CHUNKS: u32 = 33;

#[test]
pub(super) fn fixture_cleanup_preserves_bounded_progress_and_replay() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let fixture = setup_fresh_active_component_registry();
    let pic = fixture.pic();
    let store = fixture.wasm_store;
    let root = fixture.root;
    crate::pic::protected_state::set_funding(
        pic,
        root,
        Principal::anonymous(),
        canic::dto::state::SetCyclesFundingRequest { enabled: false },
    )
    .unwrap();
    for _ in 0..20 {
        pic.tick().unwrap();
    }
    let content = publish(pic, store, root);
    let initial = status(pic, store, root);
    assert_eq!(initial.inventory.stored_fixture_chunks, u64::from(CHUNKS));
    gc(pic, store, root, OPERATION, WasmStoreGcTarget::Prepared).unwrap();
    let prepared = status(pic, store, root);
    gc(pic, store, root, OPERATION, WasmStoreGcTarget::Prepared).unwrap();
    assert_eq!(status(pic, store, root), prepared);
    assert_eq!(
        gc(pic, store, root, [0x97; 32], WasmStoreGcTarget::Complete),
        Err(Error::from_registered(
            canic_contracts::diagnostics::codes::STATE_CONFLICT
        ))
    );
    assert_eq!(status(pic, store, root), prepared);
    let denied = store_upload_as(
        pic,
        store,
        root,
        FixtureChunkUpload {
            content_id: content,
            index: 0,
            bytes: vec![0; canic_core::CANIC_WASM_CHUNK_BYTES],
        },
    );
    assert_eq!(
        denied,
        Err(Error::from_registered(
            canic_contracts::diagnostics::codes::STATE_CONFLICT
        ))
    );

    let mut requests = 0;
    let mut maximum_request_micros = 0;
    let mut total_micros = 0;
    let mut previous = prepared;
    while previous.gc.mode != WasmStoreGcMode::Complete {
        assert!(
            requests < 3,
            "maximum-size fixture cleanup must finish in three batches"
        );
        let before = Instant::now();
        gc(pic, store, root, OPERATION, WasmStoreGcTarget::Complete).unwrap();
        // The final batch also awaits management chunk-store inspection and
        // clearing; drain those callbacks before deciding another request is needed.
        for _ in 0..20 {
            pic.tick().unwrap();
        }
        let spent = before.elapsed().as_micros();
        maximum_request_micros = maximum_request_micros.max(spent);
        total_micros += spent;
        requests += 1;
        let current = status(pic, store, root);
        assert!(current.occupied_store_bytes < previous.occupied_store_bytes);
        assert!(
            previous.inventory.stored_fixture_chunks - current.inventory.stored_fixture_chunks
                <= 16
        );
        if requests == 1 {
            assert_eq!(current.gc.mode, WasmStoreGcMode::Clearing);
            // Stop/restore the same installed state between cleanup transactions.
            pic.stop_canister(store, Some(root)).unwrap();
            let snapshot = pic.take_canister_snapshot(store, Some(root), None).unwrap();
            pic.load_canister_snapshot(store, Some(root), snapshot.id.clone())
                .unwrap();
            pic.start_canister(store, Some(root)).unwrap();
            assert_eq!(status(pic, store, root), current);
            pic.delete_canister_snapshot(store, Some(root), snapshot.id)
                .unwrap();
            for _ in 0..10 {
                pic.tick().unwrap();
            }
        }
        previous = current;
    }
    eprintln!(
        "fixture GC: chunks={CHUNKS} chunk_bytes={} requests={requests} total_micros={total_micros} maximum_request_micros={maximum_request_micros}",
        canic_core::CANIC_WASM_CHUNK_BYTES
    );
    assert_eq!(previous.inventory.stored_fixture_chunks, 0);
    assert_eq!(previous.inventory.fixture_sources, 0);
    assert_eq!(previous.occupied_store_bytes, 0);
    gc(pic, store, root, OPERATION, WasmStoreGcTarget::Complete).unwrap();
    for _ in 0..4 {
        pic.tick().unwrap();
    }
    assert_eq!(status(pic, store, root), previous);
    drop(fixture);
}

fn publish(pic: &PocketIc, store: Principal, root: Principal) -> [u8; 32] {
    let bytes = vec![0; canic_core::CANIC_WASM_CHUNK_BYTES];
    let descriptor = canic::dto::fixture_provisioning::FixtureDescriptor {
        schema_version: 1,
        format_hash: [0x96; 32],
        completion_summary: [0x96; 32],
        encoded_length: u64::from(CHUNKS) * bytes.len() as u64,
        chunks: (0..CHUNKS)
            .map(
                |_| canic::dto::fixture_provisioning::FixtureChunkDescriptor {
                    digest: wasm_hash(&bytes).try_into().unwrap(),
                    length: u32::try_from(bytes.len()).unwrap(),
                },
            )
            .collect(),
    };
    let response: Result<Result<FixtureSourceStatus, FixtureStoreError>, Error> = update_wire(
        pic,
        store,
        root,
        WireCallSpec {
            method: canic::protocol::store_publication::fixture_prepare::METHOD,
            method_id: canic::protocol::store_publication::fixture_prepare::METHOD_ID,
            request_limits: canic::protocol::store_publication::REQUEST_LIMITS,
            response_limits: canic::protocol::store_publication::RESPONSE_LIMITS,
        },
        &descriptor,
    )
    .unwrap();
    let content = response.unwrap().unwrap().content_id;
    for index in 0..CHUNKS {
        store_upload_as(
            pic,
            store,
            root,
            FixtureChunkUpload {
                content_id: content,
                index,
                bytes: bytes.clone(),
            },
        )
        .unwrap()
        .unwrap();
    }
    content
}

fn gc(
    pic: &PocketIc,
    store: Principal,
    root: Principal,
    operation_id: [u8; 32],
    target: WasmStoreGcTarget,
) -> Result<canic::dto::role::OperationReceipt, Error> {
    update_wire(
        pic,
        store,
        root,
        WireCallSpec {
            method: canic::protocol::store_gc::METHOD,
            method_id: canic::protocol::store_gc::METHOD_ID,
            request_limits: canic::protocol::store_gc::REQUEST_LIMITS,
            response_limits: canic::protocol::store_gc::RESPONSE_LIMITS,
        },
        &WasmStoreGcRequest {
            operation_id,
            target,
        },
    )
    .unwrap()
}

fn status(pic: &PocketIc, store: Principal, root: Principal) -> WasmStoreStatusResponse {
    let response: Result<WasmStoreStatusResponse, Error> = query_wire(
        pic,
        store,
        root,
        WireCallSpec {
            method: canic::protocol::store_inventory::storage::METHOD,
            method_id: canic::protocol::store_inventory::storage::METHOD_ID,
            request_limits: canic::protocol::store_inventory::REQUEST_LIMITS,
            response_limits: canic::protocol::store_inventory::RESPONSE_LIMITS,
        },
        &(),
    )
    .unwrap();
    response.unwrap()
}
