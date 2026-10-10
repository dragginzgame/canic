//! Module: ops::auth::delegated::active_proof
//!
//! Responsibility: validate and materialize issuer-local active delegation proof state.
//! Does not own: active proof storage, root proof construction, or endpoint guards.
//! Boundary: pure installation helper called before auth storage mutation.

use super::canonical::CanonicalAuthError;
use crate::{
    cdk::types::Principal,
    dto::auth::{ActiveDelegationProof, DelegationProof},
    ops::auth::AuthChainKeyRootVerifierConfig,
};
use thiserror::Error;

///
/// InstallActiveDelegationProofInput
///
/// Input for validating and materializing one active delegation proof.
///

pub struct InstallActiveDelegationProofInput<'a> {
    pub proof: DelegationProof,
    pub installed_by: Principal,
    pub this_canister: Principal,
    pub now_ns: u64,
    pub verifier: &'a AuthChainKeyRootVerifierConfig,
}

///
/// InstallActiveDelegationProofError
///
/// Typed failure surface for active delegation proof installation.
///

#[derive(Debug, Eq, Error, PartialEq)]
pub enum InstallActiveDelegationProofError {
    #[error("active delegation proof is for another issuer")]
    IssuerMismatch,
    #[error("active delegation proof cert is not yet valid")]
    CertNotYetValid,
    #[error("active delegation proof cert expired")]
    CertExpired,
    #[error("active delegation proof root proof invalid: {0}")]
    #[cfg(any(feature = "auth-chain-key-ecdsa", test))]
    RootProofInvalid(ic_auth::token::TokenVerificationError),
    #[error("active delegation proof root key is not permitted on this network")]
    #[cfg(any(feature = "auth-chain-key-ecdsa", test))]
    RootPolicyRejected,
    #[error("active delegation proof verification support is unavailable")]
    #[cfg(not(any(feature = "auth-chain-key-ecdsa", test)))]
    VerificationUnavailable,
    #[error(transparent)]
    Canonical(#[from] CanonicalAuthError),
}

pub fn install_active_delegation_proof(
    input: InstallActiveDelegationProofInput<'_>,
) -> Result<ActiveDelegationProof, InstallActiveDelegationProofError> {
    let cert = &input.proof.cert;
    if cert.issuer_pid != input.this_canister {
        return Err(InstallActiveDelegationProofError::IssuerMismatch);
    }
    if input.now_ns < cert.not_before_ns {
        return Err(InstallActiveDelegationProofError::CertNotYetValid);
    }
    if input.now_ns >= cert.expires_at_ns {
        return Err(InstallActiveDelegationProofError::CertExpired);
    }

    let verified = verify_for_installation(&input)?;
    let not_before_ns = cert.not_before_ns;
    let expires_at_ns = verified.expires_at_ns;
    let refresh_after_ns = refresh_after_ns(input.now_ns, expires_at_ns);

    Ok(ActiveDelegationProof {
        proof: input.proof,
        cert_hash: verified.cert_hash,
        not_before_ns,
        expires_at_ns,
        refresh_after_ns,
        installed_at_ns: input.now_ns,
        installed_by: input.installed_by,
    })
}

/// Proof evidence copied from IC Auth; it carries no installation authorization.
struct VerifiedActiveProof {
    cert_hash: [u8; 32],
    expires_at_ns: u64,
}

#[cfg(any(feature = "auth-chain-key-ecdsa", test))]
fn verify_for_installation(
    input: &InstallActiveDelegationProofInput<'_>,
) -> Result<VerifiedActiveProof, InstallActiveDelegationProofError> {
    let policy = &input.verifier.policy;
    let permitted_key = match policy.build_network {
        crate::ids::BuildNetwork::Ic => policy.key_id.name == "key_1",
        crate::ids::BuildNetwork::Local => {
            policy.key_id.name != "test_key_1" || input.verifier.allow_test_chain_key
        }
    };
    if !permitted_key {
        return Err(InstallActiveDelegationProofError::RootPolicyRejected);
    }
    let root_key = ic_auth::token::RootKeyPolicy {
        root_canister_id: policy.root_canister_id,
        algorithm: policy.algorithm,
        key_id: policy.key_id.clone(),
        derivation_path_hash: policy.derivation_path_hash,
        public_key: policy.public_key.clone(),
        key_version: policy.key_version,
        min_accepted_key_version: policy.min_accepted_key_version,
        min_accepted_proof_epoch: policy.min_accepted_proof_epoch,
        min_accepted_registry_epoch: policy.min_accepted_registry_epoch,
        valid_from_ns: policy.valid_from_ns,
        accept_until_ns: policy.accept_until_ns,
        max_revocation_latency_ns: policy.max_revocation_latency_ns,
    };
    let protocol_proof = super::protocol::proof(&input.proof)?;
    let verified = ic_auth::token::verify_delegation_proof(
        &protocol_proof,
        &ic_auth::token::DelegationProofVerificationContext {
            expected_issuer: input.this_canister,
            root_key: &root_key,
            now_ns: input.now_ns,
            limits: ic_auth::token::DelegationProofVerificationLimits {
                max_cert_ttl_ns: policy.max_revocation_latency_ns,
                max_token_ttl_ns: policy.max_revocation_latency_ns,
                max_future_skew_ns: crate::ops::auth::AUTH_TIME_SKEW_ALLOWANCE_NS,
                max_variable_bytes: 64 * 1024,
                max_witness_steps: 64,
            },
        },
    )
    .map_err(InstallActiveDelegationProofError::RootProofInvalid)?;
    Ok(VerifiedActiveProof {
        cert_hash: verified.certificate_hash(),
        expires_at_ns: verified.expires_at_ns(),
    })
}

#[cfg(not(any(feature = "auth-chain-key-ecdsa", test)))]
const fn verify_for_installation(
    input: &InstallActiveDelegationProofInput<'_>,
) -> Result<VerifiedActiveProof, InstallActiveDelegationProofError> {
    let _ = input.verifier;
    Err(InstallActiveDelegationProofError::VerificationUnavailable)
}

const fn refresh_after_ns(now_ns: u64, expires_at_ns: u64) -> u64 {
    let remaining_ns = expires_at_ns.saturating_sub(now_ns);
    now_ns + remaining_ns.saturating_sub(remaining_ns / 5)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::auth::{
            ChainKeyAlgorithm, ChainKeyBatchHeaderV1, ChainKeyBatchWitnessV1,
            ChainKeyDelegationCertV1, ChainKeyKeyId, ChainKeyRootSignatureV1, DelegatedRoleGrant,
            DelegationAudience, DelegationCert, IcChainKeyBatchSignatureProofV1,
            IssuerProofAlgorithm, IssuerProofBinding, RootKeyPolicyV1, RootProof,
        },
        ops::auth::delegated::canonical::{
            chain_key_batch_header_hash, chain_key_delegation_cert_hash,
            chain_key_derivation_path_hash, issuer_proof_binding_hash,
        },
    };
    use canic_contracts::ids::{BuildNetwork, CanisterRole};
    use ic_auth::token::TokenVerificationError;
    use k256::ecdsa::{Signature, SigningKey, signature::hazmat::PrehashSigner};

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn cert() -> DelegationCert {
        let issuer_proof_alg = IssuerProofAlgorithm::IcCanisterSignatureV1;
        let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 { seed_hash: [2; 32] };
        let issuer_proof_binding_hash =
            issuer_proof_binding_hash(p(2), issuer_proof_alg, issuer_proof_binding).unwrap();

        DelegationCert {
            root_pid: p(1),
            issuer_pid: p(2),
            issuer_proof_alg,
            issuer_proof_binding_hash,
            issuer_proof_binding,
            issued_at_ns: 10,
            not_before_ns: 20,
            expires_at_ns: 120,
            max_token_ttl_ns: 30,
            aud: DelegationAudience::Fleet(crate::test::support::fleet_key(1)),
            grants: vec![DelegatedRoleGrant {
                target: CanisterRole::owned("project_instance".to_string()),
                scopes: vec!["read".to_string()],
            }],
        }
    }

