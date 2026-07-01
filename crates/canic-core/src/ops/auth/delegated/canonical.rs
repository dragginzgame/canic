//! Module: ops::auth::delegated::canonical
//!
//! Responsibility: encode delegated auth material into canonical hash inputs.
//! Does not own: proof verification, storage, or endpoint authorization.
//! Boundary: pure canonicalization helper for delegated certs, tokens, and proofs.

use crate::{
    cdk::types::Principal,
    dto::auth::{
        ChainKeyAlgorithm, ChainKeyBatchHeaderV1, ChainKeyBatchWitnessStepV1,
        ChainKeyBatchWitnessV1, ChainKeyDelegationCertV1, ChainKeyKeyId, ChainKeyRootSignatureV1,
        DelegatedAuthRegistrySnapshotV1, DelegatedRoleGrant, DelegatedTokenClaims,
        DelegationAudience, DelegationCert, DelegationProof, IcChainKeyBatchSignatureProofV1,
        IssuerProof, IssuerProofAlgorithm, IssuerProofBinding, RootKeyPolicyV1, RootProof,
        RootProofMode,
    },
    ids::{BuildNetwork, CanisterRole},
};
use sha2::{Digest, Sha256};
use thiserror::Error;

const DOMAIN_SEPARATOR: &[u8] = b"CANIC-AUTH\0";
const ISSUER_PROOF_BINDING_HASH_DOMAIN: &[u8] = b"canic-issuer-proof-binding-v1";
const CHAIN_KEY_BATCH_HEADER_DOMAIN: &[u8] = b"CANIC_ROOT_DELEGATION_CHAIN_KEY_BATCH_V1";
const CHAIN_KEY_DELEGATION_CERT_DOMAIN: &[u8] = b"CANIC_ROOT_DELEGATION_CHAIN_KEY_ISSUER_LEAF_V1";
const ROOT_KEY_POLICY_DOMAIN: &[u8] = b"CANIC_ROOT_KEY_POLICY_V1";
const DELEGATED_AUTH_REGISTRY_DOMAIN: &[u8] = b"CANIC_DELEGATED_AUTH_REGISTRY_SNAPSHOT_V1";
pub const MAX_TOKEN_EXT_BYTES: usize = 4096;

///
/// CanonicalDomain
///
/// Domain byte assigned to one delegated auth canonical payload family.
///

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalDomain {
    DelegationCert = 1,
    DelegatedTokenClaims = 2,
    DelegationProof = 3,
    RoleHash = 4,
    IssuerProof = 6,
}

///
/// CanonicalAuthError
///
/// Typed failure surface for delegated auth canonicalization.
///

#[derive(Debug, Eq, Error, PartialEq)]
pub enum CanonicalAuthError {
    #[error("delegated auth role is empty")]
    EmptyRole,
    #[error("delegated auth role contains invalid characters: {role}")]
    InvalidRole { role: String },
    #[error("delegated auth scope is empty")]
    EmptyScope,
    #[error("delegated auth scope contains invalid characters: {scope}")]
    InvalidScope { scope: String },
    #[error("delegated auth scopes must be strictly sorted and unique")]
    NonCanonicalScopes,
    #[error("delegated auth role grants must be strictly sorted and unique")]
    NonCanonicalRoles,
    #[error("delegated auth project audience is empty")]
    EmptyProject,
    #[error("delegated auth project audience contains invalid characters: {project}")]
    InvalidProject { project: String },
    #[error("delegated auth audiences must be strictly sorted and unique")]
    NonCanonicalAudiences,
    #[error("delegated auth issuer policies must be strictly sorted and unique")]
    NonCanonicalIssuerPolicies,
    #[error("delegated auth token ext is {len} bytes and exceeds max {max} bytes")]
    TokenExtTooLarge { len: usize, max: usize },
}

pub fn cert_hash(cert: &DelegationCert) -> Result<[u8; 32], CanonicalAuthError> {
    Ok(hash_bytes(&cert_bytes(cert)?))
}

pub fn claims_hash(claims: &DelegatedTokenClaims) -> Result<[u8; 32], CanonicalAuthError> {
    Ok(hash_bytes(&claims_bytes(claims)?))
}

pub fn proof_hash(proof: &DelegationProof) -> Result<[u8; 32], CanonicalAuthError> {
    Ok(hash_bytes(&proof_bytes(proof)?))
}

pub fn issuer_proof_hash(proof: &IssuerProof) -> [u8; 32] {
    hash_bytes(&issuer_proof_bytes(proof))
}

pub fn chain_key_batch_header_hash(header: &ChainKeyBatchHeaderV1) -> [u8; 32] {
    hash_chain_key_header_payload(&chain_key_batch_header_bytes(header))
}

pub fn chain_key_delegation_cert_hash(
    cert: &ChainKeyDelegationCertV1,
) -> Result<[u8; 32], CanonicalAuthError> {
    Ok(hash_chain_key_leaf_payload(
        &chain_key_delegation_cert_bytes(cert)?,
    ))
}

