//! Native payload identity and codec bounds for public responsiveness.

use super::*;
use crate::dto::public_status::{PublicHealth, PublicHealthStatus};

#[test]
fn public_health_round_trips_native_identity_and_refuses_wrong_method() {
    let request = ic_wire::native::encode_request(METHOD_ID, &(), REQUEST_LIMITS).unwrap();
    ic_wire::native::decode_request::<()>(METHOD_ID, &request, REQUEST_LIMITS).unwrap();
    let value = PublicHealth {
        canister_id: ic_platform::Principal::anonymous(),
        role: Some("fleet_coordinator".into()),
        health: PublicHealthStatus::Responding,
        observed_at_ns: u64::MAX,
    };
    let bytes = ic_wire::native::encode_response(METHOD_ID, &value, RESPONSE_LIMITS).unwrap();
    let decoded: PublicHealth =
        ic_wire::native::decode_response(METHOD_ID, &bytes, RESPONSE_LIMITS).unwrap();
    assert_eq!(decoded.canister_id, value.canister_id);
    assert_eq!(decoded.role, value.role);
    assert_eq!(decoded.health, value.health);
    assert_eq!(decoded.observed_at_ns, value.observed_at_ns);
    assert!(matches!(
        ic_wire::native::decode_response::<PublicHealth>(METHOD_ID + 1, &bytes, RESPONSE_LIMITS),
        Err(ic_wire::Error::Identity)
    ));
}

#[cfg(feature = "wire-schema")]
#[test]
fn public_health_projects_the_actual_native_payload() {
    let schema = ic_wire::typescript::native_schema::<PublicHealth>().unwrap();
    let fields = schema["fields"].as_array().unwrap();
    assert_eq!(
        fields
            .iter()
            .map(|field| field["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["canister_id", "role", "health", "observed_at_ns"]
    );
    assert_eq!(
        fields[0]["value"],
        ic_wire::typescript::native_schema::<ic_platform::Principal>().unwrap()
    );
}
