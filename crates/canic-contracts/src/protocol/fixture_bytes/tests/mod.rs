//! Full-size opaque fixture frames stay bounded and retain typed refusal boundaries.
use super::*;
use crate::dto::{
    error::Error,
    fixture_bytes::{ChunkResponse, UploadRequest},
    fixture_provisioning::{FixtureChunkUpload, FixtureStoreError},
};

#[test]
fn full_chunk_round_trips_without_changing_canonical_upload_data() {
    let original = FixtureChunkUpload {
        content_id: [8; 32],
        index: u32::MAX,
        bytes: vec![0xab; 1_048_576],
    };
    let request = UploadRequest::from(original.clone());
    let frame = ic_wire::native::encode_request(upload::METHOD_ID, &request, CHUNK_LIMITS).unwrap();
    assert!(frame.len() < 1_048_704);
    let decoded: UploadRequest =
        ic_wire::native::decode_request(upload::METHOD_ID, &frame, CHUNK_LIMITS).unwrap();
    assert_eq!(FixtureChunkUpload::from(decoded), original);
    assert!(matches!(
        ic_wire::native::decode_request::<UploadRequest>(read::METHOD_ID, &frame, CHUNK_LIMITS),
        Err(ic_wire::Error::Identity)
    ));
    let reply: Result<Result<ChunkResponse, FixtureStoreError>, Error> = Ok(Ok(ChunkResponse {
        bytes: original.bytes,
    }));
    let mut frame =
        ic_wire::native::encode_response(read::METHOD_ID, &reply, CHUNK_LIMITS).unwrap();
    let decoded: Result<Result<ChunkResponse, FixtureStoreError>, Error> =
        ic_wire::native::decode_response(read::METHOD_ID, &frame, CHUNK_LIMITS).unwrap();
    assert_eq!(decoded, reply);
    frame.push(0);
    assert!(matches!(
        ic_wire::native::decode_response::<Result<Result<ChunkResponse, FixtureStoreError>, Error>>(
            read::METHOD_ID,
            &frame,
            CHUNK_LIMITS
        ),
        Err(ic_wire::Error::Trailing)
    ));
}

#[test]
fn oversized_upload_is_refused_by_native_envelope() {
    let request = UploadRequest {
        content_id: [9; 32],
        index: 0,
        bytes: vec![0; CHUNK_LIMITS.bytes],
    };
    assert!(ic_wire::native::encode_request(upload::METHOD_ID, &request, CHUNK_LIMITS).is_err());
}

#[test]
fn chunk_authority_refusal_remains_distinct_from_public_rejection() {
    for reply in [
        Ok::<Result<ChunkResponse, FixtureStoreError>, Error>(Err(FixtureStoreError::Authority)),
        Err(Error::from_registered(
            crate::diagnostics::codes::AUTHORITY_UNAVAILABLE,
        )),
    ] {
        let frame =
            ic_wire::native::encode_response(read::METHOD_ID, &reply, CHUNK_LIMITS).unwrap();
        let decoded: Result<Result<ChunkResponse, FixtureStoreError>, Error> =
            ic_wire::native::decode_response(read::METHOD_ID, &frame, CHUNK_LIMITS).unwrap();
        assert_eq!(decoded, reply);
    }
}

#[cfg(feature = "wire-schema")]
#[test]
fn both_fixture_byte_lanes_have_native_metadata() {
    ic_wire::typescript::native_schema::<crate::dto::fixture_bytes::ReadRequest>().unwrap();
    ic_wire::typescript::native_schema::<UploadRequest>().unwrap();
    ic_wire::typescript::native_schema::<Result<Result<ChunkResponse, FixtureStoreError>, Error>>()
        .unwrap();
    ic_wire::typescript::native_schema::<
        Result<
            Result<crate::dto::fixture_provisioning::FixtureSourceStatus, FixtureStoreError>,
            Error,
        >,
    >()
    .unwrap();
}
