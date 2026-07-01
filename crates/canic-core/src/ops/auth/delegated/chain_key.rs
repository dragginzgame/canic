//! Module: ops::auth::delegated::chain_key
//!
//! Responsibility: verify the structural contract for chain-key batch root proofs.
//! Does not own: management-canister signing, endpoint auth, or issuer proof verification.
//! Boundary: pure delegated-auth helper used before algorithm-specific signature checks.

use super::canonical::{
    CanonicalAuthError, chain_key_batch_header_hash, chain_key_delegation_cert_hash,
    chain_key_derivation_path_hash,
};
use crate::{
    cdk::types::Principal,
    dto::auth::{
        ChainKeyAlgorithm, ChainKeyBatchWitnessStepV1, ChainKeyBatchWitnessV1, ChainKeyKeyId,
        ChainKeyRootSignatureV1, DelegationCert, RootProof,
    },
    ids::BuildNetwork,
    ops::auth::AUTH_TIME_SKEW_ALLOWANCE_NS,
};
#[cfg(any(feature = "auth-chain-key-ecdsa", test))]
use k256::ecdsa::{
    Signature as K256EcdsaSignature, VerifyingKey as K256VerifyingKey,
    signature::hazmat::PrehashVerifier,
};
use sha2::{Digest, Sha256};
use thiserror::Error;

const CHAIN_KEY_BATCH_SCHEMA_VERSION_V1: u16 = 1;
const PRODUCTION_ECDSA_KEY_ID: &str = "key_1";
const TEST_ECDSA_KEY_ID: &str = "test_key_1";
const ECDSA_SECP256K1_SIGNATURE_LENGTH: usize = 64;
const SECP256K1_ORDER_HALF: [u8; 32] = [
    0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0x5d, 0x57, 0x6e, 0x73, 0x57, 0xa4, 0x50, 0x1d, 0xdf, 0xe9, 0x2f, 0x46, 0x68, 0x1b, 0x20, 0xa0,
];

///
/// ChainKeyRootVerifierPolicy
///
/// Local verifier trust policy for accepted root chain-key batch proofs.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::ops::auth) struct ChainKeyRootVerifierPolicy {
    pub root_canister_id: Principal,
    pub algorithm: ChainKeyAlgorithm,
    pub key_id: ChainKeyKeyId,
    pub derivation_path_hash: [u8; 32],
    pub public_key: Vec<u8>,
    pub key_version: u64,
    pub min_accepted_key_version: u64,
    pub min_accepted_proof_epoch: u64,
    pub min_accepted_registry_epoch: u64,
    pub valid_from_ns: u64,
    pub accept_until_ns: u64,
    pub build_network: BuildNetwork,
    pub allow_test_chain_key: bool,
    pub max_revocation_latency_ns: u64,
}

///
/// VerifyChainKeyBatchRootProofInput
///
/// Input for verifying that a chain-key root proof authorizes a delegation cert.
///

pub(in crate::ops::auth) struct VerifyChainKeyBatchRootProofInput<'a> {
    pub cert: &'a DelegationCert,
    pub root_proof: &'a RootProof,
    pub policy: &'a ChainKeyRootVerifierPolicy,
    pub now_ns: u64,
}

///
/// ChainKeySignatureVerificationInput
///
/// Algorithm-specific signature verification payload after structural checks.
///

pub(in crate::ops::auth) struct ChainKeySignatureVerificationInput<'a> {
    pub algorithm: ChainKeyAlgorithm,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "structural chain-key proof verification already checks key id before ECDSA verification"
        )
    )]
    pub key_id: &'a ChainKeyKeyId,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "structural chain-key proof verification already checks derivation path before ECDSA verification"
        )
    )]
    pub derivation_path: &'a [Vec<u8>],
    pub public_key: &'a [u8],
    pub message_hash: [u8; 32],
    pub signature: &'a [u8],
}

///
/// ChainKeyRootProofError
///
/// Typed failure surface for root chain-key batch proof verification.
///