pub fn chain_key_derivation_path_hash(derivation_path: &[Vec<u8>]) -> [u8; 32] {
    crate::domain::auth::chain_key_derivation_path_hash(derivation_path)
}

pub fn root_key_policy_hash(policy: &RootKeyPolicyV1) -> [u8; 32] {
    let payload = root_key_policy_bytes(policy);
    let mut out = Vec::with_capacity(ROOT_KEY_POLICY_DOMAIN.len() + 4 + payload.len());
    out.extend_from_slice(ROOT_KEY_POLICY_DOMAIN);
    encode_bytes(&mut out, &payload);
    hash_bytes(&out)
}

pub fn delegated_auth_registry_hash(
    snapshot: &DelegatedAuthRegistrySnapshotV1,
) -> Result<[u8; 32], CanonicalAuthError> {
    let payload = delegated_auth_registry_snapshot_bytes(snapshot)?;
    let mut out = Vec::with_capacity(DELEGATED_AUTH_REGISTRY_DOMAIN.len() + 4 + payload.len());
    out.extend_from_slice(DELEGATED_AUTH_REGISTRY_DOMAIN);
    encode_bytes(&mut out, &payload);
    Ok(hash_bytes(&out))
}

pub fn issuer_proof_binding_hash(
    issuer_pid: Principal,
    issuer_proof_alg: IssuerProofAlgorithm,
    issuer_proof_binding: IssuerProofBinding,
) -> [u8; 32] {
    let mut out = Vec::with_capacity(128);
    out.extend_from_slice(ISSUER_PROOF_BINDING_HASH_DOMAIN);
    encode_principal(&mut out, issuer_pid);
    encode_issuer_proof_algorithm(&mut out, issuer_proof_alg);
    encode_issuer_proof_binding(&mut out, issuer_proof_binding);
    hash_bytes(&out)
}

pub fn role_hash(role: &CanisterRole) -> Result<[u8; 32], CanonicalAuthError> {
    validate_role(role)?;

    let mut out = domain_bytes(CanonicalDomain::RoleHash);
    encode_string(&mut out, role.as_str());
    Ok(hash_bytes(&out))
}

pub fn cert_bytes(cert: &DelegationCert) -> Result<Vec<u8>, CanonicalAuthError> {
    let mut out = domain_bytes(CanonicalDomain::DelegationCert);

    encode_principal(&mut out, cert.root_pid);
    encode_principal(&mut out, cert.issuer_pid);
    encode_issuer_proof_algorithm(&mut out, cert.issuer_proof_alg);
    encode_fixed_32(&mut out, cert.issuer_proof_binding_hash);
    encode_issuer_proof_binding(&mut out, cert.issuer_proof_binding);
    encode_u64(&mut out, cert.issued_at_ns);
    encode_u64(&mut out, cert.not_before_ns);
    encode_u64(&mut out, cert.expires_at_ns);
    encode_u64(&mut out, cert.max_token_ttl_ns);
    encode_audience(&mut out, &cert.aud)?;
    encode_role_grants(&mut out, &cert.grants)?;

    Ok(out)
}

pub fn claims_bytes(claims: &DelegatedTokenClaims) -> Result<Vec<u8>, CanonicalAuthError> {
    let mut out = domain_bytes(CanonicalDomain::DelegatedTokenClaims);

    encode_principal(&mut out, claims.subject);
    encode_principal(&mut out, claims.issuer_pid);
    encode_fixed_32(&mut out, claims.cert_hash);
    encode_u64(&mut out, claims.issued_at_ns);
    encode_u64(&mut out, claims.expires_at_ns);
    encode_audience(&mut out, &claims.aud)?;
    encode_role_grants(&mut out, &claims.grants)?;
    out.extend_from_slice(&claims.nonce);
    encode_token_ext(&mut out, claims.ext.as_deref())?;

    Ok(out)
}

pub fn proof_bytes(proof: &DelegationProof) -> Result<Vec<u8>, CanonicalAuthError> {
    let mut out = domain_bytes(CanonicalDomain::DelegationProof);

    out.extend_from_slice(&cert_bytes(&proof.cert)?);
    encode_root_proof(&mut out, &proof.root_proof)?;

    Ok(out)
}

pub fn issuer_proof_bytes(proof: &IssuerProof) -> Vec<u8> {
    let mut out = domain_bytes(CanonicalDomain::IssuerProof);
    encode_issuer_proof(&mut out, proof);
    out
}

fn domain_bytes(domain: CanonicalDomain) -> Vec<u8> {
    let mut out = Vec::with_capacity(128);
    out.extend_from_slice(DOMAIN_SEPARATOR);
    out.push(domain as u8);
    out
}

fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn encode_issuer_proof_algorithm(out: &mut Vec<u8>, alg: IssuerProofAlgorithm) {
    let tag = match alg {
        IssuerProofAlgorithm::IcCanisterSignatureV1 => 1,
    };
    out.push(tag);
}

