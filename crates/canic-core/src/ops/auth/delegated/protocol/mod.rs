//! Module: ops::auth::delegated::protocol
//!
//! Responsibility: project Canic deployment-bound DTOs into IC Auth protocol values.
//! Does not own: encoding, authority enrollment, verification or storage.
//! Boundary: preserve exact identity bytes, grant order and proof material.

use crate::{dto::auth, ops::auth::delegated::canonical::CanonicalAuthError};
use canic_contracts::ids::CanisterRole;
use ic_auth_protocol_types as protocol;

pub(super) fn role(role: &CanisterRole) -> Result<protocol::AuthRole, CanonicalAuthError> {
    role.as_str().parse().map_err(|_| {
        if role.as_str().is_empty() {
            CanonicalAuthError::EmptyRole
        } else {
            CanonicalAuthError::InvalidRole {
                role: role.as_str().to_owned(),
            }
        }
    })
}

const fn audience(audience: &auth::DelegationAudience) -> protocol::DelegationAudience {
    match audience {
        auth::DelegationAudience::Fleet(fleet) => {
            protocol::DelegationAudience::Fleet(protocol::AudienceId {
                canonical_network_id: protocol::CanonicalId::from_bytes(
                    *fleet.canonical_network_id.as_bytes(),
                ),
                fleet_id: protocol::CanonicalId::from_bytes(*fleet.fleet_id.as_bytes()),
            })
        }
    }
}

fn grants(
    grants: &[auth::DelegatedRoleGrant],
) -> Result<Vec<protocol::DelegatedRoleGrant>, CanonicalAuthError> {
    grants
        .iter()
        .map(|grant| {
            Ok(protocol::DelegatedRoleGrant {
                target: role(&grant.target)?,
                scopes: grant.scopes.clone(),
            })
        })
        .collect()
}

pub(super) fn cert(
    cert: &auth::DelegationCert,
) -> Result<protocol::DelegationCert, CanonicalAuthError> {
    Ok(protocol::DelegationCert {
        root_pid: cert.root_pid,
        issuer_pid: cert.issuer_pid,
        issuer_proof_alg: cert.issuer_proof_alg,
        issuer_proof_binding_hash: cert.issuer_proof_binding_hash,
        issuer_proof_binding: cert.issuer_proof_binding,
        issued_at_ns: cert.issued_at_ns,
        not_before_ns: cert.not_before_ns,
        expires_at_ns: cert.expires_at_ns,
        max_token_ttl_ns: cert.max_token_ttl_ns,
        aud: audience(&cert.aud),
        grants: grants(&cert.grants)?,
    })
}

pub(super) fn claims(
    claims: &auth::DelegatedTokenClaims,
) -> Result<protocol::DelegatedTokenClaims, CanonicalAuthError> {
    Ok(protocol::DelegatedTokenClaims {
        presenter: claims.presenter,
        subject: claims.subject,
        issuer_pid: claims.issuer_pid,
        cert_hash: claims.cert_hash,
        issued_at_ns: claims.issued_at_ns,
        expires_at_ns: claims.expires_at_ns,
        aud: audience(&claims.aud),
        grants: grants(&claims.grants)?,
        nonce: claims.nonce,
        ext: claims.ext.clone(),
    })
}

pub(super) fn chain_key_cert(
    cert: &auth::ChainKeyDelegationCertV1,
) -> Result<protocol::ChainKeyDelegationCertV1, CanonicalAuthError> {
    Ok(protocol::ChainKeyDelegationCertV1 {
        root_canister_id: cert.root_canister_id,
        issuer_canister_id: cert.issuer_canister_id,
        proof_epoch: cert.proof_epoch,
        issuer_proof_algorithm: cert.issuer_proof_algorithm,
        issuer_proof_binding_hash: cert.issuer_proof_binding_hash,
        issuer_proof_binding: cert.issuer_proof_binding,
        max_token_ttl_ns: cert.max_token_ttl_ns,
        audience: audience(&cert.audience),
        grants: grants(&cert.grants)?,
        not_before_ns: cert.not_before_ns,
        expires_at_ns: cert.expires_at_ns,
        registry_epoch: cert.registry_epoch,
        registry_hash: cert.registry_hash,
    })
}

pub(super) fn proof(
    proof: &auth::DelegationProof,
) -> Result<protocol::DelegationProof, CanonicalAuthError> {
    let auth::RootProof::IcChainKeyBatchSignatureV1(root) = &proof.root_proof;
    Ok(protocol::DelegationProof {
        cert: cert(&proof.cert)?,
        root_proof: protocol::RootProof::IcChainKeyBatchSignatureV1(
            protocol::IcChainKeyBatchSignatureProofV1 {
                header: root.header.clone(),
                delegation_cert: chain_key_cert(&root.delegation_cert)?,
                issuer_witness: root.issuer_witness.clone(),
                signature: root.signature.clone(),
            },
        ),
    })
}
