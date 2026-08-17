use super::{RoleStatusResponse, parse_canic_metadata_version_response};
use candid::{CandidType, Encode};
use canic_core::{
    cdk::utils::hash::hex_bytes,
    dto::{
        error::Error, metadata::CanicMetadataResponse, role::RoleOverviewResponse,
        state::BootstrapStatusResponse,
    },
    ids::CanisterRole,
};

#[test]
fn parses_metadata_version_from_typed_response_bytes() {
    let output = response_json(&Ok::<_, Error>(RoleStatusResponse::Overview(
        RoleOverviewResponse {
            role: CanisterRole::from("app"),
            capabilities: Vec::new(),
            protocol_profile_digest: [1; 32],
            metadata: CanicMetadataResponse {
                package_name: "example".to_string(),
                package_version: "1.2.3".to_string(),
                package_description: "example canister".to_string(),
                canic_version: "0.93.4".to_string(),
                canister_version: 7,
            },
            bootstrap: BootstrapStatusResponse {
                ready: true,
                phase: "ready".to_string(),
                last_error: None,
            },
        },
    )));

    assert_eq!(
        parse_canic_metadata_version_response(&output).expect("decode metadata"),
        "0.93.4"
    );
}

fn response_json<T: CandidType>(response: &T) -> String {
    let bytes = Encode!(response).expect("encode response");
    serde_json::json!({ "response_bytes": hex_bytes(bytes) }).to_string()
}