fn encode_audience(
    out: &mut Vec<u8>,
    audience: &DelegationAudience,
) -> Result<(), CanonicalAuthError> {
    match audience {
        DelegationAudience::Canister(canister) => {
            out.push(1);
            encode_principal(out, *canister);
        }
        DelegationAudience::CanicSubnet(subnet) => {
            out.push(2);
            encode_principal(out, *subnet);
        }
        DelegationAudience::Project(project) => {
            out.push(3);
            encode_project(out, project)?;
        }
    }

    Ok(())
}

fn encode_role_grants(
    out: &mut Vec<u8>,
    grants: &[DelegatedRoleGrant],
) -> Result<(), CanonicalAuthError> {
    encode_len(out, grants.len());
    let mut previous = None;
    for grant in grants {
        let current = grant.target.as_str().as_bytes();
        if previous.is_some_and(|previous| previous >= current) {
            return Err(CanonicalAuthError::NonCanonicalRoles);
        }
        previous = Some(current);
        encode_role(out, &grant.target)?;
        encode_scopes(out, &grant.scopes)?;
    }
    Ok(())
}

fn hash_chain_key_header_payload(payload: &[u8]) -> [u8; 32] {
    let mut out = Vec::with_capacity(CHAIN_KEY_BATCH_HEADER_DOMAIN.len() + 4 + payload.len());
    out.extend_from_slice(CHAIN_KEY_BATCH_HEADER_DOMAIN);
    encode_bytes(&mut out, payload);
    hash_bytes(&out)
}

fn hash_chain_key_leaf_payload(payload: &[u8]) -> [u8; 32] {
    let mut out =
        Vec::with_capacity(1 + CHAIN_KEY_DELEGATION_CERT_DOMAIN.len() + 4 + payload.len());
    out.push(0);
    out.extend_from_slice(CHAIN_KEY_DELEGATION_CERT_DOMAIN);
    encode_bytes(&mut out, payload);
    hash_bytes(&out)
}

fn chain_key_batch_header_bytes(header: &ChainKeyBatchHeaderV1) -> Vec<u8> {
    let mut out = Vec::with_capacity(256);
    encode_u16(&mut out, header.schema_version);
    encode_principal(&mut out, header.root_canister_id);
    encode_fixed_32(&mut out, header.batch_id);
    encode_u64(&mut out, header.proof_epoch);
    encode_u64(&mut out, header.registry_epoch);
    encode_fixed_32(&mut out, header.registry_hash);
    encode_fixed_32(&mut out, header.tree_root);
    encode_u64(&mut out, header.not_before_ns);
    encode_u64(&mut out, header.expires_at_ns);
    encode_chain_key_algorithm(&mut out, header.algorithm);
    encode_chain_key_key_id(&mut out, &header.key_id);
    encode_fixed_32(&mut out, header.derivation_path_hash);
    encode_u64(&mut out, header.key_version);
    out
}

fn chain_key_delegation_cert_bytes(
    cert: &ChainKeyDelegationCertV1,
) -> Result<Vec<u8>, CanonicalAuthError> {
    let mut out = Vec::with_capacity(256);
    encode_principal(&mut out, cert.root_canister_id);
    encode_principal(&mut out, cert.issuer_canister_id);
    encode_u64(&mut out, cert.proof_epoch);
    encode_issuer_proof_algorithm(&mut out, cert.issuer_proof_algorithm);
    encode_fixed_32(&mut out, cert.issuer_proof_binding_hash);
    encode_issuer_proof_binding(&mut out, cert.issuer_proof_binding);
    encode_u64(&mut out, cert.max_token_ttl_ns);
    encode_audience(&mut out, &cert.audience)?;
    encode_role_grants(&mut out, &cert.grants)?;
    encode_u64(&mut out, cert.not_before_ns);
    encode_u64(&mut out, cert.expires_at_ns);
    encode_u64(&mut out, cert.registry_epoch);
    encode_fixed_32(&mut out, cert.registry_hash);
    Ok(out)
}

fn encode_root_proof(out: &mut Vec<u8>, proof: &RootProof) -> Result<(), CanonicalAuthError> {
    match proof {
        RootProof::IcChainKeyBatchSignatureV1(proof) => {
            out.push(2);
            encode_chain_key_proof(out, proof)?;
        }
    }
    Ok(())
}

fn encode_chain_key_proof(
    out: &mut Vec<u8>,
    proof: &IcChainKeyBatchSignatureProofV1,
) -> Result<(), CanonicalAuthError> {
    out.extend_from_slice(&chain_key_batch_header_bytes(&proof.header));
    out.extend_from_slice(&chain_key_delegation_cert_bytes(&proof.delegation_cert)?);
    encode_chain_key_witness(out, &proof.issuer_witness);
    encode_chain_key_signature(out, &proof.signature);
    Ok(())
}

fn encode_chain_key_witness(out: &mut Vec<u8>, witness: &ChainKeyBatchWitnessV1) {
    encode_len(out, witness.steps.len());
    for step in &witness.steps {
        match step {
            ChainKeyBatchWitnessStepV1::LeftSibling(hash) => {
                out.push(1);
                encode_fixed_32(out, *hash);
            }
            ChainKeyBatchWitnessStepV1::RightSibling(hash) => {
                out.push(2);
                encode_fixed_32(out, *hash);
            }
        }
    }
}

