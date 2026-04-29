use crate::{
    cdk::types::Principal,
    dto::auth::{
        DelegatedTokenClaimsV2, DelegationAudienceV2, DelegationCertV2, RootKeyCertificateV2,
        ShardKeyBindingV2, SignatureAlgorithmV2,
    },
    ids::CanisterRole,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use thiserror::Error;

const DOMAIN_SEPARATOR: &[u8] = b"CANIC-AUTH-V2\0";

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalDomainV2 {
    DelegationCert = 1,
    DelegatedTokenClaims = 2,
    RootKeyCertificate = 3,
    RoleHash = 4,
    DerivationPath = 5,
}

#[derive(Debug, Eq, Error, PartialEq)]
pub enum CanonicalAuthV2Error {
    #[error("delegated auth v2 role is empty")]
    EmptyRole,
    #[error("delegated auth v2 role contains invalid characters: {role}")]
    InvalidRole { role: String },
    #[error("delegated auth v2 scope is empty")]
    EmptyScope,
    #[error("delegated auth v2 scope contains invalid characters: {scope}")]
    InvalidScope { scope: String },
}

pub fn cert_hash(cert: &DelegationCertV2) -> Result<[u8; 32], CanonicalAuthV2Error> {
    Ok(hash_bytes(&cert_bytes(cert)?))
}

pub fn claims_hash(claims: &DelegatedTokenClaimsV2) -> Result<[u8; 32], CanonicalAuthV2Error> {
    Ok(hash_bytes(&claims_bytes(claims)?))
}

pub fn root_key_certificate_payload_hash(cert: &RootKeyCertificateV2) -> [u8; 32] {
    hash_bytes(&root_key_certificate_payload_bytes(cert))
}

pub fn public_key_hash(public_key_sec1: &[u8]) -> [u8; 32] {
    hash_bytes(public_key_sec1)
}

pub fn key_name_hash(key_name: &str) -> [u8; 32] {
    hash_bytes(key_name.as_bytes())
}

pub fn derivation_path_hash(path: &[Vec<u8>]) -> [u8; 32] {
    let mut out = domain_bytes(CanonicalDomainV2::DerivationPath);
    encode_len(&mut out, path.len());
    for segment in path {
        encode_bytes(&mut out, segment);
    }
    hash_bytes(&out)
}

pub fn role_hash(role: &CanisterRole) -> Result<[u8; 32], CanonicalAuthV2Error> {
    validate_role(role)?;

    let mut out = domain_bytes(CanonicalDomainV2::RoleHash);
    encode_string(&mut out, role.as_str());
    Ok(hash_bytes(&out))
}

pub fn cert_bytes(cert: &DelegationCertV2) -> Result<Vec<u8>, CanonicalAuthV2Error> {
    let mut out = domain_bytes(CanonicalDomainV2::DelegationCert);

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
    encode_scopes(&mut out, &cert.scopes)?;
    encode_audience(&mut out, &cert.aud)?;
    encode_option_fixed_32(&mut out, cert.verifier_role_hash);

    Ok(out)
}

pub fn claims_bytes(claims: &DelegatedTokenClaimsV2) -> Result<Vec<u8>, CanonicalAuthV2Error> {
    let mut out = domain_bytes(CanonicalDomainV2::DelegatedTokenClaims);

    encode_u16(&mut out, claims.version);
    encode_principal(&mut out, claims.subject);
    encode_principal(&mut out, claims.issuer_shard_pid);
    encode_fixed_32(&mut out, claims.cert_hash);
    encode_u64(&mut out, claims.issued_at);
    encode_u64(&mut out, claims.expires_at);
    encode_audience(&mut out, &claims.aud)?;
    encode_scopes(&mut out, &claims.scopes)?;
    out.extend_from_slice(&claims.nonce);

    Ok(out)
}

pub fn root_key_certificate_payload_bytes(cert: &RootKeyCertificateV2) -> Vec<u8> {
    let mut out = domain_bytes(CanonicalDomainV2::RootKeyCertificate);

    encode_principal(&mut out, cert.root_pid);
    encode_string(&mut out, &cert.key_id);
    encode_algorithm(&mut out, cert.alg);
    encode_bytes(&mut out, &cert.public_key_sec1);
    encode_fixed_32(&mut out, cert.key_hash);
    encode_u64(&mut out, cert.not_before);
    encode_option_u64(&mut out, cert.not_after);

    out
}

fn domain_bytes(domain: CanonicalDomainV2) -> Vec<u8> {
    let mut out = Vec::with_capacity(128);
    out.extend_from_slice(DOMAIN_SEPARATOR);
    out.push(domain as u8);
    out
}

fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn encode_algorithm(out: &mut Vec<u8>, alg: SignatureAlgorithmV2) {
    let tag = match alg {
        SignatureAlgorithmV2::EcdsaP256Sha256 => 1,
    };
    out.push(tag);
}

fn encode_audience(
    out: &mut Vec<u8>,
    audience: &DelegationAudienceV2,
) -> Result<(), CanonicalAuthV2Error> {
    match audience {
        DelegationAudienceV2::Roles(roles) => {
            out.push(1);
            encode_roles(out, roles)?;
        }
        DelegationAudienceV2::Principals(principals) => {
            out.push(2);
            encode_principals(out, principals);
        }
        DelegationAudienceV2::RolesOrPrincipals { roles, principals } => {
            out.push(3);
            encode_roles(out, roles)?;
            encode_principals(out, principals);
        }
    }

    Ok(())
}

fn encode_shard_key_binding(out: &mut Vec<u8>, binding: ShardKeyBindingV2) {
    match binding {
        ShardKeyBindingV2::IcThresholdEcdsa {
            key_name_hash,
            derivation_path_hash,
        } => {
            out.push(1);
            encode_fixed_32(out, key_name_hash);
            encode_fixed_32(out, derivation_path_hash);
        }
    }
}

fn encode_roles(out: &mut Vec<u8>, roles: &[CanisterRole]) -> Result<(), CanonicalAuthV2Error> {
    let mut encoded = BTreeSet::new();
    for role in roles {
        validate_role(role)?;
        encoded.insert(role.as_str().as_bytes().to_vec());
    }

    encode_len(out, encoded.len());
    for role in encoded {
        encode_bytes(out, &role);
    }

    Ok(())
}

fn encode_scopes(out: &mut Vec<u8>, scopes: &[String]) -> Result<(), CanonicalAuthV2Error> {
    let mut encoded = BTreeSet::new();
    for scope in scopes {
        validate_scope(scope)?;
        encoded.insert(scope.as_bytes().to_vec());
    }

    encode_len(out, encoded.len());
    for scope in encoded {
        encode_bytes(out, &scope);
    }

    Ok(())
}

fn encode_principals(out: &mut Vec<u8>, principals: &[Principal]) {
    let mut encoded = BTreeSet::new();
    for principal in principals {
        encoded.insert(principal.as_slice().to_vec());
    }

    encode_len(out, encoded.len());
    for principal in encoded {
        encode_bytes(out, &principal);
    }
}

fn validate_role(role: &CanisterRole) -> Result<(), CanonicalAuthV2Error> {
    let role = role.as_str();
    if role.is_empty() {
        return Err(CanonicalAuthV2Error::EmptyRole);
    }
    if !role.bytes().all(is_canonical_label_byte) {
        return Err(CanonicalAuthV2Error::InvalidRole {
            role: role.to_string(),
        });
    }
    Ok(())
}

fn validate_scope(scope: &str) -> Result<(), CanonicalAuthV2Error> {
    if scope.is_empty() {
        return Err(CanonicalAuthV2Error::EmptyScope);
    }
    if !scope.bytes().all(is_canonical_label_byte) {
        return Err(CanonicalAuthV2Error::InvalidScope {
            scope: scope.to_string(),
        });
    }
    Ok(())
}

const fn is_canonical_label_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b':' | b'-')
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

