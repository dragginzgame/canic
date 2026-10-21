//! Module: ops::auth::root_canister_sig
//!
//! Responsibility: prepare, retrieve, and verify root canister-signature proofs.
//! Does not own: root delegation certificate construction or endpoint DTOs.
//! Boundary: auth ops helper for root canister-signature proof material.

use super::AuthOps;
#[cfg(feature = "auth-root-canister-sig-verify")]
use super::canister_signature::{CanisterSignatureVerificationInput, verify_canister_signature};
#[cfg(test)]
use crate::domain::auth::{IC_ROOT_PUBLIC_KEY_RAW_LENGTH, ic_root_public_key_raw_from_der_or_raw};
use crate::{InternalError, cdk::types::Principal, dto::auth::RoleAttestationRootProof};

#[cfg(any(
    test,
    not(feature = "auth-root-canister-sig-create"),
    not(feature = "auth-root-canister-sig-verify")
))]
use crate::ops::auth::AuthSignatureError;

#[cfg(feature = "auth-root-canister-sig-create")]
pub const ROOT_PROOF_RETRIEVAL_TTL_NS: u64 = 60_000_000_000;

#[cfg(any(
    feature = "auth-root-canister-sig-create",
    feature = "auth-root-canister-sig-verify",
    test
))]
const ROOT_CANISTER_SIG_SEED: &[u8] = b"canic-root-role-attestation";
#[cfg(any(
    feature = "auth-root-canister-sig-create",
    feature = "auth-root-canister-sig-verify",
    test
))]
const ROOT_CANISTER_SIG_DOMAIN: &[u8] = b"canic-root-role-attestation";

///
/// PreparedRootCanisterSignature
///
/// Prepared root canister-signature metadata returned after leaf creation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedRootCanisterSignature {
    pub retrieval_expires_at_ns: u64,
}

impl AuthOps {
    pub(crate) fn prepare_root_canister_signature(
        payload_hash: [u8; 32],
        now_ns: u64,
    ) -> Result<PreparedRootCanisterSignature, InternalError> {
        #[cfg(feature = "auth-root-canister-sig-create")]
        {
            use crate::ops::runtime::metrics::delegated_auth::DelegatedAuthMetrics;
            DelegatedAuthMetrics::record_root_proof_prepare_started();
            let prepared = super::signature_store::prepare(
                ic_auth::signature_store::SignatureInputs {
                    seed: ROOT_CANISTER_SIG_SEED,
                    domain: ROOT_CANISTER_SIG_DOMAIN,
                    message: &payload_hash,
                },
                now_ns,
                ROOT_PROOF_RETRIEVAL_TTL_NS,
            )
            .inspect_err(|_| DelegatedAuthMetrics::record_root_proof_prepare_failed())?;
            DelegatedAuthMetrics::record_root_proof_prepare_completed();
            Ok(PreparedRootCanisterSignature {
                retrieval_expires_at_ns: prepared.retrieval_expires_at_ns,
            })
        }
        #[cfg(not(feature = "auth-root-canister-sig-create"))]
        {
            let _ = (payload_hash, now_ns);
            Err(AuthSignatureError::ProofUnavailable.into())
        }
    }

    pub(crate) fn get_root_canister_signature_proof(
        payload_hash: [u8; 32],
        root_pid: Principal,
        now_ns: u64,
    ) -> Result<RoleAttestationRootProof, InternalError> {
        #[cfg(feature = "auth-root-canister-sig-create")]
        {
            super::signature_store::retrieve(
                ic_auth::signature_store::SignatureInputs {
                    seed: ROOT_CANISTER_SIG_SEED,
                    domain: ROOT_CANISTER_SIG_DOMAIN,
                    message: &payload_hash,
                },
                root_pid,
                now_ns,
            )
            .map(|proof| {
                RoleAttestationRootProof::IcCanisterSignatureV1(
                    crate::dto::auth::IcCanisterSignatureProofV1 {
                        signature_cbor: proof.signature_cbor,
                        public_key_der: proof.public_key_der,
                    },
                )
            })
        }
        #[cfg(not(feature = "auth-root-canister-sig-create"))]
        {
            let _ = (payload_hash, root_pid, now_ns);
            Err(AuthSignatureError::ProofUnavailable.into())
        }
    }