fn encode_chain_key_signature(out: &mut Vec<u8>, signature: &ChainKeyRootSignatureV1) {
    encode_chain_key_algorithm(out, signature.algorithm);
    encode_chain_key_key_id(out, &signature.key_id);
    encode_chain_key_derivation_path(out, &signature.derivation_path);
    encode_bytes(out, &signature.public_key);
    encode_bytes(out, &signature.signature);
}

fn encode_chain_key_derivation_path(out: &mut Vec<u8>, derivation_path: &[Vec<u8>]) {
    encode_len(out, derivation_path.len());
    for path_component in derivation_path {
        encode_bytes(out, path_component);
    }
}

fn encode_chain_key_algorithm(out: &mut Vec<u8>, algorithm: ChainKeyAlgorithm) {
    let tag = match algorithm {
        ChainKeyAlgorithm::EcdsaSecp256k1 => 1,
    };
    out.push(tag);
}

fn encode_chain_key_key_id(out: &mut Vec<u8>, key_id: &ChainKeyKeyId) {
    encode_string(out, &key_id.name);
}

fn root_key_policy_bytes(policy: &RootKeyPolicyV1) -> Vec<u8> {
    let mut out = Vec::with_capacity(256);
    encode_principal(&mut out, policy.root_canister_id);
    encode_root_proof_mode(&mut out, policy.proof_mode);
    encode_chain_key_algorithm(&mut out, policy.algorithm);
    encode_chain_key_key_id(&mut out, &policy.key_id);
    encode_fixed_32(&mut out, policy.derivation_path_hash);
    encode_bytes(&mut out, &policy.public_key);
    encode_u64(&mut out, policy.key_version);
    encode_u64(&mut out, policy.min_accepted_key_version);
    encode_u64(&mut out, policy.min_accepted_proof_epoch);
    encode_u64(&mut out, policy.min_accepted_registry_epoch);
    encode_u64(&mut out, policy.max_revocation_latency_ns);
    encode_u64(&mut out, policy.valid_from_ns);
    encode_u64(&mut out, policy.accept_until_ns);
    encode_build_network(&mut out, policy.build_network);
    out
}

fn delegated_auth_registry_snapshot_bytes(
    snapshot: &DelegatedAuthRegistrySnapshotV1,
) -> Result<Vec<u8>, CanonicalAuthError> {
    let mut out = Vec::with_capacity(512);
    encode_u16(&mut out, snapshot.schema_version);
    encode_principal(&mut out, snapshot.root_canister_id);
    encode_u64(&mut out, snapshot.registry_epoch);
    encode_root_proof_mode(&mut out, snapshot.proof_mode);
    encode_fixed_32(&mut out, snapshot.root_key_policy_hash);
    encode_registry_issuer_policies(&mut out, &snapshot.issuer_policies)?;
    Ok(out)
}

fn encode_registry_issuer_policies(
    out: &mut Vec<u8>,
    issuer_policies: &[crate::dto::auth::DelegatedAuthIssuerPolicySnapshotV1],
) -> Result<(), CanonicalAuthError> {
    encode_len(out, issuer_policies.len());
    let mut previous = None;
    for policy in issuer_policies {
        let current = policy.issuer_canister_id.as_slice();
        if previous.is_some_and(|previous| previous >= current) {
            return Err(CanonicalAuthError::NonCanonicalIssuerPolicies);
        }
        previous = Some(current);

        encode_principal(out, policy.issuer_canister_id);
        encode_bool(out, policy.enabled);
        encode_root_proof_mode(out, policy.preferred_proof_mode);
        encode_audiences(out, &policy.allowed_audiences)?;
        encode_role_grants(out, &policy.allowed_grants)?;
        encode_u64(out, policy.max_root_proof_ttl_ns);
        encode_u64(out, policy.max_token_ttl_ns);
        encode_issuer_proof_algorithm(out, policy.issuer_proof_algorithm);
        encode_fixed_32(out, policy.issuer_proof_binding_hash);
        encode_fixed_32(out, policy.renewal_template_hash);
    }
    Ok(())
}

fn encode_audiences(
    out: &mut Vec<u8>,
    audiences: &[DelegationAudience],
) -> Result<(), CanonicalAuthError> {
    encode_len(out, audiences.len());
    let mut previous = None;
    for audience in audiences {
        let current = audience_bytes(audience)?;
        if previous
            .as_ref()
            .is_some_and(|previous: &Vec<u8>| previous.as_slice() >= current.as_slice())
        {
            return Err(CanonicalAuthError::NonCanonicalAudiences);
        }
        out.extend_from_slice(&current);
        previous = Some(current);
    }
    Ok(())
}

fn audience_bytes(audience: &DelegationAudience) -> Result<Vec<u8>, CanonicalAuthError> {
    let mut out = Vec::with_capacity(64);
    encode_audience(&mut out, audience)?;
    Ok(out)
}

fn encode_root_proof_mode(out: &mut Vec<u8>, _mode: RootProofMode) {
    out.push(2);
}

