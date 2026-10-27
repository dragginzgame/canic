//! External Candid ledger codecs and method-bound native fixture controls.

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

/// Decode the argument list with the fixture's existing CDK skipping quota.
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

/// Encode the single reply value returned by every fixture RPC.
pub fn encode_reply<T: CandidType>(value: T) -> Vec<u8> {
    encode_one(value).unwrap_or_else(|error| trap(error.to_string()))
}

// Method-bound native controls do not participate in external ledger codecs.
macro_rules! control_codec {
    ($module:ident, $id:ident, $request:ty, ()) => {
        pub mod $module {
            pub fn decode(bytes: Vec<u8>) -> $request {
                ic_wire::native::decode_request(
                    crate::protocol::$id,
                    &bytes,
                    crate::protocol::LIMITS,
                )
                .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
            }
            pub fn encode() -> Vec<u8> {
                ic_wire::native::encode_response(crate::protocol::$id, &(), crate::protocol::LIMITS)
                    .unwrap_or_else(|error| ic_platform::cdk::api::trap(error.to_string()))
            }
        }
    };
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
control_codec!(
    set_transfer_fee_override,
    SET_TRANSFER_FEE_OVERRIDE,
    Option<u128>,
    ()
);
control_codec!(lose_next_transfer_reply, LOSE_NEXT_TRANSFER_REPLY, (), ());
control_codec!(withdrawal_count, WITHDRAWAL_COUNT, (), u64);
control_codec!(
    set_creation_retry_refusal,
    SET_CREATION_RETRY_REFUSAL,
    Option<crate::CreationRetryRefusal>,
    ()
);
control_codec!(transfer_count, TRANSFER_COUNT, (), u64);
control_codec!(request_count, REQUEST_COUNT, (), u64);
control_codec!(
    requested_amounts,
    REQUESTED_AMOUNTS,
    (),
    Vec<ic_wire::Bytes<{ crate::protocol::MAGNITUDE_BYTES }>>
);
