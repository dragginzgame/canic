use crate::{
    cdk::types::Principal,
    dto::auth::{
        DelegatedRoleGrant, DelegatedTokenClaims, DelegationAudience, DelegationCert,
        ShardKeyBinding, SignatureAlgorithm,
    },
    ids::CanisterRole,
};
use sha2::{Digest, Sha256};
use thiserror::Error;

const DOMAIN_SEPARATOR: &[u8] = b"CANIC-AUTH\0";

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalDomain {
    DelegationCert = 1,
    DelegatedTokenClaims = 2,
    RoleHash = 4,
    DerivationPath = 5,
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
}

pub fn cert_hash(cert: &DelegationCert) -> Result<[u8; 32], CanonicalAuthError> {
    Ok(hash_bytes(&cert_bytes(cert)?))
}

pub fn claims_hash(claims: &DelegatedTokenClaims) -> Result<[u8; 32], CanonicalAuthError> {
    Ok(hash_bytes(&claims_bytes(claims)?))
}

pub fn public_key_hash(public_key_sec1: &[u8]) -> [u8; 32] {
    hash_bytes(public_key_sec1)
}

pub fn key_name_hash(key_name: &str) -> [u8; 32] {
    hash_bytes(key_name.as_bytes())
}

pub fn derivation_path_hash(path: &[Vec<u8>]) -> [u8; 32] {
    let mut out = domain_bytes(CanonicalDomain::DerivationPath);
    encode_len(&mut out, path.len());
    for segment in path {
        encode_bytes(&mut out, segment);
    }
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

    encode_u16(&mut out, cert.version);
    encode_principal(&mut out, cert.root_pid);
    encode_string(&mut out, &cert.root_key_id);
    encode_fixed_32(&mut out, cert.root_key_hash);
    encode_algorithm(&mut out, cert.alg);
    encode_principal(&mut out, cert.shard_pid);
    encode_string(&mut out, &cert.shard_key_id);
    encode_bytes(&mut out, &cert.shard_public_key_sec1);
    encode_fixed_32(&mut out, cert.shard_key_hash);
    encode_shard_key_binding(&mut out, cert.shard_key_binding);
    encode_u64(&mut out, cert.issued_at);
    encode_u64(&mut out, cert.expires_at);
    encode_u64(&mut out, cert.max_token_ttl_secs);
    encode_audience(&mut out, &cert.aud)?;
    encode_role_grants(&mut out, &cert.grants)?;

    Ok(out)
}

pub fn claims_bytes(claims: &DelegatedTokenClaims) -> Result<Vec<u8>, CanonicalAuthError> {
    let mut out = domain_bytes(CanonicalDomain::DelegatedTokenClaims);

    encode_u16(&mut out, claims.version);
    encode_principal(&mut out, claims.subject);
    encode_principal(&mut out, claims.issuer_shard_pid);
    encode_fixed_32(&mut out, claims.cert_hash);
    encode_u64(&mut out, claims.issued_at);
    encode_u64(&mut out, claims.expires_at);
    encode_audience(&mut out, &claims.aud)?;
    encode_role_grants(&mut out, &claims.grants)?;
    out.extend_from_slice(&claims.nonce);

    Ok(out)
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

fn encode_algorithm(out: &mut Vec<u8>, alg: SignatureAlgorithm) {
    let tag = match alg {
        SignatureAlgorithm::EcdsaP256Sha256 => 1,
    };
    out.push(tag);
}

fn encode_audience(
    out: &mut Vec<u8>,
    audience: &DelegationAudience,
) -> Result<(), CanonicalAuthError> {
    match audience {
        DelegationAudience::Canic => {
            out.push(1);
        }
        DelegationAudience::Project(project) => {
            out.push(2);
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

fn encode_shard_key_binding(out: &mut Vec<u8>, binding: ShardKeyBinding) {
    match binding {
        ShardKeyBinding::IcThresholdEcdsa {
            key_name_hash,
            derivation_path_hash,
        } => {
            out.push(1);
            encode_fixed_32(out, key_name_hash);
            encode_fixed_32(out, derivation_path_hash);
        }
    }
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

fn encode_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
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

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn sample_cert() -> DelegationCert {
        DelegationCert {
            version: 2,
            root_pid: p(1),
            root_key_id: "root-key".to_string(),
            root_key_hash: [2; 32],
            alg: SignatureAlgorithm::EcdsaP256Sha256,
            shard_pid: p(3),
            shard_key_id: "shard-key".to_string(),
            shard_public_key_sec1: vec![4, 5, 6],
            shard_key_hash: [7; 32],
            shard_key_binding: ShardKeyBinding::IcThresholdEcdsa {
                key_name_hash: [8; 32],
                derivation_path_hash: [9; 32],
            },
            issued_at: 100,
            expires_at: 200,
            max_token_ttl_secs: 60,
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
            version: 2,
            subject: p(10),
            issuer_shard_pid: p(11),
            cert_hash: [12; 32],
            issued_at: 100,
            expires_at: 120,
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![DelegatedRoleGrant {
                target: CanisterRole::new("project_instance"),
                scopes: vec!["Read".to_string()],
            }],
            nonce: [14; 16],
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
            version: 2,
            subject: p(10),
            issuer_shard_pid: p(11),
            cert_hash: [12; 32],
            issued_at: 100,
            expires_at: 120,
            aud: DelegationAudience::Project("test".to_string()),
            grants: vec![grant("project_instance", &["write", "read"])],
            nonce: [14; 16],
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
    fn derivation_path_hash_preserves_segment_boundaries() {
        let left = vec![b"ab".to_vec(), b"c".to_vec()];
        let right = vec![b"a".to_vec(), b"bc".to_vec()];

        assert_ne!(derivation_path_hash(&left), derivation_path_hash(&right));
        assert_eq!(key_name_hash("icp_test_key"), key_name_hash("icp_test_key"));
    }
}
