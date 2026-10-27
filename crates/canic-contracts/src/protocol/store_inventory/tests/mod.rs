//! Catalog and retained inventory use different identities despite sharing caller authority.
use super::*;
use crate::dto::{error::Error, template::*};

#[test]
fn catalog_frame_preserves_hashes_sizes_and_rejects_storage_identity() {
    let entries = vec![WasmStoreCatalogEntryResponse {
        role: crate::ids::CanisterRole::WASM_STORE,
        template_id: "store".into(),
        version: "current".into(),
        payload_hash: vec![255; 32],
        payload_size_bytes: u64::MAX,
    }];
    let frame = ic_wire::native::encode_response(
        catalog::METHOD_ID,
        &Ok::<_, Error>(entries.clone()),
        RESPONSE_LIMITS,
    )
    .unwrap();
    let decoded: Result<Vec<WasmStoreCatalogEntryResponse>, Error> =
        ic_wire::native::decode_response(catalog::METHOD_ID, &frame, RESPONSE_LIMITS).unwrap();
    assert_eq!(decoded.unwrap(), entries);
    assert!(matches!(
        ic_wire::native::decode_response::<Result<Vec<WasmStoreCatalogEntryResponse>, Error>>(
            storage::METHOD_ID,
            &frame,
            RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn storage_and_catalog_have_complete_native_graphs() {
    ic_wire::typescript::native_schema::<Result<WasmStoreStatusResponse, Error>>().unwrap();
    ic_wire::typescript::native_schema::<Result<Vec<WasmStoreCatalogEntryResponse>, Error>>()
        .unwrap();
}
