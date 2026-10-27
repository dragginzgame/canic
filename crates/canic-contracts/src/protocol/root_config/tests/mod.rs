//! Exact text preservation and bounded query frames.

use super::*;
use crate::dto::{error::Error, role::ConfigStatusResponse};

#[test]
fn config_preserves_source_text_and_frame_identity() {
    let request = ic_wire::native::encode_request(METHOD_ID, &(), REQUEST_LIMITS).unwrap();
    ic_wire::native::decode_request::<()>(METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    let response: Result<_, Error> = Ok(ConfigStatusResponse {
        toml: "# retained comment\n[app]\nname = \"café\"\n".into(),
    });
    let bytes = ic_wire::native::encode_response(METHOD_ID, &response, RESPONSE_LIMITS).unwrap();
    let actual: Result<ConfigStatusResponse, Error> =
        ic_wire::native::decode_response(METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    assert_eq!(actual, response);
    assert!(matches!(
        ic_wire::native::decode_response::<Result<ConfigStatusResponse, Error>>(
            METHOD_ID + 1,
            &bytes,
            RESPONSE_LIMITS,
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[test]
fn config_refuses_oversized_source() {
    let response: Result<_, Error> = Ok(ConfigStatusResponse {
        toml: "x".repeat(RESPONSE_LIMITS.bytes),
    });
    assert!(matches!(
        ic_wire::native::encode_response(METHOD_ID, &response, RESPONSE_LIMITS),
        Err(ic_wire::Error::Limit)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn config_schema_describes_the_complete_result() {
    ic_wire::typescript::native_schema::<Result<ConfigStatusResponse, Error>>().unwrap();
}