    fn proof() -> DelegationProof {
        let cert = cert();
        let policy = verifier().policy;
        let leaf = ChainKeyDelegationCertV1 {
            root_canister_id: cert.root_pid,
            issuer_canister_id: cert.issuer_pid,
            proof_epoch: 5,
            issuer_proof_algorithm: cert.issuer_proof_alg,
            issuer_proof_binding_hash: cert.issuer_proof_binding_hash,
            issuer_proof_binding: cert.issuer_proof_binding,
            max_token_ttl_ns: cert.max_token_ttl_ns,
            audience: cert.aud.clone(),
            grants: cert.grants.clone(),
            not_before_ns: cert.not_before_ns,
            expires_at_ns: cert.expires_at_ns,
            registry_epoch: 6,
            registry_hash: [8; 32],
        };
        let header = ChainKeyBatchHeaderV1 {
            schema_version: 1,
            root_canister_id: cert.root_pid,
            batch_id: [7; 32],
            proof_epoch: leaf.proof_epoch,
            registry_epoch: leaf.registry_epoch,
            registry_hash: leaf.registry_hash,
            tree_root: chain_key_delegation_cert_hash(&leaf).unwrap(),
            not_before_ns: cert.not_before_ns,
            expires_at_ns: cert.expires_at_ns,
            algorithm: policy.algorithm,
            key_id: policy.key_id.clone(),
            derivation_path_hash: policy.derivation_path_hash,
            key_version: policy.key_version,
        };
        let signature: Signature = signing_key()
            .sign_prehash(&chain_key_batch_header_hash(&header).unwrap())
            .unwrap();
        DelegationProof {
            cert,
            root_proof: RootProof::IcChainKeyBatchSignatureV1(IcChainKeyBatchSignatureProofV1 {
                header,
                delegation_cert: leaf,
                issuer_witness: ChainKeyBatchWitnessV1 { steps: Vec::new() },
                signature: ChainKeyRootSignatureV1 {
                    algorithm: policy.algorithm,
                    key_id: policy.key_id,
                    derivation_path: vec![b"canic".to_vec()],
                    public_key: policy.public_key,
                    signature: signature.to_bytes().to_vec(),
                },
            }),
        }
    }

