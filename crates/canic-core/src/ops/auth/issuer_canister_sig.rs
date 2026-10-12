//! Module: ops::auth::issuer_canister_sig
//!
//! Responsibility: prepare, retrieve, and verify issuer canister-signature proofs.
//! Does not own: delegated-token claims, root proof provisioning, or endpoint DTOs.
//! Boundary: auth ops helper for issuer-local canister-signature proof material.

use super::AuthOps;
#[cfg(any(
    not(feature = "auth-issuer-canister-sig-create"),
    not(feature = "auth-issuer-canister-sig-verify")
))]
use crate::ops::auth::AuthSignatureError;
use crate::{InternalError, cdk::types::Principal, dto::auth::IssuerProof};
use sha2::{Digest, Sha256};

#[cfg(feature = "auth-issuer-canister-sig-create")]
pub const ISSUER_PROOF_RETRIEVAL_TTL_NS: u64 = 60_000_000_000;

const ISSUER_CANISTER_SIG_SEED: &[u8] = b"canic-issuer-delegated-token";
#[cfg(any(feature = "auth-issuer-canister-sig-create", test))]
const ISSUER_CANISTER_SIG_DOMAIN: &[u8] = b"canic-issuer-delegated-token";

///
/// PreparedIssuerCanisterSignature
///
/// Prepared issuer canister-signature metadata returned after leaf creation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedIssuerCanisterSignature {
    pub retrieval_expires_at_ns: u64,
}

pub fn issuer_canister_sig_seed_hash() -> [u8; 32] {
    Sha256::digest(ISSUER_CANISTER_SIG_SEED).into()
}

impl AuthOps {
    pub(crate) fn prepare_issuer_canister_signature(
        payload_hash: [u8; 32],
        now_ns: u64,
    ) -> Result<PreparedIssuerCanisterSignature, InternalError> {
        #[cfg(feature = "auth-issuer-canister-sig-create")]
        {
            use crate::ops::runtime::metrics::delegated_auth::DelegatedAuthMetrics;
            DelegatedAuthMetrics::record_issuer_proof_prepare_started();
            let prepared = super::signature_store::prepare(
                ic_auth::signature_store::SignatureInputs {
                    seed: ISSUER_CANISTER_SIG_SEED,
                    domain: ISSUER_CANISTER_SIG_DOMAIN,
                    message: &payload_hash,
                },
                now_ns,
                ISSUER_PROOF_RETRIEVAL_TTL_NS,
            )
            .inspect_err(|_| DelegatedAuthMetrics::record_issuer_proof_prepare_failed())?;
            DelegatedAuthMetrics::record_issuer_proof_prepare_completed();
            Ok(PreparedIssuerCanisterSignature {
                retrieval_expires_at_ns: prepared.retrieval_expires_at_ns,
            })
        }
        #[cfg(not(feature = "auth-issuer-canister-sig-create"))]
        {
            let _ = (payload_hash, now_ns);
            Err(AuthSignatureError::ProofUnavailable.into())
        }
    }

    pub(crate) fn get_issuer_canister_signature_proof(
        payload_hash: [u8; 32],
        issuer_pid: Principal,
    ) -> Result<IssuerProof, InternalError> {
        #[cfg(feature = "auth-issuer-canister-sig-create")]
        {
            super::signature_store::retrieve(
                ic_auth::signature_store::SignatureInputs {
                    seed: ISSUER_CANISTER_SIG_SEED,
                    domain: ISSUER_CANISTER_SIG_DOMAIN,
                    message: &payload_hash,
                },
                issuer_pid,
                crate::ops::ic::IcOps::now_nanos(),
            )
            .map(IssuerProof::IcCanisterSignatureV1)
        }
        #[cfg(not(feature = "auth-issuer-canister-sig-create"))]
        {
            let _ = (payload_hash, issuer_pid);
            Err(AuthSignatureError::ProofUnavailable.into())
        }
    }

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

    #[cfg(not(feature = "auth-issuer-canister-sig-create"))]
    #[test]
    fn issuer_canister_sig_create_requires_create_feature() {
        assert!(AuthOps::prepare_issuer_canister_signature([1; 32], 1).is_err());
        assert!(
            AuthOps::get_issuer_canister_signature_proof([2; 32], Principal::from_slice(&[2; 29]),)
                .is_err()
        );
    }
}
