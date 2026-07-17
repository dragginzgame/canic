use super::{IcpJsonResponseError, decode_json_response, decode_json_result_response};
use candid::Encode;
use canic_core::{
    cdk::utils::hash::hex_bytes,
    dto::error::{Error as CanicError, ErrorCode},
};

#[test]
fn decodes_plain_typed_response_bytes() {
    let output = response_json(&42_u64);

    assert_eq!(
        decode_json_response::<u64>(&output).expect("decode value"),
        42
    );
}

#[test]
fn decodes_successful_typed_result_response_bytes() {
    let output = response_json(&Ok::<u64, CanicError>(42));

    assert_eq!(
        decode_json_result_response::<u64>(&output).expect("decode result"),
        42
    );
}

#[test]
fn preserves_typed_canister_rejection() {
    let output = response_json(&Err::<u64, _>(CanicError::forbidden("denied")));
    let error = decode_json_result_response::<u64>(&output).expect_err("reject result");

    let IcpJsonResponseError::Rejected(error) = error else {
        panic!("expected typed canister rejection");
    };
    assert_eq!(error.code, ErrorCode::Forbidden);
    assert_eq!(error.message, "denied");
}

#[test]
fn requires_top_level_string_response_bytes() {
    for output in [r"{}", r#"{"response_bytes":null}"#] {
        assert!(matches!(
            decode_json_response::<u64>(output),
            Err(IcpJsonResponseError::MissingResponseBytes)
        ));
    }
}

#[test]
fn rejects_invalid_hex_and_candid() {
    assert!(matches!(
        decode_json_response::<u64>(r#"{"response_bytes":"not-hex"}"#),
        Err(IcpJsonResponseError::Hex(_))
    ));
    assert!(matches!(
        decode_json_response::<u64>(r#"{"response_bytes":"00"}"#),
        Err(IcpJsonResponseError::Candid(_))
    ));
}

fn response_json<T: candid::CandidType>(response: &T) -> String {
    let bytes = Encode!(response).expect("encode response");
    serde_json::json!({
        "response_bytes": hex_bytes(bytes),
        "response_text": null,
        "response_candid": "scripted",
    })
    .to_string()
}