fn encode_build_network(out: &mut Vec<u8>, network: BuildNetwork) {
    let tag = match network {
        BuildNetwork::Ic => 1,
        BuildNetwork::Local => 2,
    };
    out.push(tag);
}

fn encode_bool(out: &mut Vec<u8>, value: bool) {
    out.push(u8::from(value));
}

fn encode_issuer_proof(out: &mut Vec<u8>, proof: &IssuerProof) {
    match proof {
        IssuerProof::IcCanisterSignatureV1(proof) => {
            out.push(1);
            encode_bytes(out, &proof.signature_cbor);
            encode_bytes(out, &proof.public_key_der);
        }
    }
}

fn encode_issuer_proof_binding(out: &mut Vec<u8>, binding: IssuerProofBinding) {
    match binding {
        IssuerProofBinding::IcCanisterSignatureV1 { seed_hash } => {
            out.push(1);
            encode_fixed_32(out, seed_hash);
        }
    }
}

fn encode_token_ext(out: &mut Vec<u8>, ext: Option<&[u8]>) -> Result<(), CanonicalAuthError> {
    match ext {
        Some(ext) => {
            if ext.len() > MAX_TOKEN_EXT_BYTES {
                return Err(CanonicalAuthError::TokenExtTooLarge {
                    len: ext.len(),
                    max: MAX_TOKEN_EXT_BYTES,
                });
            }
            out.push(1);
            encode_bytes(out, ext);
        }
        None => out.push(0),
    }
    Ok(())
}

fn encode_role(out: &mut Vec<u8>, role: &CanisterRole) -> Result<(), CanonicalAuthError> {
    validate_role(role)?;
    encode_bytes(out, role.as_str().as_bytes());
    Ok(())
}

fn encode_scopes(out: &mut Vec<u8>, scopes: &[String]) -> Result<(), CanonicalAuthError> {
    let mut previous = None;
    for scope in scopes {
        validate_scope_label(scope)?;
        let current = scope.as_bytes();
        if previous.is_some_and(|previous| previous >= current) {
            return Err(CanonicalAuthError::NonCanonicalScopes);
        }
        previous = Some(current);
    }

    encode_len(out, scopes.len());
    for scope in scopes {
        encode_bytes(out, scope.as_bytes());
    }

    Ok(())
}

fn validate_role(role: &CanisterRole) -> Result<(), CanonicalAuthError> {
    let role = role.as_str();
    if role.is_empty() {
        return Err(CanonicalAuthError::EmptyRole);
    }
    if !role.bytes().all(is_canonical_label_byte) {
        return Err(CanonicalAuthError::InvalidRole {
            role: role.to_string(),
        });
    }
    Ok(())
}

pub fn validate_scope_label(scope: &str) -> Result<(), CanonicalAuthError> {
    if scope.is_empty() {
        return Err(CanonicalAuthError::EmptyScope);
    }
    if !scope.bytes().all(is_canonical_label_byte) {
        return Err(CanonicalAuthError::InvalidScope {
            scope: scope.to_string(),
        });
    }
    Ok(())
}

fn validate_project(project: &str) -> Result<(), CanonicalAuthError> {
    if project.is_empty() {
        return Err(CanonicalAuthError::EmptyProject);
    }
    if !project.bytes().all(is_canonical_project_byte) {
        return Err(CanonicalAuthError::InvalidProject {
            project: project.to_string(),
        });
    }
    Ok(())
}

fn encode_project(out: &mut Vec<u8>, project: &str) -> Result<(), CanonicalAuthError> {
    validate_project(project)?;
    encode_bytes(out, project.as_bytes());
    Ok(())
}

const fn is_canonical_label_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b':' | b'-')
}

const fn is_canonical_project_byte(byte: u8) -> bool {
    is_canonical_label_byte(byte) || byte == b'.'
}

fn encode_string(out: &mut Vec<u8>, value: &str) {
    encode_bytes(out, value.as_bytes());
}

fn encode_principal(out: &mut Vec<u8>, principal: Principal) {
    encode_bytes(out, principal.as_slice());
}

fn encode_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    encode_len(out, bytes.len());
    out.extend_from_slice(bytes);
}

fn encode_fixed_32(out: &mut Vec<u8>, bytes: [u8; 32]) {
    out.extend_from_slice(&bytes);
}

