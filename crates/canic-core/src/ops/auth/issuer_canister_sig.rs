//! Module: ops::auth::issuer_canister_sig
//!
//! Responsibility: identify issuer signature capability and its Root proof binding.
//! Does not own: delegated-token claims, root proof provisioning, or endpoint DTOs.
//! Boundary: auth ops helper for issuer-local canister-signature proof material.

use super::AuthOps;
use sha2::{Digest, Sha256};

const ISSUER_CANISTER_SIG_SEED: &[u8] = b"canic-issuer-delegated-token";
#[cfg(test)]
const ISSUER_CANISTER_SIG_DOMAIN: &[u8] = b"canic-issuer-delegated-token";

pub fn issuer_canister_sig_seed_hash() -> [u8; 32] {
    Sha256::digest(ISSUER_CANISTER_SIG_SEED).into()
}

impl AuthOps {
    pub(crate) const fn issuer_canister_sig_verify_enabled() -> bool {
        cfg!(feature = "auth-issuer-canister-sig-verify")
    }

    pub(crate) const fn issuer_canister_sig_create_enabled() -> bool {
        cfg!(feature = "auth-issuer-canister-sig-create")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issuer_canister_sig_verification_message_prefixes_domain_len() {
        let payload_hash = [7; 32];
        let msg = ic_auth::canister_signature::domain_separated_message(
            ISSUER_CANISTER_SIG_DOMAIN,
            payload_hash,
        )
        .unwrap();

        assert_eq!(usize::from(msg[0]), ISSUER_CANISTER_SIG_DOMAIN.len());
        assert_eq!(
            &msg[1..=ISSUER_CANISTER_SIG_DOMAIN.len()],
            ISSUER_CANISTER_SIG_DOMAIN
        );
        assert_eq!(&msg[1 + ISSUER_CANISTER_SIG_DOMAIN.len()..], payload_hash);
    }

    #[test]
    fn issuer_canister_sig_seed_hash_matches_binding_seed_hash_input() {
        let seed_hash = issuer_canister_sig_seed_hash();
        let expected: [u8; 32] = Sha256::digest(b"canic-issuer-delegated-token").into();

        assert_eq!(seed_hash, expected);
    }

    #[test]
    fn issuer_canister_sig_feature_flags_match_compile_features() {
        assert_eq!(
            AuthOps::issuer_canister_sig_create_enabled(),
            cfg!(feature = "auth-issuer-canister-sig-create")
        );
        assert_eq!(
            AuthOps::issuer_canister_sig_verify_enabled(),
            cfg!(feature = "auth-issuer-canister-sig-verify")
        );
    }
}
