use super::{
    parse_canic_metadata_response, parse_canic_metadata_version_response, verify_overview_binding,
};
use crate::protocol_binding::RegistryProtocolBinding;
use canic_contracts::{
    dto::{
        error::Error,
        metadata::CanicMetadataResponse,
        role::{RoleCapability, RoleOverviewResponse},
        state::BootstrapStatusResponse,
    },
    ids::CanisterRole,
};
use canic_core::{
    cdk::utils::hash::hex_bytes,
    role_contract::{ProtocolProfileDigest, RoleCapabilityKey},
};

use std::collections::BTreeSet;

#[test]
fn parses_metadata_version_from_typed_response_bytes() {
    let output = response_hex(&Ok::<_, Error>(RoleOverviewResponse {
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
    }));

    assert_eq!(
        parse_canic_metadata_version_response(&output).expect("decode metadata"),
        "0.93.4"
    );
}

#[test]
fn overview_only_verifies_an_already_selected_binding() {
    let output = response_hex(&Ok::<_, Error>(RoleOverviewResponse {
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
    }));
    let overview = parse_canic_metadata_response(&output).expect("decode Overview");
    let mut selected = RegistryProtocolBinding {
        release_identity: "0.93.4".to_string(),
        role: CanisterRole::from("app"),
        capabilities: BTreeSet::from([RoleCapabilityKey::ChildProvisioning]),
        wire_schema_sha256: [2; 32],
        protocol_profile_digest: ProtocolProfileDigest::from_bytes([1; 32]),
    };
    verify_overview_binding(&overview, &selected).expect("exact selected binding");

    selected.capabilities.clear();
    assert!(verify_overview_binding(&overview, &selected).is_err());
    selected
        .capabilities
        .insert(RoleCapabilityKey::ChildProvisioning);
    selected.release_identity = "0.93.5".to_string();
    assert!(verify_overview_binding(&overview, &selected).is_err());
}

fn response_hex<T: serde::Serialize>(response: &T) -> String {
    let bytes = ic_wire::native::encode_response(
        super::public_overview::METHOD_ID,
        response,
        super::public_overview::RESPONSE_LIMITS,
    )
    .expect("encode response");
    hex_bytes(bytes)
}

#[test]
fn raw_overview_rejects_wrong_method_and_preserves_remote_error() {
    let error = Error::from_registered(canic_contracts::diagnostics::codes::REQUEST_CAPACITY);
    let reply: Result<RoleOverviewResponse, Error> = Err(error);
    let output = response_hex(&reply);
    assert!(matches!(
        parse_canic_metadata_response(&output),
        Err(super::CanicMetadataQueryError::Response(crate::icp::IcpJsonResponseError::Rejected(actual))) if actual == error
    ));
    let wrong_method = ic_wire::native::encode_response(
        super::public_overview::METHOD_ID + 1,
        &reply,
        super::public_overview::RESPONSE_LIMITS,
    )
    .unwrap();
    assert!(matches!(
        parse_canic_metadata_response(&hex_bytes(wrong_method)),
        Err(super::CanicMetadataQueryError::Wire(
            ic_wire::Error::Identity
        ))
    ));
}
