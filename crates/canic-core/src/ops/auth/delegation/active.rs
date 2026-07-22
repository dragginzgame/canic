//! Module: ops::auth::delegation::active
//!
//! Responsibility: install and report issuer-local active delegation proof state.
//! Does not own: root batch metadata, root issuer policy, or endpoint guards.

use super::super::AuthOps;
use super::errors::map_install_active_delegation_proof_error;
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{
        ActiveDelegationProof, ActiveDelegationProofStatus, ActiveDelegationProofStatusResponse,
        DelegationProof, RootProof,
    },
    ops::{
        auth::delegated::active_proof::{
            InstallActiveDelegationProofInput,
            install_active_delegation_proof as build_active_delegation_proof,
        },
        auth::delegated::chain_key::ChainKeyRootProofError,
        ic::IcOps,
        storage::auth::AuthStateOps,
    },
};

pub(super) fn install_active_delegation_proof(
    proof: DelegationProof,
    installed_by: Principal,
) -> Result<ActiveDelegationProof, InternalError> {
    let cfg = AuthOps::auth_proof_verifier_config()?;
    let now_ns = IcOps::now_nanos();
    let active_proof = build_active_delegation_proof(
        InstallActiveDelegationProofInput {
            proof,
            installed_by,
            this_canister: IcOps::canister_self(),
            now_ns,
        },
        |cert, root_proof| AuthOps::verify_delegation_root_proof(cert, root_proof, &cfg, now_ns),
    )
    .map_err(map_install_active_delegation_proof_error)?;

    set_active_delegation_proof(active_proof.clone());
    Ok(active_proof)
}

pub(super) fn active_delegation_proof(
    now_ns: u64,
) -> Result<Option<ActiveDelegationProof>, InternalError> {
    let proof = AuthStateOps::active_delegation_proof(now_ns);
    if let Some(proof) = proof.as_ref() {
        require_current_epoch_floors(proof)?;
    }
    Ok(proof)
}

pub(super) fn active_delegation_proof_status(
    now_ns: u64,
) -> Result<ActiveDelegationProofStatusResponse, InternalError> {
    let proof = AuthStateOps::active_delegation_proof_snapshot();
    if let Some(proof) = proof.as_ref()
        && now_ns < proof.expires_at_ns
    {
        require_current_epoch_floors(proof)?;
    }
    Ok(active_delegation_proof_status_response(now_ns, proof))
}

fn set_active_delegation_proof(proof: ActiveDelegationProof) {
    AuthStateOps::set_active_delegation_proof(proof);
}

fn require_current_epoch_floors(proof: &ActiveDelegationProof) -> Result<(), InternalError> {
    let verifier = AuthOps::auth_proof_verifier_config()?;
    let policy = verifier.chain_key_root.ok_or_else(|| {
        InternalError::auth_material_stale("chain-key root verifier policy is not configured")
    })?;
    validate_root_proof_epoch_floors(
        &proof.proof.root_proof,
        policy.policy.min_accepted_proof_epoch,
        policy.policy.min_accepted_registry_epoch,
    )
    .map_err(|cause| InternalError::auth_material_stale(cause.to_string()))
}

const fn validate_root_proof_epoch_floors(
    proof: &RootProof,
    min_proof_epoch: u64,
    min_registry_epoch: u64,
) -> Result<(), ChainKeyRootProofError> {
    let RootProof::IcChainKeyBatchSignatureV1(root_proof) = proof;
    if root_proof.header.proof_epoch < min_proof_epoch {
        return Err(ChainKeyRootProofError::ProofEpochTooOld {
            min: min_proof_epoch,
            found: root_proof.header.proof_epoch,
        });
    }
    if root_proof.header.registry_epoch < min_registry_epoch {
        return Err(ChainKeyRootProofError::RegistryEpochTooOld {
            min: min_registry_epoch,
            found: root_proof.header.registry_epoch,
        });
    }
    Ok(())
}

pub(super) fn active_delegation_proof_status_response(
    now_ns: u64,
    proof: Option<ActiveDelegationProof>,
) -> ActiveDelegationProofStatusResponse {
    let Some(proof) = proof else {
        return ActiveDelegationProofStatusResponse {
            status: ActiveDelegationProofStatus::Missing,
            root_pid: None,
            issuer_pid: None,
            cert_hash: None,
            expires_at_ns: None,
            refresh_after_ns: None,
        };
    };

    let status = if now_ns >= proof.expires_at_ns {
        ActiveDelegationProofStatus::Expired
    } else if now_ns >= proof.refresh_after_ns {
        ActiveDelegationProofStatus::RefreshNeeded
    } else {
        ActiveDelegationProofStatus::Valid
    };

    ActiveDelegationProofStatusResponse {
        status,
        root_pid: Some(proof.proof.cert.root_pid),
        issuer_pid: Some(proof.proof.cert.issuer_pid),
        cert_hash: Some(proof.cert_hash),
        expires_at_ns: Some(proof.expires_at_ns),
        refresh_after_ns: Some(proof.refresh_after_ns),
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::auth::test_fixtures::chain_key_root_proof;

    #[test]
    fn active_proof_epoch_floors_reject_stale_restored_material() {
        let proof = chain_key_root_proof(8);

        assert_eq!(
            validate_root_proof_epoch_floors(&proof, 10, 1),
            Err(ChainKeyRootProofError::ProofEpochTooOld { min: 10, found: 9 })
        );
        assert_eq!(
            validate_root_proof_epoch_floors(&proof, 1, 11),
            Err(ChainKeyRootProofError::RegistryEpochTooOld { min: 11, found: 10 })
        );
        assert_eq!(validate_root_proof_epoch_floors(&proof, 9, 10), Ok(()));
    }
}