    fn signing_key() -> SigningKey {
        SigningKey::from_slice(&[7; 32]).unwrap()
    }

    fn verifier() -> AuthChainKeyRootVerifierConfig {
        AuthChainKeyRootVerifierConfig {
            policy: RootKeyPolicyV1 {
                root_canister_id: p(1),
                algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
                key_id: ChainKeyKeyId {
                    name: "test_key_1".to_owned(),
                },
                derivation_path_hash: chain_key_derivation_path_hash(&[b"canic".to_vec()]).unwrap(),
                public_key: signing_key()
                    .verifying_key()
                    .to_encoded_point(true)
                    .as_bytes()
                    .to_vec(),
                key_version: 4,
                min_accepted_key_version: 4,
                min_accepted_proof_epoch: 5,
                min_accepted_registry_epoch: 6,
                max_revocation_latency_ns: 100,
                valid_from_ns: 1,
                accept_until_ns: 200,
                build_network: BuildNetwork::Local,
            },
            allow_test_chain_key: true,
        }
    }

    fn input(
        proof: DelegationProof,
        verifier: &AuthChainKeyRootVerifierConfig,
    ) -> InstallActiveDelegationProofInput<'_> {
        InstallActiveDelegationProofInput {
            proof,
            installed_by: p(10),
            this_canister: p(2),
            now_ns: 20,
            verifier,
        }
    }

    #[test]
    fn install_active_delegation_proof_builds_active_state_after_root_verify() {
        let verifier = verifier();
        let active = install_active_delegation_proof(input(proof(), &verifier)).unwrap();

        assert_eq!(active.proof.cert.issuer_pid, p(2));
        assert_eq!(active.not_before_ns, 20);
        assert_eq!(active.expires_at_ns, 120);
        assert_eq!(active.refresh_after_ns, 100);
        assert_eq!(active.installed_at_ns, 20);
        assert_eq!(active.installed_by, p(10));
    }

    #[test]
    fn install_active_delegation_proof_rejects_wrong_issuer() {
        let verifier = verifier();
        let mut input = input(proof(), &verifier);
        input.this_canister = p(9);

        assert_eq!(
            install_active_delegation_proof(input),
            Err(InstallActiveDelegationProofError::IssuerMismatch)
        );
    }

    #[test]
    fn install_active_delegation_proof_rejects_time_bounds() {
        let verifier = verifier();
        let mut early = input(proof(), &verifier);
        early.now_ns = 19;
        assert_eq!(
            install_active_delegation_proof(early),
            Err(InstallActiveDelegationProofError::CertNotYetValid)
        );

        let mut expired = input(proof(), &verifier);
        expired.now_ns = 120;
        assert_eq!(
            install_active_delegation_proof(expired),
            Err(InstallActiveDelegationProofError::CertExpired)
        );
    }

    #[test]
    fn install_active_delegation_proof_rejects_root_proof_failure() {
        let verifier = verifier();
        let mut proof = proof();
        let RootProof::IcChainKeyBatchSignatureV1(root) = &mut proof.root_proof;
        root.signature.signature[0] ^= 1;
        assert_eq!(
            install_active_delegation_proof(input(proof, &verifier)),
            Err(InstallActiveDelegationProofError::RootProofInvalid(
                TokenVerificationError::RootSignatureInvalid
            ))
        );
    }

    #[test]
    fn installation_caps_validity_and_refresh_at_enrolled_key_deadline() {
        let mut verifier = verifier();
        verifier.policy.accept_until_ns = 70;
        let active = install_active_delegation_proof(input(proof(), &verifier)).unwrap();
        assert_eq!(active.proof.cert.expires_at_ns, 120);
        assert_eq!(active.expires_at_ns, 70);
        assert_eq!(active.refresh_after_ns, 60);
        assert_eq!(
            active.cert_hash,
            super::super::canonical::cert_hash(&active.proof.cert).unwrap()
        );
    }

    #[test]
    fn installation_rechecks_enrollment_epochs_and_network_admission() {
        let mut verifier = verifier();
        verifier.policy.min_accepted_proof_epoch = 6;
        assert_eq!(
            install_active_delegation_proof(input(proof(), &verifier)),
            Err(InstallActiveDelegationProofError::RootProofInvalid(
                TokenVerificationError::StaleAuthority {
                    field: "proof_epoch"
                }
            ))
        );
        verifier.policy.min_accepted_proof_epoch = 5;
        verifier.policy.public_key = SigningKey::from_slice(&[9; 32])
            .unwrap()
            .verifying_key()
            .to_encoded_point(true)
            .as_bytes()
            .to_vec();
        assert_eq!(
            install_active_delegation_proof(input(proof(), &verifier)),
            Err(InstallActiveDelegationProofError::RootProofInvalid(
                TokenVerificationError::BindingMismatch {
                    field: "root_public_key"
                }
            ))
        );
        verifier.policy.build_network = BuildNetwork::Ic;
        assert_eq!(
            install_active_delegation_proof(input(proof(), &verifier)),
            Err(InstallActiveDelegationProofError::RootPolicyRejected)
        );
        verifier.policy.build_network = BuildNetwork::Local;
        verifier.allow_test_chain_key = false;
        assert_eq!(
            install_active_delegation_proof(input(proof(), &verifier)),
            Err(InstallActiveDelegationProofError::RootPolicyRejected)
        );
    }

    #[test]
    fn installation_rejects_excess_variable_material_and_witness_depth() {
        let verifier = verifier();
        let mut large = proof();
        let RootProof::IcChainKeyBatchSignatureV1(root) = &mut large.root_proof;
        root.signature.derivation_path = vec![vec![1; 64 * 1024]];
        assert_eq!(
            install_active_delegation_proof(input(large, &verifier)),
            Err(InstallActiveDelegationProofError::RootProofInvalid(
                TokenVerificationError::InputTooLarge
            ))
        );
        let mut deep = proof();
        let RootProof::IcChainKeyBatchSignatureV1(root) = &mut deep.root_proof;
        root.issuer_witness.steps =
            vec![crate::dto::auth::ChainKeyBatchWitnessStepV1::LeftSibling([1; 32]); 65];
        assert_eq!(
            install_active_delegation_proof(input(deep, &verifier)),
            Err(InstallActiveDelegationProofError::RootProofInvalid(
                TokenVerificationError::InputTooLarge
            ))
        );
    }
}
