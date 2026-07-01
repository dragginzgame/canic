//! Module: ops::auth::delegation
//!
//! Responsibility: expose auth-ops facade methods for active delegation proof
//! state and chain-key root delegation proof renewal.
//! Does not own: endpoint authorization, IC call orchestration, or stable
//! auth record layout.
//! Boundary: API/workflow layers call this after endpoint guards have already
//! accepted the caller.

mod active;
mod chain_key_batch;
mod chain_key_registry;
mod errors;
mod root_issuer_policy;
mod root_issuer_renewal;

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
        RootDelegationProofBatchInstallRequest, RootDelegationProofBatchProof,
        RootDelegationProofInstallOutcome, RootIssuerPolicyResponse, RootIssuerPolicyUpsertRequest,
        RootIssuerRenewalStatusRequest, RootIssuerRenewalStatusResponse,
        RootIssuerRenewalTemplateResponse, RootIssuerRenewalTemplateUpsertRequest,
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

    pub(crate) fn has_enabled_root_issuer_renewal_templates() -> bool {
        root_issuer_renewal::has_enabled_root_issuer_renewal_templates()
    }

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

    pub(crate) const fn chain_key_ecdsa_enabled() -> bool {
        cfg!(feature = "auth-chain-key-ecdsa")
    }

    pub(crate) const fn chain_key_root_sign_enabled() -> bool {
        cfg!(feature = "auth-chain-key-root-sign")
    }
}

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