fn encode_option_fixed_32(out: &mut Vec<u8>, bytes: Option<[u8; 32]>) {
    match bytes {
        Some(bytes) => {
            out.push(1);
            encode_fixed_32(out, bytes);
        }
        None => out.push(0),
    }
}

fn encode_option_u64(out: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            out.push(1);
            encode_u64(out, value);
        }
        None => out.push(0),
    }
}

fn encode_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn encode_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn encode_len(out: &mut Vec<u8>, len: usize) {
    let len = u32::try_from(len).expect("delegated auth v2 canonical vector length exceeds u32");
    out.extend_from_slice(&len.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn sample_cert() -> DelegationCertV2 {
        DelegationCertV2 {
            version: 2,
            root_pid: p(1),
            root_key_id: "root-key".to_string(),
            root_key_hash: [2; 32],
            alg: SignatureAlgorithmV2::EcdsaP256Sha256,
            shard_pid: p(3),
            shard_key_id: "shard-key".to_string(),
            shard_public_key_sec1: vec![4, 5, 6],
            shard_key_hash: [7; 32],
            shard_key_binding: ShardKeyBindingV2::IcThresholdEcdsa {
                key_name_hash: [8; 32],
                derivation_path_hash: [9; 32],
            },
            issued_at: 100,
            expires_at: 200,
            max_token_ttl_secs: 60,
            scopes: vec!["write".to_string(), "read".to_string()],
            aud: DelegationAudienceV2::Roles(vec![CanisterRole::new("project_instance")]),
            verifier_role_hash: Some(role_hash(&CanisterRole::new("project_instance")).unwrap()),
        }
    }

    #[test]
    fn cert_hash_canonicalizes_role_scope_and_principal_order() {
        let mut left = sample_cert();
        left.scopes = vec!["write".to_string(), "read".to_string(), "read".to_string()];
        left.aud = DelegationAudienceV2::RolesOrPrincipals {
            roles: vec![
                CanisterRole::new("project_instance"),
                CanisterRole::new("project_instance"),
            ],
            principals: vec![p(9), p(4), p(9)],
        };

        let mut right = sample_cert();
        right.scopes = vec!["read".to_string(), "write".to_string()];
        right.aud = DelegationAudienceV2::RolesOrPrincipals {
            roles: vec![CanisterRole::new("project_instance")],
            principals: vec![p(4), p(9)],
        };

        assert_eq!(cert_hash(&left).unwrap(), cert_hash(&right).unwrap());
    }

    #[test]
    fn cert_hash_rejects_noncanonical_roles() {
        let mut cert = sample_cert();
        cert.aud =
            DelegationAudienceV2::Roles(vec![CanisterRole::owned("ProjectInstance".to_string())]);

        assert_eq!(
            cert_hash(&cert),
            Err(CanonicalAuthV2Error::InvalidRole {
                role: "ProjectInstance".to_string(),
            })
        );
    }

    #[test]
    fn claims_hash_rejects_noncanonical_scopes() {
        let claims = DelegatedTokenClaimsV2 {
            version: 2,
            subject: p(10),
            issuer_shard_pid: p(11),
            cert_hash: [12; 32],
            issued_at: 100,
            expires_at: 120,
            aud: DelegationAudienceV2::Principals(vec![p(13)]),
            scopes: vec!["Read".to_string()],
            nonce: [14; 16],
        };

        assert_eq!(
            claims_hash(&claims),
            Err(CanonicalAuthV2Error::InvalidScope {
                scope: "Read".to_string(),
            })
        );
    }

    #[test]
    fn claims_hash_canonicalizes_audience_and_scope_order() {
        let mut left = DelegatedTokenClaimsV2 {
            version: 2,
            subject: p(10),
            issuer_shard_pid: p(11),
            cert_hash: [12; 32],
            issued_at: 100,
            expires_at: 120,
            aud: DelegationAudienceV2::RolesOrPrincipals {
                roles: vec![
                    CanisterRole::new("project_instance"),
                    CanisterRole::new("project_instance"),
                ],
                principals: vec![p(30), p(20), p(30)],
            },
            scopes: vec!["write".to_string(), "read".to_string(), "read".to_string()],
            nonce: [14; 16],
        };
        let mut right = left.clone();
        right.aud = DelegationAudienceV2::RolesOrPrincipals {
            roles: vec![CanisterRole::new("project_instance")],
            principals: vec![p(20), p(30)],
        };
        right.scopes = vec!["read".to_string(), "write".to_string()];

        assert_eq!(claims_hash(&left).unwrap(), claims_hash(&right).unwrap());

        left.nonce = [15; 16];
        assert_ne!(claims_hash(&left).unwrap(), claims_hash(&right).unwrap());
    }

    #[test]
    fn role_hash_is_domain_separated_from_certificate_hash() {
        let role = CanisterRole::new("project_instance");
        let cert = sample_cert();

        assert_ne!(role_hash(&role).unwrap(), cert_hash(&cert).unwrap());
    }

    #[test]
    fn root_key_certificate_payload_ignores_authority_signature() {
        let mut left = RootKeyCertificateV2 {
            root_pid: p(1),
            key_id: "root-key".to_string(),
            alg: SignatureAlgorithmV2::EcdsaP256Sha256,
            public_key_sec1: vec![1, 2, 3],
            key_hash: [4; 32],
            not_before: 100,
            not_after: Some(200),
            authority_sig: vec![5, 6],
        };
        let mut right = left.clone();
        right.authority_sig = vec![7, 8, 9];

        assert_eq!(
            root_key_certificate_payload_hash(&left),
            root_key_certificate_payload_hash(&right)
        );

        left.key_id = "other-root-key".to_string();
        assert_ne!(
            root_key_certificate_payload_hash(&left),
            root_key_certificate_payload_hash(&right)
        );
    }

    #[test]
    fn derivation_path_hash_preserves_segment_boundaries() {
        let left = vec![b"ab".to_vec(), b"c".to_vec()];
        let right = vec![b"a".to_vec(), b"bc".to_vec()];

        assert_ne!(derivation_path_hash(&left), derivation_path_hash(&right));
        assert_eq!(key_name_hash("dfx_test_key"), key_name_hash("dfx_test_key"));
    }
}
