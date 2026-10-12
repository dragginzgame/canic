//! Module: ops::auth::delegated::protocol
//!
//! Responsibility: project Canic deployment-bound DTOs into IC Auth protocol values.
//! Does not own: encoding, authority enrollment, verification or storage.
//! Boundary: preserve exact identity bytes, grant order and proof material.

use crate::{dto::auth, ops::auth::delegated::canonical::CanonicalAuthError};
use canic_contracts::ids::CanisterRole;
use ic_auth_protocol_types as protocol;

pub(in crate::ops::auth) fn role(
    role: &CanisterRole,
) -> Result<protocol::AuthRole, CanonicalAuthError> {
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

pub(in crate::ops::auth) const fn audience(
    audience: &auth::DelegationAudience,
) -> protocol::DelegationAudience {
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

pub(in crate::ops::auth) fn grants(
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

#[cfg(any(test, feature = "auth-delegated-token-verify"))]
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

pub(in crate::ops::auth) fn proof(
    proof: &auth::DelegationProof,
) -> Result<protocol::DelegationProof, CanonicalAuthError> {
    proof_parts(&proof.cert, &proof.root_proof)
}

pub(super) fn proof_parts(
    certificate: &auth::DelegationCert,
    root_proof: &auth::RootProof,
) -> Result<protocol::DelegationProof, CanonicalAuthError> {
    let auth::RootProof::IcChainKeyBatchSignatureV1(root) = root_proof;
    Ok(protocol::DelegationProof {
        cert: cert(certificate)?,
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

fn audience_data(audience: protocol::DelegationAudience) -> auth::DelegationAudience {
    let protocol::DelegationAudience::Fleet(fleet) = audience;
    auth::DelegationAudience::Fleet(canic_contracts::ids::FleetKey {
        canonical_network_id: fleet
            .canonical_network_id
            .to_string()
            .parse()
            .expect("canonical network ID has exactly 32 bytes"),
        fleet_id: canic_contracts::ids::FleetId::from_generated_bytes(*fleet.fleet_id.as_bytes()),
    })
}

fn grants_data(grants: Vec<protocol::DelegatedRoleGrant>) -> Vec<auth::DelegatedRoleGrant> {
    grants
        .into_iter()
        .map(|grant| auth::DelegatedRoleGrant {
            target: CanisterRole::owned(grant.target.as_str().to_owned()),
            scopes: grant.scopes,
        })
        .collect()
}

pub(in crate::ops::auth) fn claims_data(
    claims: protocol::DelegatedTokenClaims,
) -> auth::DelegatedTokenClaims {
    auth::DelegatedTokenClaims {
        presenter: claims.presenter,
        subject: claims.subject,
        issuer_pid: claims.issuer_pid,
        cert_hash: claims.cert_hash,
        issued_at_ns: claims.issued_at_ns,
        expires_at_ns: claims.expires_at_ns,
        aud: audience_data(claims.aud),
        grants: grants_data(claims.grants),
        nonce: claims.nonce,
        ext: claims.ext,
    }
}

#[cfg(any(test, feature = "auth-delegated-token-verify"))]
pub(in crate::ops::auth) fn token(
    token: &auth::DelegatedToken,
) -> Result<protocol::DelegatedToken, CanonicalAuthError> {
    Ok(protocol::DelegatedToken {
        claims: claims(&token.claims)?,
        proof: proof(&token.proof)?,
        issuer_proof: token.issuer_proof.clone(),
    })
}

pub(in crate::ops::auth) fn token_data(token: protocol::DelegatedToken) -> auth::DelegatedToken {
    let cert = token.proof.cert;
    let protocol::RootProof::IcChainKeyBatchSignatureV1(root) = token.proof.root_proof;
    let delegation_cert = root.delegation_cert;
    auth::DelegatedToken {
        claims: claims_data(token.claims),
        proof: auth::DelegationProof {
            cert: auth::DelegationCert {
                root_pid: cert.root_pid,
                issuer_pid: cert.issuer_pid,
                issuer_proof_alg: cert.issuer_proof_alg,
                issuer_proof_binding_hash: cert.issuer_proof_binding_hash,
                issuer_proof_binding: cert.issuer_proof_binding,
                issued_at_ns: cert.issued_at_ns,
                not_before_ns: cert.not_before_ns,
                expires_at_ns: cert.expires_at_ns,
                max_token_ttl_ns: cert.max_token_ttl_ns,
                aud: audience_data(cert.aud),
                grants: grants_data(cert.grants),
            },
            root_proof: auth::RootProof::IcChainKeyBatchSignatureV1(
                auth::IcChainKeyBatchSignatureProofV1 {
                    header: root.header,
                    delegation_cert: auth::ChainKeyDelegationCertV1 {
                        root_canister_id: delegation_cert.root_canister_id,
                        issuer_canister_id: delegation_cert.issuer_canister_id,
                        proof_epoch: delegation_cert.proof_epoch,
                        issuer_proof_algorithm: delegation_cert.issuer_proof_algorithm,
                        issuer_proof_binding_hash: delegation_cert.issuer_proof_binding_hash,
                        issuer_proof_binding: delegation_cert.issuer_proof_binding,
                        max_token_ttl_ns: delegation_cert.max_token_ttl_ns,
                        audience: audience_data(delegation_cert.audience),
                        grants: grants_data(delegation_cert.grants),
                        not_before_ns: delegation_cert.not_before_ns,
                        expires_at_ns: delegation_cert.expires_at_ns,
                        registry_epoch: delegation_cert.registry_epoch,
                        registry_hash: delegation_cert.registry_hash,
                    },
                    issuer_witness: root.issuer_witness,
                    signature: root.signature,
                },
            ),
        },
        issuer_proof: token.issuer_proof,
    }
}
