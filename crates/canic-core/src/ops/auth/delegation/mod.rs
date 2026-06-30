//! Module: ops::auth::delegation
//!
//! Responsibility: expose auth-ops facade methods for active delegation proof
//! state and root delegation proof batch provisioning.
//! Does not own: endpoint authorization, IC call orchestration, or stable
//! auth record layout.
//! Boundary: API/workflow layers call this after endpoint guards have already
//! accepted the caller.
//!
//! Pre-0.76 bridge-backed root proof provisioning code remains here for
//! historical state and tests. The 0.76 delegated-auth hard cut rejects that
//! path from active `chain_key_batch` auth configuration.

#[cfg(test)]
mod tests;

mod active;
mod batch;
mod chain_key_batch;
mod chain_key_registry;
mod errors;
mod pending;
mod root_issuer_policy;
#[allow(
    dead_code,
    reason = "pre-0.76 bridge-backed renewal scheduler is retained for historical endpoints and tests during the hard-cut migration"
)]
mod root_issuer_renewal;

///
/// RootDelegationRenewalSweepResult
///
/// Summary of one root-managed renewal scheduler sweep.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(
    dead_code,
    reason = "pre-0.76 bridge-backed renewal scheduler is retained for historical endpoints and tests during the hard-cut migration"
)]
pub struct RootDelegationRenewalSweepResult {
    pub prepared_batch_id: Option<[u8; 32]>,
    pub prepared_attempts: usize,
    pub skipped_templates: usize,
}

