//! Module: ops::auth::issuer_canister_sig
//!
//! Responsibility: prepare, retrieve, and verify issuer canister-signature proofs.
//! Does not own: delegated-token claims, root proof provisioning, or endpoint DTOs.
//! Boundary: auth ops helper for issuer-local canister-signature proof material.

use super::AuthOps;
#[cfg(feature = "auth-issuer-canister-sig-verify")]
use super::canister_sig_key::parse_canister_sig_public_key_der;
#[cfg(feature = "auth-issuer-canister-sig-create")]
use crate::dto::auth::IcCanisterSignatureProofV1;
use crate::{
    InternalError, cdk::types::Principal, dto::auth::IssuerProof, ops::auth::AuthSignatureError,
};
use sha2::{Digest, Sha256};
#[cfg(feature = "auth-issuer-canister-sig-create")]
use std::cell::RefCell;

#[cfg(feature = "auth-issuer-canister-sig-create")]
pub const ISSUER_PROOF_RETRIEVAL_TTL_NS: u64 = 60_000_000_000;

const ISSUER_CANISTER_SIG_SEED: &[u8] = b"canic-issuer-delegated-token";
#[cfg(any(
    feature = "auth-issuer-canister-sig-create",
    feature = "auth-issuer-canister-sig-verify",
    test
))]
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

#[cfg(any(feature = "auth-issuer-canister-sig-verify", test))]
pub fn issuer_canister_sig_verification_message(payload_hash: [u8; 32]) -> Vec<u8> {
    let domain_len = u8::try_from(ISSUER_CANISTER_SIG_DOMAIN.len())
        .expect("issuer canister signature domain exceeds 255 bytes");

    let mut msg = Vec::with_capacity(1 + ISSUER_CANISTER_SIG_DOMAIN.len() + payload_hash.len());
    msg.push(domain_len);
    msg.extend_from_slice(ISSUER_CANISTER_SIG_DOMAIN);
    msg.extend_from_slice(&payload_hash);
    msg
}

impl AuthOps {
    pub(crate) fn prepare_issuer_canister_signature(
        payload_hash: [u8; 32],
        now_ns: u64,
    ) -> Result<PreparedIssuerCanisterSignature, InternalError> {
        #[cfg(feature = "auth-issuer-canister-sig-create")]
        {
            validate_issuer_canister_sig_domain_len()?;
            Ok(prepare_issuer_canister_signature(payload_hash, now_ns))
        }
        #[cfg(not(feature = "auth-issuer-canister-sig-create"))]
        {
            prepare_issuer_canister_signature(payload_hash, now_ns)
        }
    }

    pub(crate) fn get_issuer_canister_signature_proof(
        payload_hash: [u8; 32],
        issuer_pid: Principal,
    ) -> Result<IssuerProof, InternalError> {
        get_issuer_canister_signature_proof(payload_hash, issuer_pid)
    }

    pub(crate) fn verify_issuer_canister_signature_proof(
        payload_hash: [u8; 32],
        proof: &IssuerProof,
        expected_issuer_pid: Principal,
        ic_root_public_key_raw: &[u8],
    ) -> Result<(), InternalError> {
        verify_issuer_canister_signature_proof(
            payload_hash,
            proof,
            expected_issuer_pid,
            ic_root_public_key_raw,
        )
    }

    pub(crate) const fn issuer_canister_sig_verify_enabled() -> bool {
        cfg!(feature = "auth-issuer-canister-sig-verify")
    }

    pub(crate) const fn issuer_canister_sig_create_enabled() -> bool {
        cfg!(feature = "auth-issuer-canister-sig-create")
    }
}

#[cfg(feature = "auth-issuer-canister-sig-create")]
fn validate_issuer_canister_sig_domain_len() -> Result<(), InternalError> {
    u8::try_from(ISSUER_CANISTER_SIG_DOMAIN.len())
        .map(|_| ())
        .map_err(|_| {
            AuthSignatureError::ProofInvalid(
                "issuer canister signature domain exceeds 255 bytes".to_string(),
            )
            .into()
        })
}

#[cfg(feature = "auth-issuer-canister-sig-create")]
thread_local! {
    static ISSUER_CANISTER_SIGNATURES: RefCell<ic_canister_sig_creation::signature_map::SignatureMap> =
        RefCell::new(ic_canister_sig_creation::signature_map::SignatureMap::default());
}

#[cfg(feature = "auth-issuer-canister-sig-create")]
fn prepare_issuer_canister_signature(
    payload_hash: [u8; 32],
    now_ns: u64,
) -> PreparedIssuerCanisterSignature {
    use ic_canister_sig_creation::signature_map::CanisterSigInputs;

    crate::ops::runtime::metrics::delegated_auth::DelegatedAuthMetrics::record_issuer_proof_prepare_started();

    let inputs = CanisterSigInputs {
        domain: ISSUER_CANISTER_SIG_DOMAIN,
        seed: ISSUER_CANISTER_SIG_SEED,
        message: &payload_hash,
    };
    ISSUER_CANISTER_SIGNATURES.with(|signatures| {
        let mut signatures = signatures.borrow_mut();
        signatures.add_signature(&inputs);
        refresh_issuer_canister_sig_certified_data(&signatures.root_hash());
    });

    let retrieval_expires_at_ns = now_ns.saturating_add(ISSUER_PROOF_RETRIEVAL_TTL_NS);

    crate::ops::runtime::metrics::delegated_auth::DelegatedAuthMetrics::record_issuer_proof_prepare_completed();
    PreparedIssuerCanisterSignature {
        retrieval_expires_at_ns,
    }
}

