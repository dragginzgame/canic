//! Consumer-owned envelope qualifying Platform Account without an application endpoint.

use ic_platform::{Account, Principal};
use ic_wire::{Limits, native, typescript};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Test-only payload; ledger identity and authorization remain outside Account.
#[derive(Debug, Deserialize, Serialize, ic_wire::Wire)]
struct AccountEnvelope {
    account: Account,
}

pub fn method(base: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let mut method = base.clone();
    method["name"] = json!("accounts");
    method["id"] = json!("20");
    method["requestNodes"] = json!(256);
    method["responseNodes"] = json!(256);
    let schema = typescript::native_schema::<AccountEnvelope>()
        .map_err(|error| format!("account schema: {error:?}"))?;
    method["request"] = schema.clone();
    method["response"] = schema;
    Ok(method)
}

pub fn vectors(limits: Limits) -> Result<Value, Box<dyn std::error::Error>> {
    let limits = Limits {
        nodes: 256,
        ..limits
    };
    let mut vectors = Vec::new();
    for owner in [
        Principal::management_canister(),
        Principal::anonymous(),
        Principal::from_slice(&[42; 29]),
    ] {
        let absent = Account {
            owner,
            subaccount: None,
        };
        let explicit_zero = Account {
            owner,
            subaccount: Some([0; 32]),
        };
        assert_eq!(absent, explicit_zero);
        assert_ne!(
            native::encode(&absent, limits)?,
            native::encode(&explicit_zero, limits)?
        );
        for subaccount in [None, Some([0; 32]), Some([17; 32])] {
            let input = AccountEnvelope {
                account: Account { owner, subaccount },
            };
            let request = native::encode_request(20, &input, limits)?;
            let response = native::encode_response(20, &input, limits)?;
            for decoded in [
                native::decode_request::<AccountEnvelope>(20, &request, limits)?,
                native::decode_response::<AccountEnvelope>(20, &response, limits)?,
            ] {
                assert_eq!(decoded.account.owner, owner);
                // Account equality alone would conceal unwanted zero normalization.
                assert_eq!(decoded.account.subaccount, subaccount);
            }
            vectors.push(json!({"owner":owner.as_slice(), "subaccount":subaccount, "request":request, "response":response}));
        }
    }
    // A Serde tuple mirrors the field sequence but supplies an invalid owner length.
    let invalid_owner = ((vec![0_u8; 30], Option::<[u8; 32]>::None),);
    let malformed_request = native::encode_request(20, &invalid_owner, limits)?;
    let malformed_response = native::encode_response(20, &invalid_owner, limits)?;
    assert!(native::decode_request::<AccountEnvelope>(20, &malformed_request, limits).is_err());
    assert!(native::decode_response::<AccountEnvelope>(20, &malformed_response, limits).is_err());
    Ok(
        json!({"cases":vectors, "malformedRequest":malformed_request, "malformedResponse":malformed_response}),
    )
}
