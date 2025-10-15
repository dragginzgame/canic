//!
//! ops::signature
//!
//! High-level wrapper around IC canister signatures.
//!
//! This allows a canister to sign arbitrary messages without holding any private key.
//!
//! Internally uses `ic_canister_sig_creation` and certified data to produce
//! verifiable, subnet-backed canister signatures.
//!
//! For verification, see: [`ic-standalone-sig-verifier`](https://crates.io/crates/ic-standalone-sig-verifier).
//!

use crate::{
    Error, ThisError, cdk::api::certified_data_set, ops::OpsError, types::Principal,
    utils::cbor::deserialize,
};
use ic_canister_sig_creation::{
    IC_ROOT_PUBLIC_KEY, hash_with_domain, parse_canister_sig_cbor,
    signature_map::{CanisterSigInputs, LABEL_SIG, SignatureMap},
};
use ic_signature_verification::verify_canister_sig;
use serde::de::DeserializeOwned;
use std::cell::RefCell;

thread_local! {
    /// Transient signature map, kept in heap memory only.
    /// Entries expire automatically after ~1 minute.
    static SIGNATURES: RefCell<SignatureMap> = RefCell::new(SignatureMap::default());
}

///
/// SignatureError
///

#[derive(Debug, ThisError)]
pub enum SignatureError {
    #[error("cannot parse signature")]
    CannotParseSignature,

    #[error("cannot parse tokens")]
    CannotParseTokens,

    #[error("invalid signature")]
    InvalidSignature,
}

impl From<SignatureError> for Error {
    fn from(err: SignatureError) -> Self {
        OpsError::from(err).into()
    }
}

///
/// Prepares a canister signature for a given message and seed.
///
/// This updates the canister's `certified_data` to include the
/// new root hash so that the IC subnet will certify it.
///
/// - `seed` should uniquely identify the logical key context.
/// - `message` is the data being signed.
///
pub fn prepare(domain: &[u8], seed: &[u8], message: &[u8]) {
    let sig_inputs = CanisterSigInputs {
        domain,
        seed,
        message,
    };

    SIGNATURES.with_borrow_mut(|sigs| {
        sigs.add_signature(&sig_inputs);
    });

    // Commit new certified root
    SIGNATURES.with_borrow(|sigs| {
        certified_data_set(hash_with_domain(LABEL_SIG, &sigs.root_hash()));
    });
}

///
/// Retrieves a prepared canister signature as CBOR-encoded bytes.
///
/// Returns `None` if the signature has expired or was never prepared.
///
/// This is intended for use in query calls.
///
#[must_use]
pub fn get(domain: &[u8], seed: &[u8], message: &[u8]) -> Option<Vec<u8>> {
    let sig_inputs = CanisterSigInputs {
        domain,
        seed,
        message,
    };

    SIGNATURES.with_borrow(|sigs| sigs.get_signature_as_cbor(&sig_inputs, None).ok())
}

///
/// High-level convenience helper that combines [`prepare`] and [`get`]
/// in one call. Suitable for simple use-cases where you don’t split update/query.
///
#[must_use]
pub fn sign(domain: &[u8], seed: &[u8], message: &[u8]) -> Option<Vec<u8>> {
    prepare(domain, seed, message);
    get(domain, seed, message)
}

///
/// Verify a user token that was issued by the auth canister.
///
/// - `token_bytes`: the CBOR-encoded `AuthToken`
/// - `signature`:  the CBOR canister signature returned by auth
/// - `issuer_pid`: the Principal of the auth canister (the one that signed)
///
pub fn verify(
    token_bytes: Vec<u8>,
    signature_cbor: Vec<u8>,
    issuer_pid: Principal,
) -> Result<(), Error> {
    // 1️⃣ Parse CBOR
    parse_canister_sig_cbor(&signature_cbor).map_err(|_| SignatureError::CannotParseSignature)?;

    // 2️⃣ Verify the IC canister signature cryptographically
    verify_canister_sig(
        &signature_cbor,
        &token_bytes,
        issuer_pid.as_slice(),
        &IC_ROOT_PUBLIC_KEY,
    )
    .map_err(|_| SignatureError::InvalidSignature)?;

    Ok(())
}

///
/// Parses CBOR-encoded token bytes into a strongly-typed value `T`.
///
/// This is a thin convenience wrapper over [`deserialize`], ensuring that
/// all token deserialization uses the same canonical CBOR implementation.
///
/// # Errors
/// Returns `Error::InvalidToken` (or similar, depending on your Error type)
/// if deserialization fails.
///
pub fn parse_tokens<T>(token_bytes: &[u8]) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    let token = deserialize::<T>(token_bytes).map_err(|_| SignatureError::CannotParseTokens)?;

    Ok(token)
}

///
/// Returns the canister’s current signature root hash.
/// Useful for debugging or introspection.
///
#[must_use]
pub fn root_hash() -> Vec<u8> {
    SIGNATURES.with_borrow(|sigs| sigs.root_hash().to_vec())
}
