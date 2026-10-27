//! External Candid and method-bound native codecs for funding recovery proofs.

#[cfg(test)]
mod tests;

use ic_platform::{
    candid::{
        contract::{ArgumentDecoder, CandidType, encode_one},
        decoder::{DecoderConfig, decode_args_with_config},
    },
    cdk::api::trap,
};
use serde::Deserialize;

/// Decode the argument list with the probe's existing CDK skipping quota.
pub fn decode_args<T: for<'de> ArgumentDecoder<'de>>(bytes: Vec<u8>) -> T {
    let mut config = DecoderConfig::new();
    config.set_skipping_quota(10_000);
    decode_args_with_config(&bytes, &config).unwrap_or_else(|error| trap(error.to_string()))
}

/// Unwrap a single Candid argument for Platform's scalar decoder hook.
pub fn decode_one<T: CandidType + for<'de> Deserialize<'de>>(bytes: Vec<u8>) -> T {
    let (value,) = decode_args(bytes);
    value
}

/// Encode the single reply value returned by every funding probe RPC.
pub fn encode_reply<T: CandidType>(value: T) -> Vec<u8> {
    encode_one(value).unwrap_or_else(|error| trap(error.to_string()))
}

macro_rules! owned_codec {
    ($module:ident, $id:ident, $request:ty, $response:ty) => {
        pub mod $module {
            pub fn decode(bytes: Vec<u8>) -> $request {
                ic_wire::native::decode_request(
                    crate::protocol::$id,
                    &bytes,
                    crate::protocol::LIMITS,
                )
                .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
            }
            pub fn encode(value: $response) -> Vec<u8> {
                ic_wire::native::encode_response(
                    crate::protocol::$id,
                    &value,
                    crate::protocol::LIMITS,
                )
                .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
            }
        }
    };
}
owned_codec!(
    prepare,
    PREPARE,
    crate::protocol::PrepareRequest,
    crate::GrantIntent
);
owned_codec!(
    handle_request,
    HANDLE_REQUEST,
    crate::GrantRequest,
    crate::IssueResult
);
owned_codec!(request, REQUEST, (), crate::RootRequestObservation);
owned_codec!(
    emergency_refill,
    EMERGENCY_REFILL,
    (),
    crate::protocol::EmergencyOperation
);
owned_codec!(
    emergency_refill_transfer_then_trap,
    EMERGENCY_REFILL_TRANSFER_THEN_TRAP,
    (),
    crate::protocol::EmergencyOperation
);
owned_codec!(
    emergency_refill_notify_then_trap,
    EMERGENCY_REFILL_NOTIFY_THEN_TRAP,
    (),
    crate::protocol::EmergencyOperation
);
owned_codec!(
    emergency_operation,
    EMERGENCY_OPERATION,
    (),
    Option<crate::protocol::EmergencyOperation>
);
owned_codec!(issue, ISSUE, (), crate::IssueResult);
owned_codec!(
    issue_root_accept_then_trap,
    ISSUE_ROOT_ACCEPT_THEN_TRAP,
    (),
    crate::IssueResult
);
owned_codec!(issue_then_trap, ISSUE_THEN_TRAP, (), crate::IssueResult);
owned_codec!(intent, INTENT, (), Option<crate::GrantIntent>);
owned_codec!(receipt, RECEIPT, (), Option<crate::GrantReceipt>);
owned_codec!(
    acceptance_observation,
    ACCEPTANCE_OBSERVATION,
    (),
    Option<crate::AcceptanceObservation>
);
owned_codec!(accept, ACCEPT, crate::GrantRequest, crate::RootAcceptResult);
owned_codec!(
    accept_then_trap,
    ACCEPT_THEN_TRAP,
    crate::GrantRequest,
    crate::RootAcceptResult
);
