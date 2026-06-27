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
mod root_issuer_renewal;

///
/// RootDelegationRenewalSweepResult
///
/// Summary of one root-managed renewal scheduler sweep.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootDelegationRenewalSweepResult {
    pub prepared_batch_id: Option<[u8; 32]>,
    pub prepared_attempts: usize,
    pub skipped_templates: usize,
}

use super::AuthOps;
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{
        ActiveDelegationProof, ActiveDelegationProofStatusResponse, DelegationProof,
        RootDelegationProofBatchGetRequest, RootDelegationProofBatchGetResponse,
        RootDelegationProofBatchPrepareRequest, RootDelegationProofBatchPrepareResponse,
        RootDelegationProofBatchProof, RootDelegationProofInstallOutcome,
        RootDelegationRenewalProofBatchGetRequest, RootDelegationRenewalProvisionerListResponse,
        RootDelegationRenewalProvisionerResponse, RootDelegationRenewalProvisionerUpsertRequest,
        RootDelegationRenewalWorkListResponse, RootIssuerPolicyResponse,
        RootIssuerPolicyUpsertRequest, RootIssuerRenewalStatusRequest,
        RootIssuerRenewalStatusResponse, RootIssuerRenewalTemplateResponse,
        RootIssuerRenewalTemplateUpsertRequest,
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

    pub(crate) fn upsert_root_issuer_renewal_template(
        request: RootIssuerRenewalTemplateUpsertRequest,
        now_ns: u64,
    ) -> Result<RootIssuerRenewalTemplateResponse, InternalError> {
        root_issuer_renewal::upsert_root_issuer_renewal_template(request, now_ns)
    }

    pub(crate) fn root_issuer_renewal_status(
        request: RootIssuerRenewalStatusRequest,
    ) -> RootIssuerRenewalStatusResponse {
        root_issuer_renewal::root_issuer_renewal_status(request)
    }

    pub(crate) fn get_delegation_renewal_proof_batch(
        request: RootDelegationRenewalProofBatchGetRequest,
    ) -> Result<RootDelegationProofBatchGetResponse, InternalError> {
        root_issuer_renewal::get_delegation_renewal_proof_batch(request)
    }

    pub(crate) fn has_enabled_root_issuer_renewal_templates() -> bool {
        root_issuer_renewal::has_enabled_root_issuer_renewal_templates()
    }

    pub(crate) fn upsert_delegation_renewal_provisioner(
        request: RootDelegationRenewalProvisionerUpsertRequest,
    ) -> RootDelegationRenewalProvisionerResponse {
        root_issuer_renewal::upsert_delegation_renewal_provisioner(request)
    }

    pub(crate) fn delegation_renewal_provisioners() -> RootDelegationRenewalProvisionerListResponse
    {
        root_issuer_renewal::delegation_renewal_provisioners()
    }

    pub(crate) fn delegation_renewal_work(now_ns: u64) -> RootDelegationRenewalWorkListResponse {
        root_issuer_renewal::delegation_renewal_work(now_ns)
    }

    pub(crate) fn is_delegation_renewal_provisioner(principal: Principal) -> bool {
        root_issuer_renewal::is_delegation_renewal_provisioner(principal)
    }

    pub(crate) fn ensure_delegation_renewal_batch_scheduled(
        batch_id: [u8; 32],
    ) -> Result<(), InternalError> {
        root_issuer_renewal::ensure_delegation_renewal_batch_scheduled(batch_id)
    }

    pub(crate) fn prepare_due_delegation_renewals(
        max_cert_ttl_ns: u64,
        now_ns: u64,
    ) -> Result<RootDelegationRenewalSweepResult, InternalError> {
        root_issuer_renewal::prepare_due_delegation_renewals(max_cert_ttl_ns, now_ns)
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

    pub(crate) fn preflight_delegation_renewal_proof_install(
        batch_id: [u8; 32],
        proof: &RootDelegationProofBatchProof,
        now_ns: u64,
    ) -> Result<Option<[u8; 32]>, RootDelegationProofInstallOutcome> {
        root_issuer_renewal::preflight_delegation_renewal_proof_install(batch_id, proof, now_ns)
    }

    pub(crate) fn record_delegation_renewal_install_outcome(
        attempt_id: [u8; 32],
        outcome: RootDelegationProofInstallOutcome,
        now_ns: u64,
    ) {
        root_issuer_renewal::record_delegation_renewal_install_outcome(attempt_id, outcome, now_ns);
    }

    pub(crate) fn record_delegation_renewal_install_preflight_outcome(
        batch_id: [u8; 32],
        issuer_pid: Principal,
        cert_hash: [u8; 32],
        outcome: RootDelegationProofInstallOutcome,
        now_ns: u64,
    ) {
        root_issuer_renewal::record_delegation_renewal_install_preflight_outcome(
            batch_id, issuer_pid, cert_hash, outcome, now_ns,
        );
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