#[cfg(feature = "auth-issuer-canister-sig-create")]
fn refresh_issuer_canister_sig_certified_data(signature_root_hash: &[u8; 32]) {
    use ic_canister_sig_creation::signature_map::LABEL_SIG;
    use ic_certification::labeled_hash;

    ic_cdk::api::certified_data_set(labeled_hash(LABEL_SIG, signature_root_hash));
}

#[cfg(not(feature = "auth-issuer-canister-sig-create"))]
fn prepare_issuer_canister_signature(
    _payload_hash: [u8; 32],
    _now_ns: u64,
) -> Result<PreparedIssuerCanisterSignature, InternalError> {
    crate::ops::runtime::metrics::delegated_auth::DelegatedAuthMetrics::record_issuer_proof_prepare_failed();
    Err(AuthSignatureError::ProofUnavailable.into())
}

#[cfg(feature = "auth-issuer-canister-sig-create")]
fn get_issuer_canister_signature_proof(
    payload_hash: [u8; 32],
    issuer_pid: Principal,
) -> Result<IssuerProof, InternalError> {
    use ic_canister_sig_creation::{CanisterSigPublicKey, signature_map::CanisterSigInputs};

    let inputs = CanisterSigInputs {
        domain: ISSUER_CANISTER_SIG_DOMAIN,
        seed: ISSUER_CANISTER_SIG_SEED,
        message: &payload_hash,
    };
    let signature_cbor = ISSUER_CANISTER_SIGNATURES.with(|signatures| {
        signatures
            .borrow()
            .get_signature_as_cbor(&inputs, None)
            .map_err(issuer_canister_signature_cbor_error)
    })?;
    let public_key_der =
        CanisterSigPublicKey::new(issuer_pid, ISSUER_CANISTER_SIG_SEED.to_vec()).to_der();

    Ok(IssuerProof::IcCanisterSignatureV1(
        IcCanisterSignatureProofV1 {
            signature_cbor,
            public_key_der,
        },
    ))
}

#[cfg(feature = "auth-issuer-canister-sig-create")]
fn issuer_canister_signature_cbor_error(
    err: ic_canister_sig_creation::signature_map::CanisterSigError,
) -> AuthSignatureError {
    match err {
        ic_canister_sig_creation::signature_map::CanisterSigError::NoCertificate => {
            AuthSignatureError::RootDataCertificateUnavailable
        }
        err @ ic_canister_sig_creation::signature_map::CanisterSigError::NoSignature => {
            AuthSignatureError::ProofInvalid(err.to_string())
        }
    }
}

#[cfg(not(feature = "auth-issuer-canister-sig-create"))]
fn get_issuer_canister_signature_proof(
    _payload_hash: [u8; 32],
    _issuer_pid: Principal,
) -> Result<IssuerProof, InternalError> {
    Err(AuthSignatureError::ProofUnavailable.into())
}

#[cfg(feature = "auth-issuer-canister-sig-verify")]
fn verify_issuer_canister_signature_proof(
    payload_hash: [u8; 32],
    proof: &IssuerProof,
    expected_issuer_pid: Principal,
    ic_root_public_key_raw: &[u8],
) -> Result<(), InternalError> {
    let IssuerProof::IcCanisterSignatureV1(proof) = proof;
    let (canister_id, seed) = parse_canister_sig_public_key_der(&proof.public_key_der)
        .map_err(AuthSignatureError::ProofInvalid)?;
    if canister_id != expected_issuer_pid {
        return Err(AuthSignatureError::ProofInvalid(
            "issuer canister signature public key canister id mismatch".to_string(),
        )
        .into());
    }
    if seed != ISSUER_CANISTER_SIG_SEED {
        return Err(AuthSignatureError::ProofInvalid(
            "issuer canister signature seed mismatch".to_string(),
        )
        .into());
    }

    let message = issuer_canister_sig_verification_message(payload_hash);
    ic_signature_verification::verify_canister_sig(
        &message,
        &proof.signature_cbor,
        &proof.public_key_der,
        ic_root_public_key_raw,
    )
    .map_err(AuthSignatureError::ProofInvalid)?;

    Ok(())
}

#[cfg(not(feature = "auth-issuer-canister-sig-verify"))]
fn verify_issuer_canister_signature_proof(
    _payload_hash: [u8; 32],
    _proof: &IssuerProof,
    _expected_issuer_pid: Principal,
    _ic_root_public_key_raw: &[u8],
) -> Result<(), InternalError> {
    Err(AuthSignatureError::ProofUnavailable.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issuer_canister_sig_verification_message_prefixes_domain_len() {
        let payload_hash = [7; 32];
        let msg = issuer_canister_sig_verification_message(payload_hash);

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

    #[cfg(not(feature = "auth-issuer-canister-sig-verify"))]
    #[test]
    fn issuer_canister_sig_verify_requires_verify_feature() {
        let proof =
            IssuerProof::IcCanisterSignatureV1(crate::dto::auth::IcCanisterSignatureProofV1 {
                signature_cbor: vec![1, 2, 3],
                public_key_der: vec![4, 5, 6],
            });

        assert!(
            AuthOps::verify_issuer_canister_signature_proof(
                [2; 32],
                &proof,
                Principal::from_slice(&[2; 29]),
                &[9; 96],
            )
            .is_err()
        );
    }
}
