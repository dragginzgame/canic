//! Module: ops::auth::delegated::canonical
//!
//! Responsibility: adapt IC Auth encoding and frame Canic registry/key-policy identities.
//! Does not own: proof verification, storage, or endpoint authorization.
//! Boundary: signed protocol bytes are library-owned; deployment envelopes stay local.

use crate::{
    cdk::types::Principal,
    dto::auth::{
        ChainKeyAlgorithm, ChainKeyBatchHeaderV1, ChainKeyDelegationCertV1, ChainKeyKeyId,
        DelegatedAuthRegistrySnapshotV1, DelegatedRoleGrant, DelegatedTokenClaims,
        DelegationAudience, DelegationCert, DelegationProof, IssuerProof, IssuerProofAlgorithm,
        IssuerProofBinding, RootKeyPolicyV1,
    },
    ops::auth::delegated::protocol,
};
use canic_contracts::ids::{BuildNetwork, CanisterRole, FleetKey};
use ic_auth::canonical::CanonicalAuthError as ProtocolError;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[cfg(test)]
pub use ic_auth::canonical::MAX_TOKEN_EXT_BYTES;

const ROOT_KEY_POLICY_DOMAIN: &[u8] = b"CANIC_ROOT_KEY_POLICY_V1";
const DELEGATED_AUTH_REGISTRY_DOMAIN: &[u8] = b"CANIC_DELEGATED_AUTH_REGISTRY_SNAPSHOT_V1";

///
/// CanonicalAuthError
///
/// Typed failure surface for delegated auth canonicalization.
///

#[derive(Debug, Eq, Error, PartialEq)]
pub enum CanonicalAuthError {
    #[error("canonical vector length exceeds u32")]
    LengthOverflow,
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
    #[error("delegated auth audiences must be strictly sorted and unique")]
    NonCanonicalAudiences,
    #[error("delegated auth issuer policies must be strictly sorted and unique")]
    NonCanonicalIssuerPolicies,
    #[error("delegated auth token ext is {len} bytes and exceeds max {max} bytes")]
    TokenExtTooLarge { len: usize, max: usize },
}

impl From<ic_auth::canonical::CanonicalAuthError> for CanonicalAuthError {
    fn from(error: ic_auth::canonical::CanonicalAuthError) -> Self {
        match error {
            ProtocolError::LengthOverflow => Self::LengthOverflow,
            ProtocolError::EmptyScope => Self::EmptyScope,
            ProtocolError::InvalidScope { scope } => Self::InvalidScope { scope },
            ProtocolError::NonCanonicalScopes => Self::NonCanonicalScopes,
            ProtocolError::NonCanonicalRoles => Self::NonCanonicalRoles,
            ProtocolError::TokenExtTooLarge { len, max } => Self::TokenExtTooLarge { len, max },
        }
    }
}

pub fn cert_hash(cert: &DelegationCert) -> Result<[u8; 32], CanonicalAuthError> {
    Ok(ic_auth::canonical::cert_hash(&protocol::cert(cert)?)?)
}

pub fn claims_hash(claims: &DelegatedTokenClaims) -> Result<[u8; 32], CanonicalAuthError> {
    Ok(ic_auth::canonical::claims_hash(&protocol::claims(claims)?)?)
}

pub fn proof_hash(proof: &DelegationProof) -> Result<[u8; 32], CanonicalAuthError> {
    Ok(ic_auth::canonical::proof_hash(&protocol::proof(proof)?)?)
}

pub fn issuer_proof_hash(proof: &IssuerProof) -> Result<[u8; 32], CanonicalAuthError> {
    Ok(ic_auth::canonical::issuer_proof_hash(proof)?)
}

pub fn chain_key_batch_header_hash(
    header: &ChainKeyBatchHeaderV1,
) -> Result<[u8; 32], CanonicalAuthError> {
    Ok(ic_auth::canonical::chain_key_batch_header_hash(header)?)
}

pub fn chain_key_delegation_cert_hash(
    cert: &ChainKeyDelegationCertV1,
) -> Result<[u8; 32], CanonicalAuthError> {
    Ok(ic_auth::canonical::chain_key_delegation_cert_hash(
        &protocol::chain_key_cert(cert)?,
    )?)
}

pub fn chain_key_derivation_path_hash(path: &[Vec<u8>]) -> Result<[u8; 32], CanonicalAuthError> {
    Ok(ic_auth::canonical::chain_key_derivation_path_hash(path)?)
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
) -> Result<[u8; 32], CanonicalAuthError> {
    Ok(ic_auth::canonical::issuer_proof_binding_hash(
        issuer_pid,
        issuer_proof_alg,
        issuer_proof_binding,
    )?)
}

