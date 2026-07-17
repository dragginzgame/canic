use super::parse_canic_metadata_version_response;
use candid::{CandidType, Encode};
use canic_core::{cdk::utils::hash::hex_bytes, dto::metadata::CanicMetadataResponse};

#[test]
fn parses_metadata_version_from_typed_response_bytes() {
    let output = response_json(&CanicMetadataResponse {
        package_name: "example".to_string(),
        package_version: "1.2.3".to_string(),
        package_description: "example canister".to_string(),
        canic_version: "0.93.4".to_string(),
        canister_version: 7,
    });

    assert_eq!(
        parse_canic_metadata_version_response(&output).expect("decode metadata"),
        "0.93.4"
    );
}

fn response_json<T: CandidType>(response: &T) -> String {
    let bytes = Encode!(response).expect("encode response");
    serde_json::json!({ "response_bytes": hex_bytes(bytes) }).to_string()
}
