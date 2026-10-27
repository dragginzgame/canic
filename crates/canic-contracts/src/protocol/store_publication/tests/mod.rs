//! Native publication controls preserve metadata, hashes and typed Store failures.
use super::*;
use crate::{
    dto::{error::Error, fixture_provisioning::*, template::*},
    ids::*,
};

#[test]
fn preparation_preserves_manifest_and_exact_chunk_hashes() {
    let manifest = TemplateManifestInput {
        template_id: "template".into(),
        role: CanisterRole::WASM_STORE,
        version: "release".into(),
        payload_hash: vec![1; 32],
        payload_size_bytes: 42,
        store_binding: WasmStoreBinding::new("store"),
        chunking_mode: TemplateChunkingMode::Chunked,
        manifest_state: TemplateManifestState::Approved,
        approved_at: Some(11),
        created_at: 10,
    };
    let request = TemplateChunkSetPrepareInput {
        manifest: Some(manifest.clone()),
        template_id: manifest.template_id.clone(),
        version: manifest.version.clone(),
        payload_hash: manifest.payload_hash.clone(),
        payload_size_bytes: manifest.payload_size_bytes,
        chunk_hashes: vec![vec![2; 32], vec![3; 32]],
    };
    let frame =
        ic_wire::native::encode_request(prepare::METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    let decoded: TemplateChunkSetPrepareInput =
        ic_wire::native::decode_request(prepare::METHOD_ID, &frame, REQUEST_LIMITS).unwrap();
    assert_eq!(decoded, request);
    assert!(matches!(
        ic_wire::native::decode_request::<TemplateChunkSetPrepareInput>(
            stage::METHOD_ID,
            &frame,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
    let mut trailing = frame;
    trailing.push(0);
    assert!(matches!(
        ic_wire::native::decode_request::<TemplateChunkSetPrepareInput>(
            prepare::METHOD_ID,
            &trailing,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Trailing)
    ));
    let stage =
        ic_wire::native::encode_request(stage::METHOD_ID, &manifest, REQUEST_LIMITS).unwrap();
    let decoded: TemplateManifestInput =
        ic_wire::native::decode_request(stage::METHOD_ID, &stage, REQUEST_LIMITS).unwrap();
    assert_eq!(decoded, manifest);
}

#[test]
fn fixture_preparation_keeps_store_refusals_distinct_from_endpoint_refusals() {
    for reply in [
        Ok::<Result<FixtureSourceStatus, FixtureStoreError>, Error>(Err(
            FixtureStoreError::Conflict,
        )),
        Err(Error::from_registered(
            crate::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        )),
    ] {
        let frame =
            ic_wire::native::encode_response(fixture_prepare::METHOD_ID, &reply, RESPONSE_LIMITS)
                .unwrap();
        let decoded: Result<Result<FixtureSourceStatus, FixtureStoreError>, Error> =
            ic_wire::native::decode_response(fixture_prepare::METHOD_ID, &frame, RESPONSE_LIMITS)
                .unwrap();
        assert_eq!(decoded, reply);
    }
}

#[test]
fn gc_target_and_operation_identity_survive_the_native_request() {
    for target in [WasmStoreGcTarget::Prepared, WasmStoreGcTarget::Complete] {
        let request = WasmStoreGcRequest {
            operation_id: [4; 32],
            target,
        };
        let frame = ic_wire::native::encode_request(
            crate::protocol::store_gc::METHOD_ID,
            &request,
            REQUEST_LIMITS,
        )
        .unwrap();
        let decoded: WasmStoreGcRequest = ic_wire::native::decode_request(
            crate::protocol::store_gc::METHOD_ID,
            &frame,
            REQUEST_LIMITS,
        )
        .unwrap();
        assert_eq!(decoded, request);
    }
}

#[cfg(feature = "wire-schema")]
#[test]
fn all_control_graphs_have_native_metadata() {
    ic_wire::typescript::native_schema::<TemplateLookupRequest>().unwrap();
    ic_wire::typescript::native_schema::<TemplateChunkSetPrepareInput>().unwrap();
    ic_wire::typescript::native_schema::<TemplateManifestInput>().unwrap();
    ic_wire::typescript::native_schema::<FixtureDescriptor>().unwrap();
    ic_wire::typescript::native_schema::<WasmStoreGcRequest>().unwrap();
    ic_wire::typescript::native_schema::<Result<TemplateChunkSetInfoResponse, Error>>().unwrap();
    ic_wire::typescript::native_schema::<
        Result<Result<FixtureSourceStatus, FixtureStoreError>, Error>,
    >()
    .unwrap();
    ic_wire::typescript::native_schema::<Result<crate::dto::role::OperationReceipt, Error>>()
        .unwrap();
}
