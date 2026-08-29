use super::{
    RoleStatusResponse, parse_canic_metadata_response, parse_canic_metadata_version_response,
    verify_overview_binding,
};
use crate::protocol_binding::{RegistryProtocolBinding, ResolvedProtocolBinding};
use candid::{CandidType, Encode};
use canic_core::{
    cdk::utils::hash::hex_bytes,
    dto::{
        error::Error,
        metadata::CanicMetadataResponse,
        role::{RoleCapability, RoleOverviewResponse},
        state::BootstrapStatusResponse,
    },
    ids::CanisterRole,
    role_contract::{ProtocolProfileDigest, RoleCapabilityKey},
};
use std::{collections::BTreeSet, path::PathBuf};

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

#[test]
fn overview_only_verifies_an_already_selected_binding() {
    let output = response_json(&Ok::<_, Error>(RoleStatusResponse::Overview(
        RoleOverviewResponse {
            role: CanisterRole::from("app"),
            capabilities: vec![RoleCapability::ChildProvisioning],
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
    let overview = parse_canic_metadata_response(&output).expect("decode Overview");
    let mut selected = ResolvedProtocolBinding {
        binding: RegistryProtocolBinding {
            release_identity: "0.93.4".to_string(),
            role: CanisterRole::from("app"),
            capabilities: BTreeSet::from([RoleCapabilityKey::ChildProvisioning]),
            candid_sha256: [2; 32],
            protocol_profile_digest: ProtocolProfileDigest::from_bytes([1; 32]),
        },
        candid_path: PathBuf::from("app.did"),
    };
    verify_overview_binding(&overview, &selected).expect("exact selected binding");

    selected.binding.release_identity = "0.93.5".to_string();
    assert!(verify_overview_binding(&overview, &selected).is_err());
}

fn response_json<T: CandidType>(response: &T) -> String {
    let bytes = Encode!(response).expect("encode response");
    serde_json::json!({ "response_bytes": hex_bytes(bytes) }).to_string()
}