#[derive(Debug, Eq, Error, PartialEq)]
pub(in crate::ops::auth) enum ChainKeyRootProofError {
    #[error("delegated auth chain-key root proof schema version mismatch")]
    SchemaVersionMismatch { expected: u16, found: u16 },
    #[error("delegated auth chain-key root canister mismatch")]
    RootCanisterMismatch {
        expected: Principal,
        found: Principal,
    },
    #[error("delegated auth chain-key issuer canister mismatch")]
    IssuerCanisterMismatch {
        expected: Principal,
        found: Principal,
    },
    #[error("delegated auth chain-key header/delegation-cert mismatch: {field}")]
    HeaderDelegationCertMismatch { field: &'static str },
    #[error("delegated auth chain-key header/signature mismatch: {field}")]
    HeaderSignatureMismatch { field: &'static str },
    #[error("delegated auth chain-key active-cert/delegation-cert mismatch: {field}")]
    DelegationCertMismatch { field: &'static str },
    #[error("delegated auth chain-key verifier policy mismatch: {field}")]
    PolicyMismatch { field: &'static str },
    #[error("delegated auth chain-key test key is rejected for this verifier policy")]
    TestKeyRejected,
    #[error("delegated auth chain-key proof epoch {found} is below verifier minimum {min}")]
    ProofEpochTooOld { min: u64, found: u64 },
    #[error("delegated auth chain-key key version {found} is below verifier minimum {min}")]
    KeyVersionTooOld { min: u64, found: u64 },
    #[error("delegated auth chain-key registry epoch {found} is below verifier minimum {min}")]
    RegistryEpochTooOld { min: u64, found: u64 },
    #[error("delegated auth chain-key {target} window is invalid")]
    InvalidWindow { target: &'static str },
    #[error("delegated auth chain-key {target} is not yet valid")]
    NotYetValid { target: &'static str },
    #[error("delegated auth chain-key {target} expired")]
    Expired { target: &'static str },
    #[error("delegated auth chain-key proof ttl {ttl_ns}ns exceeds verifier max {max_ttl_ns}ns")]
    RootProofTtlExceeded { ttl_ns: u64, max_ttl_ns: u64 },
    #[error("delegated auth chain-key delegation cert is outside batch window")]
    DelegationCertOutsideBatchWindow,
    #[error("delegated auth chain-key batch Merkle witness is invalid")]
    InvalidMerkleWitness,
    #[error("delegated auth chain-key ECDSA signature length {len} is invalid")]
    InvalidSignatureLength { len: usize },
    #[error("delegated auth chain-key ECDSA signature {component} component is zero")]
    ZeroSignatureComponent { component: &'static str },
    #[error("delegated auth chain-key ECDSA signature has high-s encoding")]
    HighSSignature,
    #[error("delegated auth chain-key signature invalid: {0}")]
    SignatureInvalid(String),
    #[error(transparent)]
    Canonical(#[from] CanonicalAuthError),
}

pub(in crate::ops::auth) fn verify_chain_key_batch_root_proof<F>(
    input: VerifyChainKeyBatchRootProofInput<'_>,
    mut verify_signature: F,
) -> Result<(), ChainKeyRootProofError>
where
    F: FnMut(ChainKeySignatureVerificationInput<'_>) -> Result<(), String>,
{
    let RootProof::IcChainKeyBatchSignatureV1(proof) = input.root_proof;

    let header = &proof.header;
    let delegation_cert = &proof.delegation_cert;
    let signature = &proof.signature;
    let policy = input.policy;

    if header.schema_version != CHAIN_KEY_BATCH_SCHEMA_VERSION_V1 {
        return Err(ChainKeyRootProofError::SchemaVersionMismatch {
            expected: CHAIN_KEY_BATCH_SCHEMA_VERSION_V1,
            found: header.schema_version,
        });
    }
    if input.cert.root_pid != policy.root_canister_id {
        return Err(ChainKeyRootProofError::RootCanisterMismatch {
            expected: policy.root_canister_id,
            found: input.cert.root_pid,
        });
    }
    if header.root_canister_id != policy.root_canister_id {
        return Err(ChainKeyRootProofError::RootCanisterMismatch {
            expected: policy.root_canister_id,
            found: header.root_canister_id,
        });
    }
    if delegation_cert.root_canister_id != header.root_canister_id {
        return Err(ChainKeyRootProofError::HeaderDelegationCertMismatch {
            field: "root_canister_id",
        });
    }

    verify_policy_window(policy, input.now_ns)?;
    verify_key_id_network(policy)?;
    verify_window(
        "batch",
        header.not_before_ns,
        header.expires_at_ns,
        input.now_ns,
    )?;
    verify_window(
        "delegation_cert",
        delegation_cert.not_before_ns,
        delegation_cert.expires_at_ns,
        input.now_ns,
    )?;
    verify_header_leaf_binding(header, delegation_cert)?;
    verify_cert_leaf_binding(input.cert, delegation_cert)?;
    verify_header_signature_binding(header, signature)?;
    verify_policy_binding(policy, header, signature)?;
    verify_root_proof_ttl(policy, header.not_before_ns, header.expires_at_ns)?;
    verify_chain_key_ecdsa_signature_shape(&signature.signature)?;

    let leaf_hash = chain_key_delegation_cert_hash(delegation_cert)?;
    if chain_key_batch_witness_root(leaf_hash, &proof.issuer_witness) != header.tree_root {
        return Err(ChainKeyRootProofError::InvalidMerkleWitness);
    }

    verify_signature(ChainKeySignatureVerificationInput {
        algorithm: signature.algorithm,
        key_id: &signature.key_id,
        derivation_path: &signature.derivation_path,
        public_key: &signature.public_key,
        message_hash: chain_key_batch_header_hash(header),
        signature: &signature.signature,
    })
    .map_err(ChainKeyRootProofError::SignatureInvalid)
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
    let verifying_key = K256VerifyingKey::from_sec1_bytes(input.public_key)
        .map_err(|err| format!("invalid chain-key secp256k1 public key: {err}"))?;
    let signature = K256EcdsaSignature::from_slice(input.signature)
        .map_err(|err| format!("invalid chain-key ECDSA signature encoding: {err}"))?;

    verifying_key
        .verify_prehash(&input.message_hash, &signature)
        .map_err(|err| format!("chain-key ECDSA signature verification failed: {err}"))
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
    K256VerifyingKey::from_sec1_bytes(public_key)
        .map(|_| ())
        .map_err(|err| format!("invalid chain-key secp256k1 public key: {err}"))
}

#[cfg(not(any(feature = "auth-chain-key-ecdsa", test)))]
fn verify_chain_key_ecdsa_public_key_shape_enabled(_public_key: &[u8]) -> Result<(), String> {
    Err(
        "chain-key ECDSA public-key validation support is not enabled; enable the `auth-chain-key-ecdsa` feature"
            .to_string(),
    )
}

const fn verify_policy_window(
    policy: &ChainKeyRootVerifierPolicy,
    now_ns: u64,
) -> Result<(), ChainKeyRootProofError> {
    verify_window(
        "root_key_policy",
        policy.valid_from_ns,
        policy.accept_until_ns,
        now_ns,
    )
}

fn verify_key_id_network(
    policy: &ChainKeyRootVerifierPolicy,
) -> Result<(), ChainKeyRootProofError> {
    if policy.build_network == BuildNetwork::Ic {
        if policy.key_id.name != PRODUCTION_ECDSA_KEY_ID {
            return Err(ChainKeyRootProofError::TestKeyRejected);
        }
        return Ok(());
    }

    if policy.key_id.name == TEST_ECDSA_KEY_ID && !policy.allow_test_chain_key {
        return Err(ChainKeyRootProofError::TestKeyRejected);
    }
    Ok(())
}

const fn verify_window(
    target: &'static str,
    not_before_ns: u64,
    expires_at_ns: u64,
    now_ns: u64,
) -> Result<(), ChainKeyRootProofError> {
    if not_before_ns >= expires_at_ns {
        return Err(ChainKeyRootProofError::InvalidWindow { target });
    }
    if not_before_ns > now_ns.saturating_add(AUTH_TIME_SKEW_ALLOWANCE_NS) {
        return Err(ChainKeyRootProofError::NotYetValid { target });
    }
    if now_ns >= expires_at_ns {
        return Err(ChainKeyRootProofError::Expired { target });
    }
    Ok(())
}

fn verify_header_leaf_binding(
    header: &crate::dto::auth::ChainKeyBatchHeaderV1,
    leaf: &crate::dto::auth::ChainKeyDelegationCertV1,
) -> Result<(), ChainKeyRootProofError> {
    if leaf.proof_epoch != header.proof_epoch {
        return Err(ChainKeyRootProofError::HeaderDelegationCertMismatch {
            field: "proof_epoch",
        });
    }
    if leaf.registry_epoch != header.registry_epoch {
        return Err(ChainKeyRootProofError::HeaderDelegationCertMismatch {
            field: "registry_epoch",
        });
    }
    if leaf.registry_hash != header.registry_hash {
        return Err(ChainKeyRootProofError::HeaderDelegationCertMismatch {
            field: "registry_hash",
        });
    }
    if leaf.not_before_ns < header.not_before_ns || leaf.expires_at_ns > header.expires_at_ns {
        return Err(ChainKeyRootProofError::DelegationCertOutsideBatchWindow);
    }
    Ok(())
}

fn verify_cert_leaf_binding(
    cert: &DelegationCert,
    leaf: &crate::dto::auth::ChainKeyDelegationCertV1,
) -> Result<(), ChainKeyRootProofError> {
    if leaf.issuer_canister_id != cert.issuer_pid {
        return Err(ChainKeyRootProofError::IssuerCanisterMismatch {
            expected: cert.issuer_pid,
            found: leaf.issuer_canister_id,
        });
    }
    if leaf.issuer_proof_algorithm != cert.issuer_proof_alg {
        return Err(ChainKeyRootProofError::DelegationCertMismatch {
            field: "issuer_proof_algorithm",
        });
    }
    if leaf.issuer_proof_binding_hash != cert.issuer_proof_binding_hash {
        return Err(ChainKeyRootProofError::DelegationCertMismatch {
            field: "issuer_proof_binding_hash",
        });
    }
    if leaf.issuer_proof_binding != cert.issuer_proof_binding {
        return Err(ChainKeyRootProofError::DelegationCertMismatch {
            field: "issuer_proof_binding",
        });
    }
    if leaf.max_token_ttl_ns != cert.max_token_ttl_ns {
        return Err(ChainKeyRootProofError::DelegationCertMismatch {
            field: "max_token_ttl_ns",
        });
    }
    if leaf.audience != cert.aud {
        return Err(ChainKeyRootProofError::DelegationCertMismatch { field: "audience" });
    }
    if leaf.grants != cert.grants {
        return Err(ChainKeyRootProofError::DelegationCertMismatch { field: "grants" });
    }
    if leaf.not_before_ns != cert.not_before_ns {
        return Err(ChainKeyRootProofError::DelegationCertMismatch {
            field: "not_before_ns",
        });
    }
    if leaf.expires_at_ns != cert.expires_at_ns {
        return Err(ChainKeyRootProofError::DelegationCertMismatch {
            field: "expires_at_ns",
        });
    }
    Ok(())
}

fn verify_header_signature_binding(
    header: &crate::dto::auth::ChainKeyBatchHeaderV1,
    signature: &ChainKeyRootSignatureV1,
) -> Result<(), ChainKeyRootProofError> {
    if signature.algorithm != header.algorithm {
        return Err(ChainKeyRootProofError::HeaderSignatureMismatch { field: "algorithm" });
    }
    if signature.key_id != header.key_id {
        return Err(ChainKeyRootProofError::HeaderSignatureMismatch { field: "key_id" });
    }
    if chain_key_derivation_path_hash(&signature.derivation_path) != header.derivation_path_hash {
        return Err(ChainKeyRootProofError::HeaderSignatureMismatch {
            field: "derivation_path_hash",
        });
    }
    Ok(())
}

fn verify_policy_binding(
    policy: &ChainKeyRootVerifierPolicy,
    header: &crate::dto::auth::ChainKeyBatchHeaderV1,
    signature: &ChainKeyRootSignatureV1,
) -> Result<(), ChainKeyRootProofError> {
    if header.algorithm != policy.algorithm {
        return Err(ChainKeyRootProofError::PolicyMismatch { field: "algorithm" });
    }
    if header.key_id != policy.key_id {
        return Err(ChainKeyRootProofError::PolicyMismatch { field: "key_id" });
    }
    if header.derivation_path_hash != policy.derivation_path_hash {
        return Err(ChainKeyRootProofError::PolicyMismatch {
            field: "derivation_path_hash",
        });
    }
    if signature.public_key != policy.public_key {
        return Err(ChainKeyRootProofError::PolicyMismatch {
            field: "public_key",
        });
    }
    if header.key_version != policy.key_version {
        return Err(ChainKeyRootProofError::PolicyMismatch {
            field: "key_version",
        });
    }
    if header.proof_epoch < policy.min_accepted_proof_epoch {
        return Err(ChainKeyRootProofError::ProofEpochTooOld {
            min: policy.min_accepted_proof_epoch,
            found: header.proof_epoch,
        });
    }
    if header.key_version < policy.min_accepted_key_version {
        return Err(ChainKeyRootProofError::KeyVersionTooOld {
            min: policy.min_accepted_key_version,
            found: header.key_version,
        });
    }
    if header.registry_epoch < policy.min_accepted_registry_epoch {
        return Err(ChainKeyRootProofError::RegistryEpochTooOld {
            min: policy.min_accepted_registry_epoch,
            found: header.registry_epoch,
        });
    }
    Ok(())
}

fn verify_root_proof_ttl(
    policy: &ChainKeyRootVerifierPolicy,
    not_before_ns: u64,
    expires_at_ns: u64,
) -> Result<(), ChainKeyRootProofError> {
    let ttl_ns = expires_at_ns
        .checked_sub(not_before_ns)
        .ok_or(ChainKeyRootProofError::InvalidWindow { target: "batch" })?;
    if ttl_ns > policy.max_revocation_latency_ns {
        return Err(ChainKeyRootProofError::RootProofTtlExceeded {
            ttl_ns,
            max_ttl_ns: policy.max_revocation_latency_ns,
        });
    }
    Ok(())
}

pub(in crate::ops::auth) fn verify_chain_key_ecdsa_signature_shape(
    signature: &[u8],
) -> Result<(), ChainKeyRootProofError> {
    if signature.len() != ECDSA_SECP256K1_SIGNATURE_LENGTH {
        return Err(ChainKeyRootProofError::InvalidSignatureLength {
            len: signature.len(),
        });
    }

    let (r, s) = signature.split_at(32);
    if is_zero_32(r) {
        return Err(ChainKeyRootProofError::ZeroSignatureComponent { component: "r" });
    }
    if is_zero_32(s) {
        return Err(ChainKeyRootProofError::ZeroSignatureComponent { component: "s" });
    }
    if greater_than_32(s, &SECP256K1_ORDER_HALF) {
        return Err(ChainKeyRootProofError::HighSSignature);
    }
    Ok(())
}

fn is_zero_32(bytes: &[u8]) -> bool {
    debug_assert_eq!(bytes.len(), 32);
    bytes.iter().all(|byte| *byte == 0)
}

fn greater_than_32(left: &[u8], right: &[u8; 32]) -> bool {
    debug_assert_eq!(left.len(), 32);
    left > right.as_slice()
}

fn chain_key_batch_witness_root(leaf_hash: [u8; 32], witness: &ChainKeyBatchWitnessV1) -> [u8; 32] {
    witness
        .steps
        .iter()
        .fold(leaf_hash, |current, step| match step {
            ChainKeyBatchWitnessStepV1::LeftSibling(sibling) => {
                chain_key_batch_node_hash(*sibling, current)
            }
            ChainKeyBatchWitnessStepV1::RightSibling(sibling) => {
                chain_key_batch_node_hash(current, *sibling)
            }
        })
}

fn chain_key_batch_node_hash(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([1]);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::auth::{
            ChainKeyBatchHeaderV1, ChainKeyDelegationCertV1, DelegatedRoleGrant,
            DelegationAudience, IcChainKeyBatchSignatureProofV1, IssuerProofAlgorithm,
            IssuerProofBinding,
        },
        ids::CanisterRole,
        ops::auth::delegated::canonical::issuer_proof_binding_hash,
    };
    use std::cell::Cell;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn grant(role: &str, scopes: &[&str]) -> DelegatedRoleGrant {
        DelegatedRoleGrant {
            target: CanisterRole::owned(role.to_string()),
            scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
        }
    }

    fn policy() -> ChainKeyRootVerifierPolicy {
        ChainKeyRootVerifierPolicy {
            root_canister_id: p(1),
            algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
            key_id: ChainKeyKeyId {
                name: "test_key_1".to_string(),
            },
            derivation_path_hash: chain_key_derivation_path_hash(&derivation_path()),
            public_key: vec![2; 33],
            key_version: 4,
            min_accepted_key_version: 4,
            min_accepted_proof_epoch: 10,
            min_accepted_registry_epoch: 11,
            valid_from_ns: 1,
            accept_until_ns: 1_000,
            build_network: BuildNetwork::Local,
            allow_test_chain_key: true,
            max_revocation_latency_ns: 500,
        }
    }

    fn derivation_path() -> Vec<Vec<u8>> {
        vec![b"canic".to_vec(), b"delegation".to_vec()]
    }

    fn cert() -> DelegationCert {
        let issuer_proof_alg = IssuerProofAlgorithm::IcCanisterSignatureV1;
        let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 { seed_hash: [5; 32] };
        let issuer_proof_binding_hash =
            issuer_proof_binding_hash(p(3), issuer_proof_alg, issuer_proof_binding);

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
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![grant("project_instance", &["read", "write"])],
        }
    }

    fn proof_for_cert(cert: &DelegationCert, policy: &ChainKeyRootVerifierPolicy) -> RootProof {
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
            schema_version: CHAIN_KEY_BATCH_SCHEMA_VERSION_V1,
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

        RootProof::IcChainKeyBatchSignatureV1(IcChainKeyBatchSignatureProofV1 {
            header,
            delegation_cert,
            issuer_witness: ChainKeyBatchWitnessV1 { steps: Vec::new() },
            signature: ChainKeyRootSignatureV1 {
                algorithm: policy.algorithm,
                key_id: policy.key_id.clone(),
                derivation_path: derivation_path(),
                public_key: policy.public_key.clone(),
                signature: vec![9; 64],
            },
        })
    }

    fn verify(
        cert: &DelegationCert,
        proof: &RootProof,
        policy: &ChainKeyRootVerifierPolicy,
    ) -> Result<(), ChainKeyRootProofError> {
        verify_chain_key_batch_root_proof(
            VerifyChainKeyBatchRootProofInput {
                cert,
                root_proof: proof,
                policy,
                now_ns: 150,
            },
            |_| Ok(()),
        )
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
        let calls = Cell::new(0);

        verify_chain_key_batch_root_proof(
            VerifyChainKeyBatchRootProofInput {
                cert: &cert,
                root_proof: &proof,
                policy: &policy,
                now_ns: 150,
            },
            |input| {
                calls.set(calls.get() + 1);
                assert_eq!(input.algorithm, policy.algorithm);
                assert_eq!(input.key_id, &policy.key_id);
                assert_eq!(input.derivation_path, derivation_path().as_slice());
                assert_eq!(input.public_key, policy.public_key.as_slice());
                assert_eq!(input.signature, &[9; 64]);
                Ok(())
            },
        )
        .expect("valid chain-key batch proof should verify");

        assert_eq!(calls.get(), 1);
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
            Err(ChainKeyRootProofError::IssuerCanisterMismatch {
                expected: p(3),
                found: p(4),
            })
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_wrong_audience() {
        let cert = cert();
        let policy = policy();
        let mut proof = proof_for_cert(&cert, &policy);
        mutate_proof(&mut proof, |proof| {
            proof.delegation_cert.audience = DelegationAudience::Project("other".to_string());
            proof.header.tree_root =
                chain_key_delegation_cert_hash(&proof.delegation_cert).unwrap();
        });

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(ChainKeyRootProofError::DelegationCertMismatch { field: "audience" })
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
            Err(ChainKeyRootProofError::DelegationCertMismatch { field: "grants" })
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
            Err(ChainKeyRootProofError::RootCanisterMismatch {
                expected: p(1),
                found: p(9),
            })
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
            Err(ChainKeyRootProofError::DelegationCertOutsideBatchWindow)
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
            Err(ChainKeyRootProofError::PolicyMismatch { field: "key_id" })
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
            Err(ChainKeyRootProofError::HeaderSignatureMismatch {
                field: "derivation_path_hash",
            })
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
            Err(ChainKeyRootProofError::PolicyMismatch {
                field: "public_key"
            })
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_stale_proof_epoch() {
        let cert = cert();
        let baseline_policy = policy();
        let policy = ChainKeyRootVerifierPolicy {
            min_accepted_proof_epoch: 11,
            ..baseline_policy.clone()
        };
        let proof = proof_for_cert(&cert, &baseline_policy);

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(ChainKeyRootProofError::ProofEpochTooOld { min: 11, found: 10 })
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_stale_registry_epoch() {
        let cert = cert();
        let baseline_policy = policy();
        let policy = ChainKeyRootVerifierPolicy {
            min_accepted_registry_epoch: 12,
            ..baseline_policy.clone()
        };
        let proof = proof_for_cert(&cert, &baseline_policy);

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(ChainKeyRootProofError::RegistryEpochTooOld { min: 12, found: 11 })
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_root_proof_ttl_above_revocation_latency() {
        let cert = cert();
        let baseline_policy = policy();
        let policy = ChainKeyRootVerifierPolicy {
            max_revocation_latency_ns: 399,
            ..baseline_policy.clone()
        };
        let proof = proof_for_cert(&cert, &baseline_policy);

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(ChainKeyRootProofError::RootProofTtlExceeded {
                ttl_ns: 400,
                max_ttl_ns: 399,
            })
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_mainnet_test_key_policy() {
        let cert = cert();
        let policy = ChainKeyRootVerifierPolicy {
            build_network: BuildNetwork::Ic,
            ..policy()
        };
        let proof = proof_for_cert(&cert, &policy);

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(ChainKeyRootProofError::TestKeyRejected)
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
            Err(ChainKeyRootProofError::InvalidSignatureLength { len: 63 })
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_high_s_ecdsa_signature() {
        let cert = cert();
        let policy = policy();
        let mut proof = proof_for_cert(&cert, &policy);
        mutate_proof(&mut proof, |proof| {
            let mut signature = vec![9; 64];
            signature[32..].copy_from_slice(&[0xff; 32]);
            proof.signature.signature = signature;
        });

        assert_eq!(
            verify(&cert, &proof, &policy),
            Err(ChainKeyRootProofError::HighSSignature)
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
            Err(ChainKeyRootProofError::InvalidMerkleWitness)
        );
    }

    #[test]
    fn chain_key_merkle_witness_root_matches_golden_fixture() {
        let cert = cert();
        let policy = policy();
        let proof = proof_for_cert(&cert, &policy);
        let RootProof::IcChainKeyBatchSignatureV1(proof) = proof;
        let leaf_hash = chain_key_delegation_cert_hash(&proof.delegation_cert).unwrap();
        let witness = ChainKeyBatchWitnessV1 {
            steps: vec![
                ChainKeyBatchWitnessStepV1::LeftSibling([42; 32]),
                ChainKeyBatchWitnessStepV1::RightSibling([43; 32]),
            ],
        };

        assert_eq!(
            leaf_hash,
            [
                153, 26, 229, 175, 151, 99, 103, 193, 39, 115, 15, 221, 247, 160, 5, 134, 10, 209,
                107, 150, 79, 65, 245, 135, 211, 49, 98, 6, 150, 89, 145, 238,
            ]
        );
        assert_eq!(
            chain_key_batch_witness_root(leaf_hash, &witness),
            [
                220, 227, 136, 35, 77, 43, 39, 161, 214, 210, 239, 199, 239, 248, 96, 164, 231, 48,
                194, 190, 203, 40, 147, 159, 24, 9, 250, 229, 250, 148, 219, 81,
            ]
        );
    }

    #[test]
    fn chain_key_batch_root_proof_rejects_signature_failure() {
        let cert = cert();
        let policy = policy();
        let proof = proof_for_cert(&cert, &policy);

        let err = verify_chain_key_batch_root_proof(
            VerifyChainKeyBatchRootProofInput {
                cert: &cert,
                root_proof: &proof,
                policy: &policy,
                now_ns: 150,
            },
            |input| {
                assert_eq!(input.message_hash, {
                    let RootProof::IcChainKeyBatchSignatureV1(proof) = &proof;
                    chain_key_batch_header_hash(&proof.header)
                });
                Err("bad signature".to_string())
            },
        )
        .expect_err("signature callback failure must reject");

        assert_eq!(
            err,
            ChainKeyRootProofError::SignatureInvalid("bad signature".to_string())
        );
    }
}
