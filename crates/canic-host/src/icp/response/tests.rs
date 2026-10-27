use super::{IcpJsonResponseError, decode_json_response};
use canic_contracts::{diagnostics::codes, dto::error::Error as CanicError};
use canic_core::cdk::utils::hash::hex_bytes;
use ic_host_tools::response::{JsonErrorKind, ResponseError};
use ic_platform::candid::contract::Encode;

#[test]
fn decodes_plain_typed_response_bytes() {
    let output = response_json(&42_u64);

    assert_eq!(
        decode_json_response::<u64>(&output).expect("decode value"),
        42
    );
}

#[test]
fn preserves_typed_canister_rejection() {
    let error =
        IcpJsonResponseError::Rejected(CanicError::from_registered(codes::AUTHORITY_UNAUTHORIZED));
    let IcpJsonResponseError::Rejected(error) = error else {
        panic!("expected typed canister rejection");
    };
    assert_eq!(error.code(), codes::AUTHORITY_UNAUTHORIZED.raw_code());

    let rendered = IcpJsonResponseError::Rejected(error).to_string();
    assert!(rendered.contains("canister rejected request: E30 AUTHORITY_UNAUTHORIZED"));
    assert!(rendered.contains("origin: topology_authority"));
    assert!(rendered.contains("Authority is unauthorized."));
}

#[test]
fn requires_top_level_string_response_bytes() {
    for output in [r"{}", r#"{"response_bytes":null}"#] {
        assert!(matches!(
            decode_json_response::<u64>(output),
            Err(IcpJsonResponseError::Envelope(
                ResponseError::MissingResponseBytes
            ))
        ));
    }
}

#[test]
fn rejects_invalid_hex_and_candid() {
    assert!(matches!(
        decode_json_response::<u64>(r#"{"response_bytes":"not-hex"}"#),
        Err(IcpJsonResponseError::Envelope(ResponseError::InvalidHex {
            offset: 0
        }))
    ));
    assert!(matches!(
        decode_json_response::<u64>(r#"{"response_bytes":"00"}"#),
        Err(IcpJsonResponseError::Candid(_))
    ));
}

#[test]
fn envelope_errors_preserve_shared_categories_and_source() {
    for output in [
        r#"{"response_bytes":42}"#,
        r#"{"response_bytes":"00","response_bytes":"01"}"#,
    ] {
        let error = decode_json_response::<u64>(output).unwrap_err();
        assert!(matches!(
            error,
            IcpJsonResponseError::Envelope(ResponseError::Json {
                kind: JsonErrorKind::Data,
                ..
            })
        ));
        assert!(
            std::error::Error::source(&error)
                .unwrap()
                .downcast_ref::<ResponseError>()
                .is_some()
        );
    }
    assert!(matches!(
        decode_json_response::<u64>("not json"),
        Err(IcpJsonResponseError::Envelope(ResponseError::Json {
            kind: JsonErrorKind::Syntax,
            ..
        }))
    ));
    assert!(matches!(
        super::response_bytes(r#"{"response_bytes":"f"}"#),
        Err(IcpJsonResponseError::Envelope(ResponseError::OddHexLength))
    ));
    assert!(matches!(
        super::response_bytes(r#"{"response_bytes":" 00"}"#),
        Err(IcpJsonResponseError::Envelope(ResponseError::InvalidHex {
            offset: 0
        }))
    ));
    assert_eq!(
        super::response_bytes(r#"{"response_bytes":""}"#).unwrap(),
        Vec::<u8>::new()
    );
}

fn response_json<T: ic_platform::candid::contract::CandidType>(response: &T) -> String {
    let bytes = Encode!(response).expect("encode response");
    serde_json::json!({
        "response_bytes": hex_bytes(bytes),
        "response_text": null,
        "response_candid": "scripted",
    })
    .to_string()
}