use super::{
    AuthOps, AuthValidationError,
    delegated::chain_key_signing::{
        ManagementCanisterChainKeySigner, chain_key_signing_policy_from_config,
    },
};
use super::{
    ChainKeyRootDelegationBatchSigningResult, ChainKeyRootDelegationBatchSweepResult,
    PrepareChainKeyRootDelegationBatchInput,
};
use crate::{
    InternalError,
    cdk::types::Principal,
    config::schema::DelegatedTokenConfig,
    dto::auth::{
        ActiveDelegationProof, ActiveDelegationProofStatusResponse, DelegationProof,
        RootDelegationProofBatchGetRequest, RootDelegationProofBatchGetResponse,
        RootDelegationProofBatchInstallRequest, RootDelegationProofBatchPrepareRequest,
        RootDelegationProofBatchPrepareResponse, RootDelegationProofBatchProof,
        RootDelegationProofInstallOutcome, RootDelegationRenewalProofBatchGetRequest,
        RootDelegationRenewalProvisionerListResponse, RootDelegationRenewalProvisionerResponse,
        RootDelegationRenewalProvisionerUpsertRequest, RootDelegationRenewalWorkListResponse,
        RootIssuerPolicyResponse, RootIssuerPolicyUpsertRequest, RootIssuerRenewalStatusRequest,
        RootIssuerRenewalStatusResponse, RootIssuerRenewalTemplateResponse,
        RootIssuerRenewalTemplateUpsertRequest,
    },
    ids::BuildNetwork,
    ops::{config::ConfigOps, ic::IcOps},
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
        now_ns: u64,
    ) -> Result<RootIssuerPolicyResponse, InternalError> {
        root_issuer_policy::upsert_root_issuer_policy(request, now_ns)
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
        now_ns: u64,
    ) -> Result<(), InternalError> {
        root_issuer_renewal::ensure_delegation_renewal_batch_scheduled(batch_id, now_ns)
    }

    #[allow(
        dead_code,
        reason = "pre-0.76 bridge-backed renewal scheduler is retained for historical endpoints and tests during the hard-cut migration"
    )]
    pub(crate) fn prepare_due_delegation_renewals(
        max_cert_ttl_ns: u64,
        now_ns: u64,
    ) -> Result<RootDelegationRenewalSweepResult, InternalError> {
        root_issuer_renewal::prepare_due_delegation_renewals(max_cert_ttl_ns, now_ns)
    }

    #[allow(
        dead_code,
        reason = "0.76 chain-key timer wiring follows registry epoch/hash source wiring"
    )]
    pub(crate) fn prepare_due_chain_key_root_delegation_batch(
        input: PrepareChainKeyRootDelegationBatchInput,
    ) -> Result<ChainKeyRootDelegationBatchSweepResult, InternalError> {
        let config = ConfigOps::delegated_tokens_config()?;
        let signing_policy = chain_key_signing_policy_from_config(
            &config,
            IcOps::canister_self(),
            input.build_network,
        )?;
        let root_key_policy = Self::auth_proof_verifier_config()?
            .chain_key_root
            .ok_or_else(|| {
                AuthValidationError::Auth(
                    "auth.delegated_tokens.chain_key_root_proof is required when root_proof_mode=\"chain_key_batch\""
                        .to_string(),
                )
            })?
            .policy;
        let registry =
            chain_key_registry::current_chain_key_delegated_auth_registry(&root_key_policy)?;
        let max_revocation_latency_ns = required_chain_key_max_revocation_latency_ns(&config)?;
        let result = chain_key_batch::prepare_due_chain_key_root_delegation_batch(
            chain_key_batch::PrepareDueChainKeyRootDelegationBatchInput {
                signing_policy: &signing_policy,
                max_cert_ttl_ns: input.max_cert_ttl_ns,
                max_revocation_latency_ns,
                min_accepted_proof_epoch: input.min_accepted_proof_epoch,
                registry_epoch: registry.snapshot.registry_epoch,
                registry_hash: registry.hash,
                required_issuer_pid: None,
                now_ns: input.now_ns,
            },
        )?;

        Ok(ChainKeyRootDelegationBatchSweepResult {
            batch_id: result.batch_id,
            prepared_issuers: result.prepared_issuers,
            skipped_templates: result.skipped_templates,
            reused_in_flight: result.reused_in_flight,
        })
    }

    #[allow(
        dead_code,
        reason = "0.76 chain-key timer wiring follows registry epoch/hash source wiring"
    )]
    pub(crate) async fn sign_next_chain_key_root_delegation_batch(
        build_network: BuildNetwork,
        now_ns: u64,
    ) -> Result<ChainKeyRootDelegationBatchSigningResult, InternalError> {
        let config = ConfigOps::delegated_tokens_config()?;
        let signing_policy =
            chain_key_signing_policy_from_config(&config, IcOps::canister_self(), build_network)?;
        let mut signer = ManagementCanisterChainKeySigner;
        let result = chain_key_batch::sign_next_chain_key_root_delegation_batch(
            &signing_policy,
            now_ns,
            &mut signer,
        )
        .await?;

        Ok(ChainKeyRootDelegationBatchSigningResult {
            batch_id: result.batch_id,
            signed: result.signed,
            reused_signed: result.reused_signed,
            signing_in_flight: result.signing_in_flight,
        })
    }

    pub(crate) fn start_next_chain_key_root_delegation_batch_install(
        now_ns: u64,
    ) -> Result<Option<RootDelegationProofBatchInstallRequest>, InternalError> {
        chain_key_batch::start_next_chain_key_root_delegation_batch_install(now_ns).map(|plan| {
            plan.map(|plan| RootDelegationProofBatchInstallRequest {
                batch_id: plan.batch_id,
                proofs: plan.proofs,
            })
        })
    }

    pub(crate) async fn get_or_create_chain_key_delegation_proof_for_issuer(
        issuer_pid: Principal,
        build_network: BuildNetwork,
        max_cert_ttl_ns: u64,
        min_accepted_proof_epoch: u64,
        now_ns: u64,
    ) -> Result<Option<RootDelegationProofBatchProof>, InternalError> {
        let config = ConfigOps::delegated_tokens_config()?;
        let signing_policy =
            chain_key_signing_policy_from_config(&config, IcOps::canister_self(), build_network)?;
        let root_key_policy = Self::auth_proof_verifier_config()?
            .chain_key_root
            .ok_or_else(|| {
                AuthValidationError::Auth(
                    "auth.delegated_tokens.chain_key_root_proof is required when root_proof_mode=\"chain_key_batch\""
                        .to_string(),
                )
            })?
            .policy;
        let registry =
            chain_key_registry::current_chain_key_delegated_auth_registry(&root_key_policy)?;
        let max_revocation_latency_ns = required_chain_key_max_revocation_latency_ns(&config)?;
        let mut signer = ManagementCanisterChainKeySigner;

        chain_key_batch::get_or_create_chain_key_delegation_proof_for_issuer(
            chain_key_batch::PrepareDueChainKeyRootDelegationBatchInput {
                signing_policy: &signing_policy,
                max_cert_ttl_ns,
                max_revocation_latency_ns,
                min_accepted_proof_epoch,
                registry_epoch: registry.snapshot.registry_epoch,
                registry_hash: registry.hash,
                required_issuer_pid: Some(issuer_pid),
                now_ns,
            },
            issuer_pid,
            &mut signer,
        )
        .await
    }

    pub(crate) fn record_chain_key_root_delegation_install_success(
        batch_id: [u8; 32],
        issuer_pid: Principal,
        cert_hash: [u8; 32],
        now_ns: u64,
    ) -> bool {
        chain_key_batch::record_chain_key_root_delegation_install_success(
            batch_id, issuer_pid, cert_hash, now_ns,
        )
    }

    pub(crate) fn record_chain_key_root_delegation_install_failure(
        batch_id: [u8; 32],
        issuer_pid: Principal,
        cert_hash: [u8; 32],
        outcome: RootDelegationProofInstallOutcome,
    ) -> bool {
        chain_key_batch::record_chain_key_root_delegation_install_failure(
            batch_id, issuer_pid, cert_hash, outcome,
        )
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

    pub(crate) fn record_manual_delegation_renewal_install_outcome(
        proof: &RootDelegationProofBatchProof,
        outcome: RootDelegationProofInstallOutcome,
        now_ns: u64,
    ) {
        root_issuer_renewal::record_manual_delegation_renewal_install_outcome(
            proof, outcome, now_ns,
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

#[allow(
    dead_code,
    reason = "0.76 chain-key timer wiring follows registry epoch/hash source wiring"
)]
fn required_chain_key_max_revocation_latency_ns(
    config: &DelegatedTokenConfig,
) -> Result<u64, InternalError> {
    let Some(max_revocation_latency_ns) = config.chain_key_root_proof.max_revocation_latency_ns
    else {
        return Err(AuthValidationError::Auth(
            "auth.delegated_tokens.chain_key_root_proof.max_revocation_latency_ns is required when root_proof_mode=\"chain_key_batch\""
                .to_string(),
        )
        .into());
    };
    if max_revocation_latency_ns == 0 {
        return Err(AuthValidationError::Auth(
            "auth.delegated_tokens.chain_key_root_proof.max_revocation_latency_ns must be greater than zero"
                .to_string(),
        )
        .into());
    }
    Ok(max_revocation_latency_ns)
}
