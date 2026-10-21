//! Module: ops::auth::delegated::chain_key
//!
//! Responsibility: validate chain-key signing material and public-key encoding.
//! Does not own: Root-proof verification, issuer policy or management calls.
//! Boundary: signing response checks and configured key admission.

use crate::dto::auth::ChainKeyAlgorithm;

/// Signature material returned by the management canister.
pub(in crate::ops::auth) struct ChainKeySignatureVerificationInput<'a> {
    pub algorithm: ChainKeyAlgorithm,
    pub public_key: &'a [u8],
    pub message_hash: [u8; 32],
    pub signature: &'a [u8],
}

pub(in crate::ops::auth) fn verify_chain_key_ecdsa_signature(
    input: ChainKeySignatureVerificationInput<'_>,
) -> Result<(), String> {
    if input.algorithm != ChainKeyAlgorithm::EcdsaSecp256k1 {
        return Err("unsupported chain-key signature algorithm".to_string());
    }

    verify_chain_key_ecdsa_signature_enabled(input)
}

#[cfg(any(feature = "auth-chain-key-ecdsa", test))]
fn verify_chain_key_ecdsa_signature_enabled(
    input: ChainKeySignatureVerificationInput<'_>,
) -> Result<(), String> {
    ic_auth::chain_key_signature::verify_secp256k1_prehash(
        input.public_key,
        &input.message_hash,
        input.signature,
    )
    .map_err(|error| format!("chain-key ECDSA signature verification failed: {error}"))
}

#[cfg(not(any(feature = "auth-chain-key-ecdsa", test)))]
fn verify_chain_key_ecdsa_signature_enabled(
    input: ChainKeySignatureVerificationInput<'_>,
) -> Result<(), String> {
    let _ = (input.public_key, input.message_hash, input.signature);
    Err(
        "chain-key ECDSA verification support is not enabled; enable the `auth-chain-key-ecdsa` feature"
            .to_string(),
    )
}

pub(in crate::ops::auth) fn verify_chain_key_ecdsa_public_key_shape(
    public_key: &[u8],
) -> Result<(), String> {
    verify_chain_key_ecdsa_public_key_shape_enabled(public_key)
}

#[cfg(any(feature = "auth-chain-key-ecdsa", test))]
fn verify_chain_key_ecdsa_public_key_shape_enabled(public_key: &[u8]) -> Result<(), String> {
    ic_auth::chain_key_signature::validate_secp256k1_public_key(public_key)
        .map_err(|error| format!("invalid chain-key secp256k1 public key: {error}"))
}

