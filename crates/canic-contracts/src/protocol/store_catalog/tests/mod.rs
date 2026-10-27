//! Typed reads preserve progress and distinguish absence from a Store refusal.
use super::*;
use crate::dto::{
    error::Error,
    fixture_provisioning::{FixtureSourceStatus, FixtureStoreError},
    template::TemplateStagingStatusResponse,
};

#[test]
fn fixture_status_preserves_progress_and_typed_absence() {
    for reply in [
        Ok::<_, Error>(Ok(FixtureSourceStatus {
            content_id: [8; 32],
            next_chunk: 3,
            chunk_count: 5,
            received_bytes: 999,
            complete: false,
        })),
        Ok(Err(FixtureStoreError::NotFound)),
    ] {
        let frame =
            ic_wire::native::encode_response(fixture::METHOD_ID, &reply, RESPONSE_LIMITS).unwrap();
        let decoded: Result<Result<FixtureSourceStatus, FixtureStoreError>, Error> =
            ic_wire::native::decode_response(fixture::METHOD_ID, &frame, RESPONSE_LIMITS).unwrap();
        assert_eq!(decoded, reply);
        assert!(matches!(
            ic_wire::native::decode_response::<
                Result<Result<FixtureSourceStatus, FixtureStoreError>, Error>,
            >(grant::METHOD_ID, &frame, RESPONSE_LIMITS),
            Err(ic_wire::Error::Identity)
        ));
    }
}

#[test]
fn staging_retains_partial_and_conflicting_chunk_evidence() {
    let status = TemplateStagingStatusResponse {
        template_id: "fixture".into(),
        version: "current".into(),
        manifest: None,
        chunk_set_present: true,
        expected_chunk_count: 3,
        expected_chunk_hashes: vec![vec![1; 32], vec![2; 32], vec![3; 32]],
        payload_hash: Some(vec![4; 32]),
        payload_size_bytes: Some(456),
        stored_chunk_hashes: vec![Some(vec![1; 32]), None, Some(vec![9; 32])],
        stored_chunk_count: 2,
        complete: false,
    };
    let mut frame =
        ic_wire::native::encode_response(template::METHOD_ID, &status, RESPONSE_LIMITS).unwrap();
    let decoded: TemplateStagingStatusResponse =
        ic_wire::native::decode_response(template::METHOD_ID, &frame, RESPONSE_LIMITS).unwrap();
    assert_eq!(decoded, status);
    frame.push(0);
    assert!(matches!(
        ic_wire::native::decode_response::<TemplateStagingStatusResponse>(
            template::METHOD_ID,
            &frame,
            RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Trailing)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn all_catalog_reads_have_native_graphs() {
    ic_wire::typescript::native_schema::<ic_platform::Principal>().unwrap();
    ic_wire::typescript::native_schema::<crate::dto::template::TemplateLookupRequest>().unwrap();
    ic_wire::typescript::native_schema::<Result<TemplateStagingStatusResponse, Error>>().unwrap();
    ic_wire::typescript::native_schema::<
        Result<Option<crate::dto::runtime_reporting::FixtureGrant>, Error>,
    >()
    .unwrap();
    ic_wire::typescript::native_schema::<
        Result<Result<FixtureSourceStatus, FixtureStoreError>, Error>,
    >()
    .unwrap();
}
