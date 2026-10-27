//! Qualify actual template keys, full chunks and method-bound frames.

use super::*;
use crate::dto::{
    error::Error,
    template::{TemplateChunkRequest, TemplateChunkResponse},
};

#[test]
fn chunk_round_trip_preserves_full_payload_and_rejects_wrong_frames() {
    let request = TemplateChunkRequest {
        template_id: "app".into(),
        version: "current".into(),
        chunk_index: u32::MAX,
    };
    let bytes = ic_wire::native::encode_request(METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    assert_eq!(
        ic_wire::native::decode_request::<TemplateChunkRequest>(METHOD_ID, &bytes, REQUEST_LIMITS)
            .unwrap(),
        request
    );
    assert!(matches!(
        ic_wire::native::decode_request::<TemplateChunkRequest>(
            METHOD_ID + 1,
            &bytes,
            REQUEST_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
    let expected = vec![0xab; 1_048_576];
    let response: Result<TemplateChunkResponse, Error> = Ok(TemplateChunkResponse {
        bytes: expected.clone(),
    });
    let mut bytes =
        ic_wire::native::encode_response(METHOD_ID, &response, RESPONSE_LIMITS).unwrap();
    assert!(bytes.len() < 1_048_640);
    let decoded: Result<TemplateChunkResponse, Error> =
        ic_wire::native::decode_response(METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    assert_eq!(decoded.unwrap().bytes, expected);
    bytes.push(0);
    assert!(
        ic_wire::native::decode_response::<Result<TemplateChunkResponse, Error>>(
            METHOD_ID,
            &bytes,
            RESPONSE_LIMITS
        )
        .is_err()
    );
}

#[test]
fn chunk_reply_refuses_frames_above_its_byte_budget() {
    let response: Result<TemplateChunkResponse, Error> = Ok(TemplateChunkResponse {
        bytes: vec![0xff; RESPONSE_LIMITS.bytes],
    });
    assert!(matches!(
        ic_wire::native::encode_response(METHOD_ID, &response, RESPONSE_LIMITS),
        Err(ic_wire::Error::Limit)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn chunk_schema_projects_actual_template_keys() {
    let schema = ic_wire::typescript::native_schema::<TemplateChunkRequest>().unwrap();
    let fields = schema["fields"].as_array().unwrap();
    assert_eq!(
        fields
            .iter()
            .map(|field| field["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["template_id", "version", "chunk_index"]
    );
    let response = ic_wire::typescript::native_schema::<TemplateChunkResponse>().unwrap();
    assert_eq!(response["fields"][0]["value"]["k"], "bytes");
    ic_wire::typescript::native_schema::<Result<TemplateChunkResponse, Error>>().unwrap();
}
