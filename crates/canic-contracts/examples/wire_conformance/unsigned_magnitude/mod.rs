//! Browser evidence keeps exact arbitrary values and distinguishes missing from zero.
use canic_contracts::ops::unsigned_magnitude::{from_nat, to_nat};
use ic_platform::candid::contract::Nat;
use ic_wire::{Bytes, Limits, native, typescript};
use num_bigint::BigUint;
use serde_json::{Value, json};

pub fn method(base: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let mut method = base.clone();
    method["name"] = json!("unsignedMagnitudes");
    method["id"] = json!("71");
    let schema = typescript::native_schema::<Option<Bytes<64>>>()
        .map_err(|error| format!("unsigned magnitude schema: {error:?}"))?;
    method["request"] = schema.clone();
    method["response"] = schema;
    Ok(method)
}

pub fn vectors(limits: Limits) -> Result<Value, Box<dyn std::error::Error>> {
    let values = [
        None,
        Some(Nat::from(0_u8)),
        Some(Nat::from(u128::MAX)),
        Some(Nat(BigUint::from(1_u8) << 128)),
        Some(Nat(BigUint::from(1_u8) << 256)),
        Some(Nat((BigUint::from(1_u8) << 512) - BigUint::from(1_u8))),
    ];
    let mut cases = Vec::new();
    for value in values {
        let magnitude = value.as_ref().map(from_nat::<64>).transpose()?;
        assert_eq!(magnitude.as_ref().map(to_nat).transpose()?, value);
        cases.push(json!({
            "decimal": value.as_ref().map(|v| v.0.to_str_radix(10)),
            "bytes": magnitude.as_ref().map(|v| v.as_slice()),
            "request": native::encode_request(71, &magnitude, limits)?,
            "response": native::encode_response(71, &magnitude, limits)?,
        }));
    }
    Ok(json!(cases))
}
