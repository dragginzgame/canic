//! Typed Overview framing, metadata fidelity and bounded rejection.

use super::*;
use crate::dto::{
    error::Error,
    metadata::CanicMetadataResponse,
    role::{RoleCapability, RoleOverviewResponse},
    state::BootstrapStatusResponse,
};

fn overview() -> RoleOverviewResponse {
    RoleOverviewResponse {
        role: "app".into(),
        capabilities: vec![RoleCapability::ChildProvisioning, RoleCapability::Runtime],
        protocol_profile_digest: [0xa5; 32],
        metadata: CanicMetadataResponse {
            package_name: "example".into(),
            package_version: "1.0.0".into(),
            package_description: "test metadata".into(),
            canic_version: "0.112.0".into(),
            canister_version: u64::MAX,
        },
        bootstrap: BootstrapStatusResponse {
            ready: false,
            phase: "pending".into(),
            last_error: Some("awaiting admission".into()),
        },
    }
}

#[test]
fn overview_preserves_identity_metadata_and_readiness() {
    let request = ic_wire::native::encode_request(METHOD_ID, &(), REQUEST_LIMITS).unwrap();
    ic_wire::native::decode_request::<()>(METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    let expected = overview();
    let response: Result<_, Error> = Ok(expected.clone());
    let bytes = ic_wire::native::encode_response(METHOD_ID, &response, RESPONSE_LIMITS).unwrap();
    let decoded: Result<RoleOverviewResponse, Error> =
        ic_wire::native::decode_response(METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    let actual = decoded.unwrap();
    assert_eq!(actual.role, expected.role);
    assert_eq!(actual.capabilities, expected.capabilities);
    assert_eq!(
        actual.protocol_profile_digest,
        expected.protocol_profile_digest
    );
    assert_eq!(actual.bootstrap, expected.bootstrap);
    assert_eq!(
        serde_json::to_value(actual.metadata).unwrap(),
        serde_json::to_value(expected.metadata).unwrap()
    );
    assert!(matches!(
        ic_wire::native::decode_response::<Result<RoleOverviewResponse, Error>>(
            METHOD_ID + 1,
            &bytes,
            RESPONSE_LIMITS
        ),
        Err(ic_wire::Error::Identity)
    ));
}

#[test]
fn overview_refuses_oversized_diagnostics() {
    let mut value = overview();
    value.bootstrap.last_error = Some("x".repeat(RESPONSE_LIMITS.bytes));
    assert!(matches!(
        ic_wire::native::encode_response(METHOD_ID, &Ok::<_, Error>(value), RESPONSE_LIMITS),
        Err(ic_wire::Error::Limit)
    ));
}

#[test]
fn overview_preserves_remote_error_code() {
    let error: Error = serde_json::from_value(serde_json::json!({"code": 65535})).unwrap();
    let bytes = ic_wire::native::encode_response(
        METHOD_ID,
        &Err::<RoleOverviewResponse, _>(error),
        RESPONSE_LIMITS,
    )
    .unwrap();
    let decoded: Result<RoleOverviewResponse, Error> =
        ic_wire::native::decode_response(METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    assert_eq!(decoded.unwrap_err(), error);
}

#[cfg(feature = "wire-schema")]
#[test]
fn overview_schema_uses_the_actual_role_text() {
    let schema = ic_wire::typescript::native_schema::<RoleOverviewResponse>().unwrap();
    let fields = schema["fields"].as_array().unwrap();
    assert_eq!(fields[0]["name"], "role");
    assert_eq!(fields[0]["value"]["k"], "text");
    ic_wire::typescript::native_schema::<Result<RoleOverviewResponse, Error>>().unwrap();
}
