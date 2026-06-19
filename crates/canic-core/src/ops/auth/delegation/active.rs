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
        DelegationProof,
    },
    ops::{
        auth::{
            AuthValidationError,
            delegated::active_proof::{
                InstallActiveDelegationProofInput,
                install_active_delegation_proof as build_active_delegation_proof,
            },
            root_canister_sig::RootPayloadKind,
        },
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
        |cert_hash, root_proof, root_pid| {
            if root_pid != cfg.root_canister_id {
                return Err(AuthValidationError::InvalidRootAuthority {
                    expected: cfg.root_canister_id,
                    found: root_pid,
                }
                .to_string());
            }
            AuthOps::verify_root_canister_signature_proof(
                RootPayloadKind::DelegationCert,
                cert_hash,
                root_proof,
                cfg.root_canister_id,
                &cfg.ic_root_public_key_raw,
            )
            .map_err(|err| err.to_string())
        },
    )
    .map_err(map_install_active_delegation_proof_error)?;

    set_active_delegation_proof(active_proof.clone());
    Ok(active_proof)
}

#[must_use]
pub(super) fn active_delegation_proof(now_ns: u64) -> Option<ActiveDelegationProof> {
    AuthStateOps::active_delegation_proof(now_ns)
}

pub(super) fn active_delegation_proof_status(now_ns: u64) -> ActiveDelegationProofStatusResponse {
    active_delegation_proof_status_response(
        now_ns,
        AuthStateOps::active_delegation_proof_snapshot(),
    )
}

fn set_active_delegation_proof(proof: ActiveDelegationProof) {
    AuthStateOps::set_active_delegation_proof(proof);
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