    pub(crate) fn verify_root_canister_signature_proof(
        payload_hash: [u8; 32],
        proof: &RoleAttestationRootProof,
        expected_root_pid: Principal,
        ic_root_public_key_raw: &[u8],
        now_ns: u64,
    ) -> Result<(), InternalError> {
        #[cfg(feature = "auth-root-canister-sig-verify")]
        {
            let RoleAttestationRootProof::IcCanisterSignatureV1(proof) = proof;
            let max_certificate_age_ns = crate::ops::config::ConfigOps::role_attestation_config()?
                .max_ttl_secs
                .checked_mul(1_000_000_000)
                .ok_or_else(|| InternalError::public(crate::diagnostics::codes::TIME_CAPACITY))?;
            verify_canister_signature(CanisterSignatureVerificationInput {
                payload_hash,
                proof,
                signing_canister: expected_root_pid,
                seed: ROOT_CANISTER_SIG_SEED,
                domain: ROOT_CANISTER_SIG_DOMAIN,
                ic_root_public_key_raw,
                now_ns,
                max_certificate_age_ns,
            })
            .map_err(InternalError::from)
        }
        #[cfg(not(feature = "auth-root-canister-sig-verify"))]
        {
            let _ = (
                payload_hash,
                proof,
                expected_root_pid,
                ic_root_public_key_raw,
                now_ns,
            );
            Err(AuthSignatureError::ProofUnavailable.into())
        }
    }

    pub(crate) const fn root_canister_sig_verify_enabled() -> bool {
        cfg!(feature = "auth-root-canister-sig-verify")
    }

    pub(crate) const fn root_canister_sig_create_enabled() -> bool {
        cfg!(feature = "auth-root-canister-sig-create")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_message_prefixes_domain_length_and_domain() {
        let msg = ic_auth::canister_signature::domain_separated_message(
            ROOT_CANISTER_SIG_DOMAIN,
            [7; 32],
        )
        .unwrap();
        let domain_len = u8::try_from(ROOT_CANISTER_SIG_DOMAIN.len()).unwrap();
        let domain_start = 1;
        let domain_end = domain_start + ROOT_CANISTER_SIG_DOMAIN.len();

        assert_eq!(msg[0], domain_len);
        assert_eq!(&msg[domain_start..domain_end], ROOT_CANISTER_SIG_DOMAIN);
        assert_eq!(&msg[domain_end..], &[7; 32]);
    }

    #[test]
    fn role_attestation_uses_expected_seed_and_domain() {
        assert_eq!(ROOT_CANISTER_SIG_SEED, b"canic-root-role-attestation");
        assert_eq!(ROOT_CANISTER_SIG_DOMAIN, b"canic-root-role-attestation");
    }

    #[test]
    fn extracts_raw_ic_root_key_from_der_or_raw() {
        const IC_ROOT_PK_DER_PREFIX: &[u8; 37] = b"\x30\x81\x82\x30\x1d\x06\x0d\x2b\x06\x01\x04\x01\x82\xdc\x7c\x05\x03\x01\x02\x01\x06\x0c\x2b\x06\x01\x04\x01\x82\xdc\x7c\x05\x03\x02\x01\x03\x61\x00";
        let mut der = IC_ROOT_PK_DER_PREFIX.to_vec();
        der.extend_from_slice(&[9; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]);

        assert_eq!(
            ic_root_public_key_raw_from_der_or_raw(&der).unwrap(),
            vec![9; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]
        );
        assert_eq!(
            ic_root_public_key_raw_from_der_or_raw(&[8; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]).unwrap(),
            vec![8; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]
        );
    }

    #[cfg(feature = "auth-root-canister-sig-create")]
    #[test]
    fn missing_data_certificate_maps_to_typed_root_error() {
        let err = super::super::signature_store::map_error(
            ic_auth::signature_store::SignatureStoreError::NoCertificate,
        );

        assert!(matches!(
            err,
            AuthSignatureError::RootDataCertificateUnavailable
        ));
    }

    #[cfg(feature = "auth-root-canister-sig-create")]
    #[test]
    fn non_certificate_signature_error_remains_invalid_proof() {
        let err = super::super::signature_store::map_error(
            ic_auth::signature_store::SignatureStoreError::NotPrepared,
        );

        assert!(matches!(err, AuthSignatureError::ProofInvalid(_)));
    }
}