fn encode_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn encode_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn encode_len(out: &mut Vec<u8>, len: usize) {
    let len = u32::try_from(len).expect("delegated auth canonical vector length exceeds u32");
    out.extend_from_slice(&len.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::auth::IcCanisterSignatureProofV1;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn sample_cert() -> DelegationCert {
        let issuer_proof_alg = IssuerProofAlgorithm::IcCanisterSignatureV1;
        let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 { seed_hash: [8; 32] };
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
            expires_at_ns: 200,
            max_token_ttl_ns: 60,
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![grant("project_instance", &["read", "write"])],
        }
    }

    fn grant(role: &str, scopes: &[&str]) -> DelegatedRoleGrant {
        DelegatedRoleGrant {
            target: CanisterRole::owned(role.to_string()),
            scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
        }
    }

    fn chain_key_proof() -> IcChainKeyBatchSignatureProofV1 {
        let key_id = ChainKeyKeyId {
            name: "test_key_1".to_string(),
        };

        IcChainKeyBatchSignatureProofV1 {
            header: ChainKeyBatchHeaderV1 {
                schema_version: 1,
                root_canister_id: p(1),
                batch_id: [31; 32],
                proof_epoch: 2,
                registry_epoch: 3,
                registry_hash: [32; 32],
                tree_root: [33; 32],
                not_before_ns: 100,
                expires_at_ns: 200,
                algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
                key_id: key_id.clone(),
                derivation_path_hash: [34; 32],
                key_version: 4,
            },
            delegation_cert: ChainKeyDelegationCertV1 {
                root_canister_id: p(1),
                issuer_canister_id: p(3),
                proof_epoch: 2,
                issuer_proof_algorithm: IssuerProofAlgorithm::IcCanisterSignatureV1,
                issuer_proof_binding_hash: [35; 32],
                issuer_proof_binding: IssuerProofBinding::IcCanisterSignatureV1 {
                    seed_hash: [36; 32],
                },
                max_token_ttl_ns: 60,
                audience: DelegationAudience::Project("test".to_string()),
                grants: vec![grant("project_instance", &["read", "write"])],
                not_before_ns: 100,
                expires_at_ns: 200,
                registry_epoch: 3,
                registry_hash: [32; 32],
            },
            issuer_witness: ChainKeyBatchWitnessV1 {
                steps: vec![
                    ChainKeyBatchWitnessStepV1::LeftSibling([37; 32]),
                    ChainKeyBatchWitnessStepV1::RightSibling([38; 32]),
                ],
            },
            signature: ChainKeyRootSignatureV1 {
                algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
                key_id,
                derivation_path: vec![b"canic".to_vec(), b"delegation".to_vec()],
                public_key: vec![39; 33],
                signature: vec![40; 64],
            },
        }
    }

    fn root_key_policy() -> RootKeyPolicyV1 {
        RootKeyPolicyV1 {
            root_canister_id: p(1),
            proof_mode: RootProofMode::ChainKeyBatch,
            algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
            key_id: ChainKeyKeyId {
                name: "test_key_1".to_string(),
            },
            derivation_path_hash: chain_key_derivation_path_hash(&[
                b"canic".to_vec(),
                b"delegation".to_vec(),
            ]),
            public_key: vec![38; 33],
            key_version: 4,
            min_accepted_key_version: 4,
            min_accepted_proof_epoch: 2,
            min_accepted_registry_epoch: 3,
            max_revocation_latency_ns: 600,
            valid_from_ns: 100,
            accept_until_ns: 1_000,
            build_network: BuildNetwork::Local,
        }
    }

    fn registry_issuer_policy(
        issuer_canister_id: Principal,
    ) -> crate::dto::auth::DelegatedAuthIssuerPolicySnapshotV1 {
        let issuer_proof_alg = IssuerProofAlgorithm::IcCanisterSignatureV1;
        let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 { seed_hash: [8; 32] };

        crate::dto::auth::DelegatedAuthIssuerPolicySnapshotV1 {
            issuer_canister_id,
            enabled: true,
            preferred_proof_mode: RootProofMode::ChainKeyBatch,
            allowed_audiences: vec![DelegationAudience::Project("test".to_string())],
            allowed_grants: vec![grant("project_instance", &["read", "write"])],
            max_root_proof_ttl_ns: 600,
            max_token_ttl_ns: 60,
            issuer_proof_algorithm: issuer_proof_alg,
            issuer_proof_binding_hash: issuer_proof_binding_hash(
                issuer_canister_id,
                issuer_proof_alg,
                issuer_proof_binding,
            ),
            renewal_template_hash: [41; 32],
        }
    }

    fn registry_snapshot() -> DelegatedAuthRegistrySnapshotV1 {
        DelegatedAuthRegistrySnapshotV1 {
            schema_version: 1,
            root_canister_id: p(1),
            registry_epoch: 3,
            proof_mode: RootProofMode::ChainKeyBatch,
            root_key_policy_hash: root_key_policy_hash(&root_key_policy()),
            issuer_policies: vec![registry_issuer_policy(p(3)), registry_issuer_policy(p(4))],
        }
    }

    #[test]
    fn cert_hash_rejects_noncanonical_scope_order() {
        let mut cert = sample_cert();
        cert.grants = vec![grant("project_instance", &["write", "read"])];

        assert_eq!(
            cert_hash(&cert),
            Err(CanonicalAuthError::NonCanonicalScopes)
        );
    }

    #[test]
    fn cert_hash_rejects_noncanonical_roles() {
        let mut cert = sample_cert();
        cert.grants = vec![DelegatedRoleGrant {
            target: CanisterRole::owned("ProjectInstance".to_string()),
            scopes: vec!["read".to_string()],
        }];

        assert_eq!(
            cert_hash(&cert),
            Err(CanonicalAuthError::InvalidRole {
                role: "ProjectInstance".to_string(),
            })
        );
    }

    #[test]
    fn claims_hash_rejects_noncanonical_scopes() {
        let claims = DelegatedTokenClaims {
            subject: p(10),
            issuer_pid: p(11),
            cert_hash: [12; 32],
            issued_at_ns: 100,
            expires_at_ns: 120,
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![DelegatedRoleGrant {
                target: CanisterRole::new("project_instance"),
                scopes: vec!["Read".to_string()],
            }],
            nonce: [14; 16],
            ext: None,
        };

        assert_eq!(
            claims_hash(&claims),
            Err(CanonicalAuthError::InvalidScope {
                scope: "Read".to_string(),
            })
        );
    }

    #[test]
    fn claims_hash_rejects_noncanonical_scope_order() {
        let left = DelegatedTokenClaims {
            subject: p(10),
            issuer_pid: p(11),
            cert_hash: [12; 32],
            issued_at_ns: 100,
            expires_at_ns: 120,
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![grant("project_instance", &["write", "read"])],
            nonce: [14; 16],
            ext: None,
        };

        assert_eq!(
            claims_hash(&left),
            Err(CanonicalAuthError::NonCanonicalScopes)
        );
    }

    #[test]
    fn role_hash_is_domain_separated_from_certificate_hash() {
        let role = CanisterRole::new("project_instance");
        let cert = sample_cert();

        assert_ne!(role_hash(&role).unwrap(), cert_hash(&cert).unwrap());
    }

    #[test]
    fn chain_key_header_and_delegation_cert_hashes_bind_core_fields() {
        let proof = chain_key_proof();
        let header_hash = chain_key_batch_header_hash(&proof.header);
        let cert_hash = chain_key_delegation_cert_hash(&proof.delegation_cert).unwrap();
        let mut changed_header = proof.header.clone();
        changed_header.key_version += 1;
        let mut changed_cert = proof.delegation_cert;
        changed_cert.max_token_ttl_ns += 1;

        assert_ne!(header_hash, chain_key_batch_header_hash(&changed_header));
        assert_ne!(
            cert_hash,
            chain_key_delegation_cert_hash(&changed_cert).unwrap()
        );
    }

    #[test]
    fn chain_key_canonical_hashes_match_golden_fixtures() {
        let proof = chain_key_proof();

        assert_eq!(
            chain_key_batch_header_hash(&proof.header),
            [
                231, 134, 199, 186, 130, 244, 250, 243, 254, 252, 150, 140, 3, 154, 230, 252, 45,
                52, 89, 215, 119, 228, 233, 231, 245, 96, 54, 45, 33, 18, 44, 192,
            ]
        );
        assert_eq!(
            chain_key_delegation_cert_hash(&proof.delegation_cert).unwrap(),
            [
                244, 24, 85, 249, 39, 254, 112, 50, 126, 247, 218, 189, 252, 25, 113, 117, 21, 152,
                4, 105, 235, 7, 3, 3, 67, 37, 164, 14, 150, 2, 48, 80,
            ]
        );
        assert_eq!(
            root_key_policy_hash(&root_key_policy()),
            [
                245, 123, 186, 75, 47, 9, 17, 164, 83, 153, 204, 101, 211, 12, 234, 140, 44, 179,
                104, 246, 15, 193, 33, 167, 24, 245, 177, 235, 49, 20, 183, 105,
            ]
        );
        assert_eq!(
            delegated_auth_registry_hash(&registry_snapshot()).unwrap(),
            [
                29, 228, 231, 71, 61, 149, 51, 92, 136, 55, 56, 134, 127, 136, 134, 175, 166, 149,
                242, 239, 235, 219, 5, 69, 113, 47, 55, 251, 255, 83, 171, 2,
            ]
        );
        assert_eq!(
            proof_hash(&DelegationProof {
                cert: sample_cert(),
                root_proof: RootProof::IcChainKeyBatchSignatureV1(proof),
            })
            .unwrap(),
            [
                76, 118, 34, 138, 14, 247, 151, 185, 110, 139, 52, 156, 178, 233, 45, 67, 147, 228,
                240, 50, 93, 113, 3, 31, 183, 207, 161, 217, 74, 90, 254, 172,
            ]
        );
    }

    #[test]
    fn root_key_policy_hash_binds_key_policy_fields() {
        let policy = root_key_policy();
        let hash = root_key_policy_hash(&policy);
        let mut changed_key = policy.clone();
        changed_key.key_version += 1;
        let mut changed_network = policy;
        changed_network.build_network = BuildNetwork::Ic;

        assert_ne!(hash, root_key_policy_hash(&changed_key));
        assert_ne!(hash, root_key_policy_hash(&changed_network));
    }

    #[test]
    fn delegated_auth_registry_hash_binds_snapshot_fields() {
        let snapshot = registry_snapshot();
        let hash = delegated_auth_registry_hash(&snapshot).unwrap();
        let mut changed_epoch = snapshot.clone();
        changed_epoch.registry_epoch += 1;
        let mut changed_issuer = snapshot;
        changed_issuer.issuer_policies[0].max_token_ttl_ns += 1;

        assert_ne!(hash, delegated_auth_registry_hash(&changed_epoch).unwrap());
        assert_ne!(hash, delegated_auth_registry_hash(&changed_issuer).unwrap());
    }

    #[test]
    fn delegated_auth_registry_hash_rejects_noncanonical_issuer_order() {
        let mut snapshot = registry_snapshot();
        snapshot.issuer_policies.reverse();

        assert_eq!(
            delegated_auth_registry_hash(&snapshot),
            Err(CanonicalAuthError::NonCanonicalIssuerPolicies)
        );
    }

    #[test]
    fn delegated_auth_registry_hash_rejects_noncanonical_audience_order() {
        let mut snapshot = registry_snapshot();
        snapshot.issuer_policies[0].allowed_audiences = vec![
            DelegationAudience::Project("z".to_string()),
            DelegationAudience::Project("a".to_string()),
        ];

        assert_eq!(
            delegated_auth_registry_hash(&snapshot),
            Err(CanonicalAuthError::NonCanonicalAudiences)
        );
    }

    #[test]
    fn chain_key_root_proof_hash_binds_witness_direction_and_public_key() {
        let proof = chain_key_proof();
        let delegation_proof = DelegationProof {
            cert: sample_cert(),
            root_proof: RootProof::IcChainKeyBatchSignatureV1(proof.clone()),
        };
        let base_hash = proof_hash(&delegation_proof).unwrap();
        let mut changed_witness = proof.clone();
        changed_witness.issuer_witness.steps[0] =
            ChainKeyBatchWitnessStepV1::RightSibling([36; 32]);
        let mut changed_public_key = proof;
        changed_public_key.signature.public_key[0] ^= 1;

        assert_ne!(
            base_hash,
            proof_hash(&DelegationProof {
                cert: sample_cert(),
                root_proof: RootProof::IcChainKeyBatchSignatureV1(changed_witness),
            })
            .unwrap()
        );
        assert_ne!(
            base_hash,
            proof_hash(&DelegationProof {
                cert: sample_cert(),
                root_proof: RootProof::IcChainKeyBatchSignatureV1(changed_public_key),
            })
            .unwrap()
        );
    }

    #[test]
    fn claims_hash_binds_ext_bytes() {
        let mut left = DelegatedTokenClaims {
            subject: p(10),
            issuer_pid: p(11),
            cert_hash: [12; 32],
            issued_at_ns: 100,
            expires_at_ns: 120,
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![grant("project_instance", &["read"])],
            nonce: [14; 16],
            ext: Some(b"user=1".to_vec()),
        };
        let mut right = left.clone();
        right.ext = Some(b"user=2".to_vec());

        assert_ne!(claims_hash(&left).unwrap(), claims_hash(&right).unwrap());
        left.ext = None;
        assert_ne!(claims_hash(&left).unwrap(), claims_hash(&right).unwrap());
    }

    #[test]
    fn claims_hash_rejects_oversized_ext() {
        let claims = DelegatedTokenClaims {
            subject: p(10),
            issuer_pid: p(11),
            cert_hash: [12; 32],
            issued_at_ns: 100,
            expires_at_ns: 120,
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![grant("project_instance", &["read"])],
            nonce: [14; 16],
            ext: Some(vec![1; MAX_TOKEN_EXT_BYTES + 1]),
        };

        assert_eq!(
            claims_hash(&claims),
            Err(CanonicalAuthError::TokenExtTooLarge {
                len: MAX_TOKEN_EXT_BYTES + 1,
                max: MAX_TOKEN_EXT_BYTES,
            })
        );
    }

    #[test]
    fn issuer_proof_hash_binds_signature_and_public_key() {
        let proof = IssuerProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
            signature_cbor: vec![1, 2, 3],
            public_key_der: vec![4, 5, 6],
        });
        let mut changed_signature = proof.clone();
        let mut changed_public_key = proof.clone();
        let IssuerProof::IcCanisterSignatureV1(changed) = &mut changed_signature;
        changed.signature_cbor[0] ^= 1;
        let IssuerProof::IcCanisterSignatureV1(changed) = &mut changed_public_key;
        changed.public_key_der[0] ^= 1;

        assert_ne!(
            issuer_proof_hash(&proof),
            issuer_proof_hash(&changed_signature)
        );
        assert_ne!(
            issuer_proof_hash(&proof),
            issuer_proof_hash(&changed_public_key)
        );
    }

    #[test]
    fn issuer_proof_binding_hash_binds_authority_context() {
        let binding = IssuerProofBinding::IcCanisterSignatureV1 { seed_hash: [7; 32] };
        let base =
            issuer_proof_binding_hash(p(1), IssuerProofAlgorithm::IcCanisterSignatureV1, binding);

        assert_ne!(
            base,
            issuer_proof_binding_hash(p(2), IssuerProofAlgorithm::IcCanisterSignatureV1, binding)
        );
        assert_ne!(
            base,
            issuer_proof_binding_hash(
                p(1),
                IssuerProofAlgorithm::IcCanisterSignatureV1,
                IssuerProofBinding::IcCanisterSignatureV1 { seed_hash: [8; 32] },
            )
        );
    }
}
