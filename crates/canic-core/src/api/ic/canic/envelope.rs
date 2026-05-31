use crate::{
    cdk::{candid::encode_one, types::Principal},
    dto::{
        auth::{
            CanicInternalCallEnvelopeV1, CanicInternalCallHeaderV1, SignedInternalInvocationProofV1,
        },
        error::Error,
    },
};

pub(super) fn build_internal_call_envelope(
    target_canister: Principal,
    target_method: &str,
    proof: SignedInternalInvocationProofV1,
    args: Vec<u8>,
) -> CanicInternalCallEnvelopeV1 {
    CanicInternalCallEnvelopeV1 {
        version: 1,
        header: CanicInternalCallHeaderV1 {
            target_canister,
            target_method: target_method.to_string(),
        },
        proof,
        args,
    }
}

pub(super) fn encode_internal_call_envelope_raw(
    envelope: CanicInternalCallEnvelopeV1,
) -> Result<Vec<u8>, Error> {
    encode_one(envelope).map_err(|err| Error::invalid(err.to_string()))
}
