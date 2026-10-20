//! Explicit Candid codecs for this fixture's existing endpoint contracts.

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

/// Preserve the empty reply argument list on fixture control endpoints.
pub fn encode_unit() -> Vec<u8> {
    ic_platform::candid::contract::encode_args(()).unwrap_or_else(|error| trap(error.to_string()))
}
