use crate::{
    cdk::types::Principal,
    dto::auth::{
        DelegatedRoleGrant, DelegatedTokenClaims, DelegationAudience, DelegationCert,
        DelegationProof, IssuerProof, IssuerProofAlgorithm, IssuerProofBinding, RootProof,
    },
    ids::CanisterRole,
};
use sha2::{Digest, Sha256};
use thiserror::Error;

const DOMAIN_SEPARATOR: &[u8] = b"CANIC-AUTH\0";
const ISSUER_PROOF_BINDING_HASH_DOMAIN: &[u8] = b"canic-issuer-proof-binding-v1";
pub const MAX_TOKEN_EXT_BYTES: usize = 4096;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalDomain {
    DelegationCert = 1,
    DelegatedTokenClaims = 2,
    DelegationProof = 3,
    RoleHash = 4,
    IssuerProof = 6,
}

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
    encode_root_proof(&mut out, &proof.root_proof);

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

fn encode_root_proof(out: &mut Vec<u8>, proof: &RootProof) {
    match proof {
        RootProof::IcCanisterSignatureV1(proof) => {
            out.push(1);
            encode_bytes(out, &proof.signature_cbor);
            encode_bytes(out, &proof.public_key_der);
        }
    }
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
