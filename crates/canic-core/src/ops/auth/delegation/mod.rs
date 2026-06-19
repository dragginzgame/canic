//! Module: ops::auth::delegation
//!
//! Responsibility: expose auth-ops facade methods for active delegation proof
//! state and root delegation proof batch provisioning.
//! Does not own: endpoint authorization, IC call orchestration, or stable
//! auth record layout.
//! Boundary: API/workflow layers call this after endpoint guards have already
//! accepted the caller.
//!
//! Root proof provisioning shape:
//! - prepare runs in a root update and commits canister-signature leaves;
//! - get runs only as a direct root query so root has `data_certificate()`;
//! - install validates retrieved proofs against pending metadata before the
//!   runtime workflow broadcasts issuer-local installs.
//!
//! MVP invariant: pending batch metadata is bounded and pruned, while
//! signature-map leaves are retained.

#[cfg(test)]
mod tests;

mod active;
mod batch;
mod errors;
mod pending;
mod root_issuer_policy;

use super::AuthOps;
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{
        ActiveDelegationProof, ActiveDelegationProofStatusResponse, DelegationProof,
        RootDelegationProofBatchGetRequest, RootDelegationProofBatchGetResponse,
        RootDelegationProofBatchPrepareRequest, RootDelegationProofBatchPrepareResponse,
        RootDelegationProofBatchProof, RootDelegationProofInstallOutcome, RootIssuerPolicyResponse,
        RootIssuerPolicyUpsertRequest,
    },
};

// -----------------------------------------------------------------------------
// AuthOps Facade
// -----------------------------------------------------------------------------

impl AuthOps {
    pub(crate) fn install_active_delegation_proof(
        proof: DelegationProof,
        installed_by: Principal,
    ) -> Result<ActiveDelegationProof, InternalError> {
        active::install_active_delegation_proof(proof, installed_by)
    }

    #[must_use]
    pub(crate) fn active_delegation_proof(now_ns: u64) -> Option<ActiveDelegationProof> {
        active::active_delegation_proof(now_ns)
    }

    pub(crate) fn active_delegation_proof_status(
        now_ns: u64,
    ) -> ActiveDelegationProofStatusResponse {
        active::active_delegation_proof_status(now_ns)
    }

    pub(crate) fn upsert_root_issuer_policy(
        request: RootIssuerPolicyUpsertRequest,
    ) -> Result<RootIssuerPolicyResponse, InternalError> {
        root_issuer_policy::upsert_root_issuer_policy(request)
    }

    pub(crate) fn prepare_delegation_proof_batch(
        request: RootDelegationProofBatchPrepareRequest,
        max_cert_ttl_ns: u64,
        issued_at_ns: u64,
    ) -> Result<RootDelegationProofBatchPrepareResponse, InternalError> {
        batch::prepare_delegation_proof_batch(request, max_cert_ttl_ns, issued_at_ns)
    }

    pub(crate) fn get_delegation_proof_batch(
        request: RootDelegationProofBatchGetRequest,
    ) -> Result<RootDelegationProofBatchGetResponse, InternalError> {
        batch::get_delegation_proof_batch(request)
    }

    pub(crate) fn preflight_delegation_proof_batch_install_proof(
        batch_id: [u8; 32],
        proof: &RootDelegationProofBatchProof,
        now_ns: u64,
    ) -> Result<(), RootDelegationProofInstallOutcome> {
        batch::preflight_delegation_proof_batch_install_proof(batch_id, proof, now_ns)
    }

    pub(crate) fn mark_delegation_proof_batch_installed(
        batch_id: [u8; 32],
        issuer_pid: Principal,
        cert_hash: [u8; 32],
    ) {
        pending::mark_delegation_proof_batch_installed(batch_id, issuer_pid, cert_hash);
    }

    pub(crate) fn prune_expired_delegation_proof_batch_metadata(now_ns: u64) {
        pending::prune_expired_pending_delegation_proof_batch_metadata(now_ns);
    }
}
