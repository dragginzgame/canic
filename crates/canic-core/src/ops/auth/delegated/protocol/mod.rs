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
        root_pid: cert.root_pid.into(),
        issuer_pid: cert.issuer_pid.into(),
        issuer_proof_alg: issuer_algorithm(cert.issuer_proof_alg),
        issuer_proof_binding_hash: cert.issuer_proof_binding_hash,
        issuer_proof_binding: issuer_binding(cert.issuer_proof_binding),
        issued_at_ns: cert.issued_at_ns,
        not_before_ns: cert.not_before_ns,
        expires_at_ns: cert.expires_at_ns,
        max_token_ttl_ns: cert.max_token_ttl_ns,
        aud: audience(&cert.aud),
        grants: grants(&cert.grants)?,
    })
}

pub(super) fn chain_key_cert(
    cert: &auth::ChainKeyDelegationCertV1,
) -> Result<protocol::ChainKeyDelegationCertV1, CanonicalAuthError> {
    Ok(protocol::ChainKeyDelegationCertV1 {
        root_canister_id: cert.root_canister_id.into(),
        issuer_canister_id: cert.issuer_canister_id.into(),
        proof_epoch: cert.proof_epoch,
        issuer_proof_algorithm: issuer_algorithm(cert.issuer_proof_algorithm),
        issuer_proof_binding_hash: cert.issuer_proof_binding_hash,
        issuer_proof_binding: issuer_binding(cert.issuer_proof_binding),
        max_token_ttl_ns: cert.max_token_ttl_ns,
        audience: audience(&cert.audience),
        grants: grants(&cert.grants)?,
        not_before_ns: cert.not_before_ns,
        expires_at_ns: cert.expires_at_ns,
        registry_epoch: cert.registry_epoch,
        registry_hash: cert.registry_hash,
    })
}

#[cfg(any(test, feature = "auth-chain-key-ecdsa"))]
pub(super) fn proof_parts(
    certificate: &auth::DelegationCert,
    root_proof: &auth::RootProof,
) -> Result<protocol::DelegationProof, CanonicalAuthError> {
    let auth::RootProof::IcChainKeyBatchSignatureV1(root) = root_proof;
    Ok(protocol::DelegationProof {
        cert: cert(certificate)?,
        root_proof: protocol::RootProof::IcChainKeyBatchSignatureV1(
            protocol::IcChainKeyBatchSignatureProofV1 {
                header: header(&root.header),
                delegation_cert: chain_key_cert(&root.delegation_cert)?,
                issuer_witness: witness(&root.issuer_witness),
                signature: signature(&root.signature),
            },
        ),
    })
}

#[cfg(test)]
pub(in crate::ops::auth) fn proof(
    proof: &auth::DelegationProof,
) -> Result<protocol::DelegationProof, CanonicalAuthError> {
    proof_parts(&proof.cert, &proof.root_proof)
}

pub(in crate::ops::auth) const fn algorithm(
    value: auth::ChainKeyAlgorithm,
) -> protocol::ChainKeyAlgorithm {
    match value {
        auth::ChainKeyAlgorithm::EcdsaSecp256k1 => protocol::ChainKeyAlgorithm::EcdsaSecp256k1,
    }
}

pub(in crate::ops::auth) fn key_id(value: &auth::ChainKeyKeyId) -> protocol::ChainKeyKeyId {
    protocol::ChainKeyKeyId {
        name: value.name.clone(),
    }
}

pub(in crate::ops::auth) const fn issuer_algorithm(
    value: auth::IssuerProofAlgorithm,
) -> protocol::IssuerProofAlgorithm {
    match value {
        auth::IssuerProofAlgorithm::IcCanisterSignatureV1 => {
            protocol::IssuerProofAlgorithm::IcCanisterSignatureV1
        }
    }
}

pub(in crate::ops::auth) const fn issuer_binding(
    value: auth::IssuerProofBinding,
) -> protocol::IssuerProofBinding {
    match value {
        auth::IssuerProofBinding::IcCanisterSignatureV1 { seed_hash } => {
            protocol::IssuerProofBinding::IcCanisterSignatureV1 { seed_hash }
        }
    }
}

pub(in crate::ops::auth) fn header(
    value: &auth::ChainKeyBatchHeaderV1,
) -> protocol::ChainKeyBatchHeaderV1 {
    protocol::ChainKeyBatchHeaderV1 {
        schema_version: value.schema_version,
        root_canister_id: value.root_canister_id.into(),
        batch_id: value.batch_id,
        proof_epoch: value.proof_epoch,
        registry_epoch: value.registry_epoch,
        registry_hash: value.registry_hash,
        tree_root: value.tree_root,
        not_before_ns: value.not_before_ns,
        expires_at_ns: value.expires_at_ns,
        algorithm: algorithm(value.algorithm),
        key_id: key_id(&value.key_id),
        derivation_path_hash: value.derivation_path_hash,
        key_version: value.key_version,
    }
}

#[cfg(any(test, feature = "auth-chain-key-ecdsa"))]
fn witness(value: &auth::ChainKeyBatchWitnessV1) -> protocol::ChainKeyBatchWitnessV1 {
    protocol::ChainKeyBatchWitnessV1 {
        steps: value
            .steps
            .iter()
            .map(|step| match step {
                auth::ChainKeyBatchWitnessStepV1::LeftSibling(hash) => {
                    protocol::ChainKeyBatchWitnessStepV1::LeftSibling(*hash)
                }
                auth::ChainKeyBatchWitnessStepV1::RightSibling(hash) => {
                    protocol::ChainKeyBatchWitnessStepV1::RightSibling(*hash)
                }
            })
            .collect(),
    }
}

pub(in crate::ops::auth) fn witness_response(
    value: protocol::ChainKeyBatchWitnessV1,
) -> auth::ChainKeyBatchWitnessV1 {
    auth::ChainKeyBatchWitnessV1 {
        steps: value
            .steps
            .into_iter()
            .map(|step| match step {
                protocol::ChainKeyBatchWitnessStepV1::LeftSibling(hash) => {
                    auth::ChainKeyBatchWitnessStepV1::LeftSibling(hash)
                }
                protocol::ChainKeyBatchWitnessStepV1::RightSibling(hash) => {
                    auth::ChainKeyBatchWitnessStepV1::RightSibling(hash)
                }
            })
            .collect(),
    }
}

#[cfg(any(test, feature = "auth-chain-key-ecdsa"))]
fn signature(value: &auth::ChainKeyRootSignatureV1) -> protocol::ChainKeyRootSignatureV1 {
    protocol::ChainKeyRootSignatureV1 {
        algorithm: algorithm(value.algorithm),
        key_id: key_id(&value.key_id),
        derivation_path: value.derivation_path.clone(),
        public_key: value.public_key.clone(),
        signature: value.signature.clone(),
    }
}
