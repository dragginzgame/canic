//! Root observations preserve publication, GC and content evidence across native frames.
use super::*;
use crate::{
    dto::{
        error::Error,
        fixture_provisioning::{FixtureSourceStatus, FixtureStoreError},
        root_store_reporting::{Overview, Store},
        template::{
            WasmStoreGcStatusResponse, WasmStoreOverviewResponse, WasmStorePublicationSlotResponse,
            WasmStorePublicationStateResponse, WasmStoreTemplateStatusResponse,
        },
    },
    ids::WasmStoreGcMode,
};

fn store() -> Store {
    Store {
        binding: "binding".into(),
        pid: ic_platform::Principal::from_slice(&[7]),
        created_at: 123,
        publication_slot: Some(WasmStorePublicationSlotResponse::Detached),
        gc: WasmStoreGcStatusResponse {
            mode: WasmStoreGcMode::InProgress,
            changed_at: 124,
            prepared_at: Some(125),
            started_at: Some(126),
            completed_at: None,
            runs_completed: 2,
        },
        approved_payload_bytes: 128,
        approved_payload_size: "128 B".into(),
        max_store_bytes: 256,
        max_store_size: "256 B".into(),
        remaining_approved_payload_bytes: 64,
        remaining_approved_payload_size: "64 B".into(),
        headroom_bytes: Some(128),
        headroom_size: Some("128 B".into()),
        within_approved_headroom: true,
        approved_template_count: 1,
        max_templates: Some(2),
        approved_release_count: 3,
        max_template_versions_per_template: Some(4),
        approved_templates: vec![WasmStoreTemplateStatusResponse {
            template_id: "template".into(),
            versions: 3,
        }],
    }
}

#[test]
fn overview_preserves_complete_publication_and_store_evidence() {
    let expected = Overview {
        publication: WasmStorePublicationStateResponse {
            active_binding: Some("active".into()),
            detached_binding: Some("binding".into()),
            retired_binding: Some("retired".into()),
            generation: u64::MAX,
            changed_at: 127,
            retired_at: 128,
        },
        stores: vec![store()],
    };
    let canonical = WasmStoreOverviewResponse::from(expected.clone());
    assert_eq!(Overview::from(canonical), expected);
    let reply: Result<_, Error> = Ok(expected.clone());
    let mut bytes =
        ic_wire::native::encode_response(overview::METHOD_ID, &reply, RESPONSE_LIMITS).unwrap();
    let observed: Result<Overview, Error> =
        ic_wire::native::decode_response(overview::METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    assert_eq!(observed.unwrap(), expected);
    assert!(matches!(
        ic_wire::native::decode_response::<Result<Overview, Error>>(
            fixture::METHOD_ID,
            &bytes,
            RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
    bytes.push(0);
    assert!(matches!(
        ic_wire::native::decode_response::<Result<Overview, Error>>(
            overview::METHOD_ID,
            &bytes,
            RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Trailing)
    ));
}

#[test]
fn fixture_progress_keeps_absence_distinct_from_authority_refusal() {
    for reply in [
        Ok(Ok(FixtureSourceStatus {
            content_id: [9; 32],
            next_chunk: 1,
            chunk_count: 2,
            received_bytes: 31,
            complete: false,
        })),
        Ok(Err(FixtureStoreError::NotFound)),
        Err(Error::from_registered(
            crate::diagnostics::codes::AUTHORITY_UNAVAILABLE,
        )),
    ] {
        let bytes =
            ic_wire::native::encode_response(fixture::METHOD_ID, &reply, RESPONSE_LIMITS).unwrap();
        let observed: Result<Result<FixtureSourceStatus, FixtureStoreError>, Error> =
            ic_wire::native::decode_response(fixture::METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
        assert_eq!(observed, reply);
    }
}

#[cfg(feature = "wire-schema")]
#[test]
fn both_reporting_graphs_have_native_metadata() {
    ic_wire::typescript::native_schema::<Result<Overview, Error>>().unwrap();
    ic_wire::typescript::native_schema::<
        Result<Result<FixtureSourceStatus, FixtureStoreError>, Error>,
    >()
    .unwrap();
}
