//! Qualify generated Result codecs against the existing lossless diagnostic payload.

use canic_contracts::dto::error::Error;
use ic_wire::{Limits, native, typescript};
use serde_json::{Value, json};

pub fn method(base: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let mut method = base.clone();
    method["name"] = json!("diagnostic");
    method["id"] = json!("19");
    method["request"] = typescript::native_schema::<bool>()
        .map_err(|error| format!("diagnostic request schema: {error:?}"))?;
    method["response"] = typescript::native_schema::<Result<u64, Error>>()
        .map_err(|error| format!("diagnostic response schema: {error:?}"))?;
    Ok(method)
}

pub fn vectors(limits: Limits) -> Result<Value, Box<dyn std::error::Error>> {
    let mut errors = Vec::new();
    for code in [0_u16, 1, u16::MAX] {
        // Decoding preserves unknown codes; production issuance still requires registration.
        let error: Error = native::decode(&native::encode(&code, limits)?, limits)?;
        assert_eq!(error.raw_code(), code);
        let response: Result<u64, Error> = Err(error);
        let frame = native::encode_response(19, &response, limits)?;
        assert_eq!(
            native::decode_response::<Result<u64, Error>>(19, &frame, limits)?,
            response
        );
        errors.push(json!({"code":code,"frame":frame}));
    }
    let success: Result<u64, Error> = Ok(u64::MAX);
    Ok(json!({
        "request":native::encode_request(19, &false, limits)?,
        "success":native::encode_response(19, &success, limits)?,
        "errors":errors,
    }))
}