#[cfg(not(any(feature = "auth-chain-key-ecdsa", test)))]
fn verify_chain_key_ecdsa_public_key_shape_enabled(_public_key: &[u8]) -> Result<(), String> {
    Err(
        "chain-key ECDSA public-key validation support is not enabled; enable the `auth-chain-key-ecdsa` feature"
            .to_string(),
    )
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        dto::auth::{
            ChainKeyBatchWitnessStepV1, ChainKeyBatchWitnessV1, ChainKeyKeyId,
            ChainKeyRootSignatureV1, DelegationCert, RootKeyPolicyV1, RootProof,
        },
    };
    use crate::{
        dto::auth::{
            ChainKeyBatchHeaderV1, ChainKeyDelegationCertV1, DelegatedRoleGrant,
            DelegationAudience, IcChainKeyBatchSignatureProofV1, IssuerProofAlgorithm,
            IssuerProofBinding,
        },
        ops::auth::{
            AuthChainKeyRootVerifierConfig,
            delegated::{
                active_proof::{
                    InstallActiveDelegationProofError, InstallActiveDelegationProofInput,
                    install_active_delegation_proof,
                },
                canonical::{
                    chain_key_batch_header_hash, chain_key_delegation_cert_hash,
                    chain_key_derivation_path_hash, issuer_proof_binding_hash,
                },
                cert_rules::DelegatedAuthTtlLimits,
                root_proof::{
                    RootDelegationProofError, VerifyRootDelegationProofInput,
                    verify_root_delegation_proof,
                },
            },
        },
    };
    use canic_contracts::ids::{BuildNetwork, CanisterRole};
    use ic_auth::delegation::DelegationProofError;
    use k256::ecdsa::{SigningKey, signature::hazmat::PrehashSigner};

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn grant(role: &str, scopes: &[&str]) -> DelegatedRoleGrant {
        DelegatedRoleGrant {
            target: CanisterRole::owned(role.to_string()),
            scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
        }
    }

    fn policy() -> AuthChainKeyRootVerifierConfig {
        AuthChainKeyRootVerifierConfig {
            policy: RootKeyPolicyV1 {
                root_canister_id: p(1),
                algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
                key_id: ChainKeyKeyId {
                    name: "test_key_1".to_string(),
                },
                derivation_path_hash: chain_key_derivation_path_hash(&derivation_path()).unwrap(),
                public_key: SigningKey::from_slice(&[7; 32])
                    .unwrap()
                    .verifying_key()
                    .to_encoded_point(true)
                    .as_bytes()
                    .to_vec(),
                key_version: 4,
                min_accepted_key_version: 4,
                min_accepted_proof_epoch: 10,
                min_accepted_registry_epoch: 11,
                valid_from_ns: 1,
                accept_until_ns: 1_000,
                build_network: BuildNetwork::Local,
                max_revocation_latency_ns: 500,
            },
            allow_test_chain_key: true,
        }
    }

    fn derivation_path() -> Vec<Vec<u8>> {
        vec![b"canic".to_vec(), b"delegation".to_vec()]
    }

    fn cert() -> DelegationCert {
        let issuer_proof_alg = IssuerProofAlgorithm::IcCanisterSignatureV1;
        let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 { seed_hash: [5; 32] };
        let issuer_proof_binding_hash =
            issuer_proof_binding_hash(p(3), issuer_proof_alg, issuer_proof_binding).unwrap();

        DelegationCert {
            root_pid: p(1),
            issuer_pid: p(3),
            issuer_proof_alg,
            issuer_proof_binding_hash,
            issuer_proof_binding,
            issued_at_ns: 100,
            not_before_ns: 100,
            expires_at_ns: 500,
            max_token_ttl_ns: 120,
            aud: DelegationAudience::Fleet(crate::test::support::fleet_key(1)),
            grants: vec![grant("project_instance", &["read", "write"])],
        }
    }

    fn proof_for_cert(cert: &DelegationCert, policy: &AuthChainKeyRootVerifierConfig) -> RootProof {
        let policy = &policy.policy;
        let batch_id = [7; 32];
        let registry_hash = [8; 32];
        let delegation_cert = ChainKeyDelegationCertV1 {
            root_canister_id: cert.root_pid,
            issuer_canister_id: cert.issuer_pid,
            proof_epoch: 10,
            issuer_proof_algorithm: cert.issuer_proof_alg,
            issuer_proof_binding_hash: cert.issuer_proof_binding_hash,
            issuer_proof_binding: cert.issuer_proof_binding,
            max_token_ttl_ns: cert.max_token_ttl_ns,
            audience: cert.aud.clone(),
            grants: cert.grants.clone(),
            not_before_ns: cert.not_before_ns,
            expires_at_ns: cert.expires_at_ns,
            registry_epoch: 11,
            registry_hash,
        };
        let tree_root = chain_key_delegation_cert_hash(&delegation_cert).unwrap();
        let header = ChainKeyBatchHeaderV1 {
            schema_version: 1,
            root_canister_id: cert.root_pid,
            batch_id,
            proof_epoch: 10,
            registry_epoch: 11,
            registry_hash,
            tree_root,
            not_before_ns: cert.not_before_ns,
            expires_at_ns: cert.expires_at_ns,
            algorithm: policy.algorithm,
            key_id: policy.key_id.clone(),
            derivation_path_hash: policy.derivation_path_hash,
            key_version: policy.key_version,
        };

        let signature: k256::ecdsa::Signature = SigningKey::from_slice(&[7; 32])
            .unwrap()
            .sign_prehash(&chain_key_batch_header_hash(&header).unwrap())
            .unwrap();
        RootProof::IcChainKeyBatchSignatureV1(IcChainKeyBatchSignatureProofV1 {
            header,
            delegation_cert,
            issuer_witness: ChainKeyBatchWitnessV1 { steps: Vec::new() },
            signature: ChainKeyRootSignatureV1 {
                algorithm: policy.algorithm,
                key_id: policy.key_id.clone(),
                derivation_path: derivation_path(),
                public_key: policy.public_key.clone(),
                signature: signature.to_bytes().to_vec(),
            },
        })
    }

    fn verify(
        cert: &DelegationCert,
        proof: &RootProof,
        policy: &AuthChainKeyRootVerifierConfig,
    ) -> Result<(), RootDelegationProofError> {
        verify_root_delegation_proof(VerifyRootDelegationProofInput {
            cert,
            root_proof: proof,
            expected_issuer: cert.issuer_pid,
            verifier: policy,
            now_ns: 150,
            ttl_limits: DelegatedAuthTtlLimits {
                max_cert_ttl_ns: 600,
                max_token_ttl_ns: 120,
            },
        })
        .map(|_| ())
    }

    fn mutate_proof(
        proof: &mut RootProof,
        mut f: impl FnMut(&mut IcChainKeyBatchSignatureProofV1),
    ) {
        let RootProof::IcChainKeyBatchSignatureV1(proof) = proof;
        f(proof);
    }

    #[test]
    fn chain_key_batch_root_proof_accepts_single_leaf_batch() {
        let cert = cert();
        let policy = policy();
        let proof = proof_for_cert(&cert, &policy);
        verify(&cert, &proof, &policy).expect("actual signed single-leaf proof must verify");
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_wrong_delegation_cert() {
        let cert = cert();
        let policy = policy();
        let mut proof = proof_for_cert(&cert, &policy);
        mutate_proof(&mut proof, |proof| {
            proof.delegation_cert.issuer_canister_id = p(4);
            proof.header.tree_root =
                chain_key_delegation_cert_hash(&proof.delegation_cert).unwrap();
        });

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(RootDelegationProofError::Verification(
                DelegationProofError::BindingMismatch {
                    field: "issuer_canister_id"
                }
            ))
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_wrong_audience() {
        let cert = cert();
        let policy = policy();
        let mut proof = proof_for_cert(&cert, &policy);
        mutate_proof(&mut proof, |proof| {
            proof.delegation_cert.audience =
                DelegationAudience::Fleet(crate::test::support::fleet_key(2));
            proof.header.tree_root =
                chain_key_delegation_cert_hash(&proof.delegation_cert).unwrap();
        });

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(RootDelegationProofError::Verification(
                DelegationProofError::BindingMismatch {
                    field: "leaf_audience"
                }
            ))
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_wrong_grants() {
        let cert = cert();
        let policy = policy();
        let mut proof = proof_for_cert(&cert, &policy);
        mutate_proof(&mut proof, |proof| {
            proof.delegation_cert.grants[0]
                .scopes
                .push("zz_extra".to_string());
            proof.header.tree_root =
                chain_key_delegation_cert_hash(&proof.delegation_cert).unwrap();
        });

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(RootDelegationProofError::Verification(
                DelegationProofError::BindingMismatch {
                    field: "leaf_grants"
                }
            ))
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_wrong_root_canister_binding() {
        let cert = cert();
        let policy = policy();
        let mut proof = proof_for_cert(&cert, &policy);
        mutate_proof(&mut proof, |proof| {
            proof.header.root_canister_id = p(9);
        });

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(RootDelegationProofError::Verification(
                DelegationProofError::BindingMismatch {
                    field: "root_canister_id"
                }
            ))
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_leaf_outside_header_window() {
        let cert = cert();
        let policy = policy();
        let mut proof = proof_for_cert(&cert, &policy);
        mutate_proof(&mut proof, |proof| {
            proof.header.not_before_ns = cert.not_before_ns + 1;
        });

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(RootDelegationProofError::Verification(
                DelegationProofError::BindingMismatch {
                    field: "leaf_batch_window"
                }
            ))
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_wrong_key_id() {
        let cert = cert();
        let policy = policy();
        let mut proof = proof_for_cert(&cert, &policy);
        mutate_proof(&mut proof, |proof| {
            proof.header.key_id.name = "other_key".to_string();
            proof.signature.key_id.name = "other_key".to_string();
        });

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(RootDelegationProofError::Verification(
                DelegationProofError::RootKeyPolicyBindingMismatch { field: "key_id" }
            ))
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_wrong_derivation_path_hash() {
        let cert = cert();
        let policy = policy();
        let mut proof = proof_for_cert(&cert, &policy);
        mutate_proof(&mut proof, |proof| {
            proof.signature.derivation_path.push(b"extra".to_vec());
        });

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(RootDelegationProofError::Verification(
                DelegationProofError::RootSignatureBindingMismatch {
                    field: "derivation_path_hash",
                }
            ))
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_wrong_public_key() {
        let cert = cert();
        let policy = policy();
        let mut proof = proof_for_cert(&cert, &policy);
        mutate_proof(&mut proof, |proof| {
            proof.signature.public_key[0] ^= 1;
        });

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(RootDelegationProofError::Verification(
                DelegationProofError::BindingMismatch {
                    field: "root_public_key"
                }
            ))
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_stale_proof_epoch() {
        let cert = cert();
        let baseline_policy = policy();
        let mut policy = baseline_policy.clone();
        policy.policy.min_accepted_proof_epoch = 11;
        let proof = proof_for_cert(&cert, &baseline_policy);

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(RootDelegationProofError::Verification(
                DelegationProofError::StaleAuthority {
                    field: "proof_epoch"
                }
            ))
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_stale_registry_epoch() {
        let cert = cert();
        let baseline_policy = policy();
        let mut policy = baseline_policy.clone();
        policy.policy.min_accepted_registry_epoch = 12;
        let proof = proof_for_cert(&cert, &baseline_policy);

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(RootDelegationProofError::Verification(
                DelegationProofError::StaleAuthority {
                    field: "registry_epoch"
                }
            ))
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_root_proof_ttl_above_revocation_latency() {
        let cert = cert();
        let baseline_policy = policy();
        let mut policy = baseline_policy.clone();
        policy.policy.max_revocation_latency_ns = 399;
        let proof = proof_for_cert(&cert, &baseline_policy);

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(RootDelegationProofError::Verification(
                DelegationProofError::TtlExceeded {
                    target: "batch",
                    ttl_ns: 400,
                    max_ttl_ns: 399,
                }
            ))
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_mainnet_test_key_policy() {
        let cert = cert();
        let mut policy = policy();
        policy.policy.build_network = BuildNetwork::Ic;
        let proof = proof_for_cert(&cert, &policy);

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(RootDelegationProofError::RootPolicyRejected)
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_malformed_ecdsa_signature_length() {
        let cert = cert();
        let policy = policy();
        let mut proof = proof_for_cert(&cert, &policy);
        mutate_proof(&mut proof, |proof| {
            proof.signature.signature = vec![9; 63];
        });

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(RootDelegationProofError::Verification(
                DelegationProofError::RootSignatureInvalid
            ))
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_high_s_ecdsa_signature() {
        let cert = cert();
        let policy = policy();
        let mut proof = proof_for_cert(&cert, &policy);
        mutate_proof(&mut proof, |proof| {
            let mut signature = proof.signature.signature.clone();
            signature[32..].fill(0);
            signature[32] = 0x80;
            proof.signature.signature = signature;
        });

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(RootDelegationProofError::Verification(
                DelegationProofError::HighSSignature
            ))
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_invalid_witness() {
        let cert = cert();
        let policy = policy();
        let mut proof = proof_for_cert(&cert, &policy);
        mutate_proof(&mut proof, |proof| {
            proof
                .issuer_witness
                .steps
                .push(ChainKeyBatchWitnessStepV1::RightSibling([1; 32]));
        });

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(RootDelegationProofError::Verification(
                DelegationProofError::InvalidMerkleWitness
            ))
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_signature_failure() {
        let cert = cert();
        let policy = policy();
        let mut proof = proof_for_cert(&cert, &policy);
        mutate_proof(&mut proof, |proof| proof.signature.signature[0] ^= 1);
        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(RootDelegationProofError::Verification(
                DelegationProofError::RootSignatureInvalid
            ))
        );
    }

    #[test]
    fn runtime_root_verification_distinguishes_signature_metadata_from_enrollment() {
        let cert = cert();
        let policy = policy();
        let original = proof_for_cert(&cert, &policy);
        let cfg = crate::ops::auth::AuthProofVerifierConfig {
            build_network: BuildNetwork::Local,
            root_canister_id: policy.policy.root_canister_id,
            ic_root_public_key_raw: vec![1; 96],
            chain_key_root: Some(policy),
        };
        let invalid = crate::diagnostics::codes::REQUEST_INVALID;
        let stale = crate::diagnostics::codes::SECURITY_CONFLICT;
        for (signature_only, conflicting_metadata, derivation, expected) in [
            (true, false, false, invalid),
            (false, false, false, stale),
            (true, false, true, invalid),
            (false, false, true, stale),
            (false, true, false, invalid),
            (false, true, true, invalid),
        ] {
            let mut proof = original.clone();
            mutate_proof(&mut proof, |proof| {
                if derivation {
                    proof.signature.derivation_path.push(b"other".to_vec());
                    if !signature_only {
                        proof.header.derivation_path_hash =
                            chain_key_derivation_path_hash(&proof.signature.derivation_path)
                                .unwrap();
                    }
                    if conflicting_metadata {
                        proof.signature.derivation_path.push(b"third".to_vec());
                    }
                } else {
                    proof.signature.key_id.name = "other".to_string();
                    if !signature_only {
                        proof.header.key_id = proof.signature.key_id.clone();
                    }
                    if conflicting_metadata {
                        proof.signature.key_id.name = "third".to_string();
                    }
                }
            });
            let field = if derivation {
                "derivation_path_hash"
            } else {
                "key_id"
            };
            let expected_cause = if signature_only || conflicting_metadata {
                DelegationProofError::RootSignatureBindingMismatch { field }
            } else {
                DelegationProofError::RootKeyPolicyBindingMismatch { field }
            };
            let installed = install_active_delegation_proof(InstallActiveDelegationProofInput {
                proof: crate::dto::auth::DelegationProof {
                    cert: cert.clone(),
                    root_proof: proof.clone(),
                },
                installed_by: cert.root_pid,
                this_canister: cert.issuer_pid,
                now_ns: 150,
                verifier: cfg.chain_key_root.as_ref().unwrap(),
            })
            .expect_err("installation must preserve the canonical binding cause");
            assert_eq!(
                &installed,
                &InstallActiveDelegationProofError::RootProofInvalid(expected_cause)
            );
            let cause = crate::ops::auth::AuthOps::verify_delegation_root_proof(
                &cert,
                &proof,
                &cfg,
                150,
                DelegatedAuthTtlLimits {
                    max_cert_ttl_ns: 600,
                    max_token_ttl_ns: 120,
                },
            )
            .expect_err("unbound signature metadata or stale enrollment must reject");
            assert_eq!(cause.public_error().code(), expected.raw_code());
        }
    }
}