pub fn role_hash(role: &CanisterRole) -> Result<[u8; 32], CanonicalAuthError> {
    Ok(ic_auth::canonical::role_hash(&protocol::role(role)?)?)
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

fn encode_audience(out: &mut Vec<u8>, audience: &DelegationAudience) {
    match audience {
        DelegationAudience::Fleet(fleet) => {
            out.push(1);
            encode_fleet_key(out, *fleet);
        }
    }
}

fn encode_fleet_key(out: &mut Vec<u8>, fleet: FleetKey) {
    encode_fixed_32(out, *fleet.canonical_network_id.as_bytes());
    encode_fixed_32(out, *fleet.fleet_id.as_bytes());
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
        let current = audience_bytes(audience);
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

fn audience_bytes(audience: &DelegationAudience) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    encode_audience(&mut out, audience);
    out
}

fn encode_build_network(out: &mut Vec<u8>, build_network: BuildNetwork) {
    let tag = match build_network {
        BuildNetwork::Ic => 1,
        BuildNetwork::Local => 2,
    };
    out.push(tag);
}

fn encode_bool(out: &mut Vec<u8>, value: bool) {
    out.push(u8::from(value));
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
    protocol::role(role).map(|_| ())
}

pub fn validate_scope_label(scope: &str) -> Result<(), CanonicalAuthError> {
    Ok(ic_auth::canonical::validate_scope_label(scope)?)
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::auth::{
        ChainKeyBatchWitnessStepV1, ChainKeyBatchWitnessV1, ChainKeyRootSignatureV1,
        IcCanisterSignatureProofV1, IcChainKeyBatchSignatureProofV1, RootProof,
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn sample_cert() -> DelegationCert {
        let issuer_proof_alg = IssuerProofAlgorithm::IcCanisterSignatureV1;
        let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 { seed_hash: [8; 32] };
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
            expires_at_ns: 200,
            max_token_ttl_ns: 60,
            aud: DelegationAudience::Fleet(crate::test::support::fleet_key(1)),
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
                audience: DelegationAudience::Fleet(crate::test::support::fleet_key(1)),
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
            algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
            key_id: ChainKeyKeyId {
                name: "test_key_1".to_string(),
            },
            derivation_path_hash: chain_key_derivation_path_hash(&[
                b"canic".to_vec(),
                b"delegation".to_vec(),
            ])
            .unwrap(),
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
            allowed_audiences: vec![DelegationAudience::Fleet(crate::test::support::fleet_key(
                1,
            ))],
            allowed_grants: vec![grant("project_instance", &["read", "write"])],
            max_root_proof_ttl_ns: 600,
            max_token_ttl_ns: 60,
            issuer_proof_algorithm: issuer_proof_alg,
            issuer_proof_binding_hash: issuer_proof_binding_hash(
                issuer_canister_id,
                issuer_proof_alg,
                issuer_proof_binding,
            )
            .unwrap(),
            renewal_template_hash: [41; 32],
        }
    }

    fn registry_snapshot() -> DelegatedAuthRegistrySnapshotV1 {
        DelegatedAuthRegistrySnapshotV1 {
            schema_version: 1,
            root_canister_id: p(1),
            registry_epoch: 3,
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
            presenter: p(10),
            subject: p(10),
            issuer_pid: p(11),
            cert_hash: [12; 32],
            issued_at_ns: 100,
            expires_at_ns: 120,
            aud: DelegationAudience::Fleet(crate::test::support::fleet_key(1)),
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
            presenter: p(10),
            subject: p(10),
            issuer_pid: p(11),
            cert_hash: [12; 32],
            issued_at_ns: 100,
            expires_at_ns: 120,
            aud: DelegationAudience::Fleet(crate::test::support::fleet_key(1)),
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
    fn claims_hash_binds_signed_presenter() {
        let claims = DelegatedTokenClaims {
            presenter: p(10),
            subject: p(10),
            issuer_pid: p(11),
            cert_hash: [12; 32],
            issued_at_ns: 100,
            expires_at_ns: 120,
            aud: DelegationAudience::Fleet(crate::test::support::fleet_key(1)),
            grants: vec![grant("project_instance", &["read"])],
            nonce: [14; 16],
            ext: None,
        };
        let mut changed_presenter = claims.clone();
        changed_presenter.presenter = p(9);

        assert_ne!(
            claims_hash(&claims).unwrap(),
            claims_hash(&changed_presenter).unwrap()
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
        let header_hash = chain_key_batch_header_hash(&proof.header).unwrap();
        let cert_hash = chain_key_delegation_cert_hash(&proof.delegation_cert).unwrap();
        let mut changed_header = proof.header.clone();
        changed_header.key_version += 1;
        let mut changed_cert = proof.delegation_cert;
        changed_cert.max_token_ttl_ns += 1;

        assert_ne!(
            header_hash,
            chain_key_batch_header_hash(&changed_header).unwrap()
        );
        assert_ne!(
            cert_hash,
            chain_key_delegation_cert_hash(&changed_cert).unwrap()
        );
    }

    #[test]
    fn chain_key_canonical_hashes_match_golden_fixtures() {
        let proof = chain_key_proof();

        assert_eq!(
            chain_key_batch_header_hash(&proof.header).unwrap(),
            [
                231, 134, 199, 186, 130, 244, 250, 243, 254, 252, 150, 140, 3, 154, 230, 252, 45,
                52, 89, 215, 119, 228, 233, 231, 245, 96, 54, 45, 33, 18, 44, 192,
            ]
        );
        assert_eq!(
            chain_key_delegation_cert_hash(&proof.delegation_cert).unwrap(),
            [
                229, 125, 46, 9, 210, 247, 67, 53, 147, 231, 131, 196, 39, 154, 47, 102, 180, 230,
                40, 138, 15, 36, 24, 77, 76, 112, 166, 245, 86, 22, 94, 216,
            ]
        );
        assert_eq!(
            root_key_policy_hash(&root_key_policy()),
            [
                2, 0, 107, 199, 176, 230, 166, 202, 46, 83, 56, 58, 11, 135, 50, 198, 84, 127, 41,
                247, 16, 170, 144, 233, 71, 115, 247, 80, 245, 70, 45, 130,
            ]
        );
        assert_eq!(
            delegated_auth_registry_hash(&registry_snapshot()).unwrap(),
            [
                252, 195, 129, 249, 237, 10, 248, 99, 142, 69, 196, 148, 124, 251, 232, 153, 92,
                99, 53, 228, 121, 76, 34, 241, 69, 48, 243, 134, 180, 173, 194, 44,
            ]
        );
        assert_eq!(
            proof_hash(&DelegationProof {
                cert: sample_cert(),
                root_proof: RootProof::IcChainKeyBatchSignatureV1(proof),
            })
            .unwrap(),
            [
                40, 0, 43, 111, 76, 97, 74, 157, 46, 216, 136, 161, 160, 142, 247, 203, 124, 187,
                2, 145, 177, 196, 48, 209, 112, 103, 122, 135, 249, 222, 243, 111,
            ]
        );
    }

    #[test]
    fn chain_key_v1_domains_and_registry_schema_remain_fixed() {
        assert_eq!(ROOT_KEY_POLICY_DOMAIN, b"CANIC_ROOT_KEY_POLICY_V1");
        assert_eq!(
            DELEGATED_AUTH_REGISTRY_DOMAIN,
            b"CANIC_DELEGATED_AUTH_REGISTRY_SNAPSHOT_V1"
        );
        assert_eq!(registry_snapshot().schema_version, 1);
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
            DelegationAudience::Fleet(crate::test::support::fleet_key(2)),
            DelegationAudience::Fleet(crate::test::support::fleet_key(1)),
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
            presenter: p(10),
            subject: p(10),
            issuer_pid: p(11),
            cert_hash: [12; 32],
            issued_at_ns: 100,
            expires_at_ns: 120,
            aud: DelegationAudience::Fleet(crate::test::support::fleet_key(1)),
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
            presenter: p(10),
            subject: p(10),
            issuer_pid: p(11),
            cert_hash: [12; 32],
            issued_at_ns: 100,
            expires_at_ns: 120,
            aud: DelegationAudience::Fleet(crate::test::support::fleet_key(1)),
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
            issuer_proof_hash(&proof).unwrap(),
            issuer_proof_hash(&changed_signature).unwrap()
        );
        assert_ne!(
            issuer_proof_hash(&proof).unwrap(),
            issuer_proof_hash(&changed_public_key).unwrap()
        );
    }

    #[test]
    fn issuer_proof_binding_hash_binds_authority_context() {
        let binding = IssuerProofBinding::IcCanisterSignatureV1 { seed_hash: [7; 32] };
        let base =
            issuer_proof_binding_hash(p(1), IssuerProofAlgorithm::IcCanisterSignatureV1, binding)
                .unwrap();

        assert_ne!(
            base,
            issuer_proof_binding_hash(p(2), IssuerProofAlgorithm::IcCanisterSignatureV1, binding)
                .unwrap()
        );
        assert_ne!(
            base,
            issuer_proof_binding_hash(
                p(1),
                IssuerProofAlgorithm::IcCanisterSignatureV1,
                IssuerProofBinding::IcCanisterSignatureV1 { seed_hash: [8; 32] },
            )
            .unwrap()
        );
    }
}
