//! Module: infra::ic::external_candid
//!
//! Responsibility: decode required external IC protocol replies through Platform.
//! Does not own: owned RPC codecs, transport, request construction or retry policy.
//! Boundary: management, ledger and NNS adapters preserve Candid tuple semantics.

use crate::infra::ic::IcInfraError;
use ic_platform::candid::contract::{
    ArgumentDecoder, CandidType, DecoderConfig, decode_args_with_config,
};
use serde::de::DeserializeOwned;

/// Decode one externally defined result, preserving the protocol's argument tuple.
pub(super) fn decode_one<R: CandidType + DeserializeOwned>(
    bytes: &[u8],
) -> Result<R, IcInfraError> {
    decode_args::<(R,)>(bytes).map(|(value,)| value)
}

/// Keep decoding failures distinct from request encoding and physical call failures.
pub(super) fn decode_args<R: for<'de> ArgumentDecoder<'de>>(
    bytes: &[u8],
) -> Result<R, IcInfraError> {
    decode_args_with_config(bytes, &DecoderConfig::new()).map_err(IcInfraError::CandidDecode)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ic_platform::candid::contract::encode_args;

    #[test]
    fn external_reply_arity_preserves_single_tuple_and_skipped_arguments() {
        let bytes = encode_args((42_u64, "extra")).unwrap();
        assert_eq!(decode_one::<u64>(&bytes).unwrap(), 42);
        assert_eq!(
            decode_args::<(u64, String)>(&bytes).unwrap(),
            (42, "extra".into())
        );
        decode_args::<()>(&encode_args(()).unwrap()).unwrap();
        assert!(matches!(
            decode_one::<u64>(&encode_args(()).unwrap()),
            Err(IcInfraError::CandidDecode(_))
        ));
    }

    #[test]
    fn malformed_external_reply_retains_decode_failure_identity() {
        assert!(matches!(
            decode_one::<u64>(&[1, 2, 3]),
            Err(IcInfraError::CandidDecode(_))
        ));
        let bytes = encode_args(("wrong type",)).unwrap();
        assert!(matches!(
            decode_one::<u64>(&bytes),
            Err(IcInfraError::CandidDecode(_))
        ));
    }
}
