//! Real Fleet/Component identifiers shared by directory authority, with unchanged storage text.

use canic_contracts::ids::{ComponentInstanceId, ComponentSpecId, FleetBinding};
use ic_wire::{Limits, native, typescript};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Test-only aggregate of actual authority fields, without introducing an endpoint.
#[derive(Debug, Deserialize, PartialEq, Serialize, ic_wire::Wire)]
struct AuthorityIdentifiers {
    fleet: FleetBinding,
    component: ComponentInstanceId,
    component_spec: ComponentSpecId,
}

pub fn method(base: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let mut method = base.clone();
    method["name"] = json!("identities");
    method["id"] = json!("21");
    method["requestAllocation"] = json!(32768);
    method["responseAllocation"] = json!(32768);
    let schema = typescript::native_schema::<AuthorityIdentifiers>()
        .map_err(|error| format!("identity schema: {error:?}"))?;
    method["request"] = schema.clone();
    method["response"] = schema;
    Ok(method)
}

pub fn vectors(limits: Limits) -> Result<Value, Box<dyn std::error::Error>> {
    let limits = Limits {
        allocation: 32768,
        ..limits
    };
    let value = json!({
        "fleet": { "fleet": { "canonical_network_id": "ab".repeat(32),
            "fleet_id": "cd".repeat(32) }, "app": "sample" },
        "component": "ef".repeat(32), "component_spec": "Users_01-test"
    });
    let input: AuthorityIdentifiers = serde_json::from_value(value.clone())?;
    let request =
        native::encode_request(21, &input, limits).map_err(|e| format!("encode request {e:?}"))?;
    let response = native::encode_response(21, &input, limits)
        .map_err(|e| format!("encode response {e:?}"))?;
    assert_eq!(
        native::decode_request::<AuthorityIdentifiers>(21, &request, limits)
            .map_err(|e| format!("decode request {e:?}"))?,
        input
    );
    assert_eq!(
        native::decode_response::<AuthorityIdentifiers>(21, &response, limits)
            .map_err(|e| format!("decode response {e:?}"))?,
        input
    );
    let mut invalid = Vec::new();
    for field in ["fleet", "component", "component_spec"] {
        let cases = if field == "component_spec" {
            vec![
                String::new(),
                "a".repeat(41),
                "users/child".into(),
                "users\n".into(),
                "é".into(),
            ]
        } else {
            vec![
                "AB".repeat(32),
                "a".repeat(63),
                "a".repeat(65),
                "gg".repeat(32),
                format!("{}\n", "ab".repeat(32)),
            ]
        };
        for bad in cases {
            let fleet = if field == "fleet" {
                bad.clone()
            } else {
                "cd".repeat(32)
            };
            let component = if field == "component" {
                bad.clone()
            } else {
                "ef".repeat(32)
            };
            let spec = if field == "component_spec" {
                bad.clone()
            } else {
                "Users_01-test".into()
            };
            // A tuple supplies the native field sequence without invoking validated constructors.
            let raw = ((("ab".repeat(32), fleet), "sample"), component, spec);
            let frame = native::encode_response(21, &raw, limits)?;
            assert!(native::decode_response::<AuthorityIdentifiers>(21, &frame, limits).is_err());
            invalid.push(json!({"field":field,"value":bad,"frame":frame}));
        }
    }
    Ok(json!({"value":value,"request":request,"response":response,"invalid":invalid}))
}
