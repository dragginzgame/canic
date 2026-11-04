//! Canonical public signing bytes shared by the issuer producer and Root verifier.
//! Encoding a challenge establishes neither custody nor enrollment authority.

use crate::{dto::portable_enrollment::PortableIssuerChallenge, protocol::portable_enrollment};

/// Encode the complete v1 Root challenge with its distinct signature domain.
///
/// The signature covers Root, Fleet, issuer, key, operation, approval, nonce and
/// original validity window. No request/response transport frame is signed.
///
/// # Errors
/// Returns the Wire error when the challenge exceeds the protocol budget.
pub fn registration_message(
    challenge: &PortableIssuerChallenge,
) -> Result<Vec<u8>, ic_wire::Error> {
    let mut bytes = portable_enrollment::REGISTRATION_DOMAIN.to_vec();
    bytes.extend(ic_wire::native::encode(
        challenge,
        portable_enrollment::RESPONSE_LIMITS,
    )?);
    Ok(bytes)
}
