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

pub use chain_key_batch::ChainKeyRootDelegationBatchInstallPlan;
pub use chain_key_batch::{
    ChainKeyRootDelegationBatchPreparation, ChainKeyRootDelegationBatchPreparePlan,
    ChainKeyRootDelegationIssuerApproval,
};

use super::{
    AuthOps, AuthValidationError,
    delegated::chain_key_signing::{
        ManagementCanisterChainKeySigner, chain_key_signing_policy_from_config,
    },
};
use super::{
    ChainKeyRootDelegationBatchSigningResult, ChainKeyRootDelegationBatchSweepResult,
    PrepareChainKeyRootDelegationBatchInput, RootIssuerRenewalTiming,
};
use crate::{
    InternalError,
    cdk::types::Principal,
    config::schema::DelegatedTokenConfig,
    dto::auth::{
        ActiveDelegationProof, ActiveDelegationProofStatusResponse, DelegationProof,
        RootDelegationProofBatchProof, RootIssuerPolicyResponse, RootIssuerPolicyUpsertRequest,
        RootIssuerRenewalStatusRequest, RootIssuerRenewalStatusResponse,
        RootIssuerRenewalTemplateResponse, RootIssuerRenewalTemplateUpsertRequest,
    },
    ids::BuildNetwork,
    model::auth::{
        ChainKeyRootDelegationInstallFailure, RootIssuerPolicy, RootIssuerRenewalTemplate,
    },
    ops::{config::ConfigOps, ic::IcOps, storage::auth::AuthStateOps},
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

    pub(crate) fn active_delegation_proof(
        now_ns: u64,
    ) -> Result<Option<ActiveDelegationProof>, InternalError> {
        active::active_delegation_proof(now_ns)
    }

    pub(crate) fn active_delegation_proof_status(
        now_ns: u64,
    ) -> Result<ActiveDelegationProofStatusResponse, InternalError> {
        active::active_delegation_proof_status(now_ns)
    }

    pub(crate) fn root_issuer_policy_from_request(
        request: RootIssuerPolicyUpsertRequest,
    ) -> RootIssuerPolicy {
        root_issuer_policy::root_issuer_policy_from_request(request)
    }

    pub(crate) fn commit_root_issuer_policy(policy: RootIssuerPolicy) -> RootIssuerPolicyResponse {
        root_issuer_policy::commit_root_issuer_policy(policy)
    }

    pub(crate) fn root_issuer_policy(issuer_pid: Principal) -> Option<RootIssuerPolicy> {
        crate::ops::storage::auth::AuthStateOps::root_issuer_policy(issuer_pid)
    }

    pub(crate) fn root_issuer_renewal_template_from_request(
        request: RootIssuerRenewalTemplateUpsertRequest,
    ) -> RootIssuerRenewalTemplate {
        root_issuer_renewal::root_issuer_renewal_template_from_request(request)
    }

    pub(crate) fn commit_root_issuer_renewal_template(
        template: RootIssuerRenewalTemplate,
        now_ns: u64,
    ) -> RootIssuerRenewalTemplateResponse {
        root_issuer_renewal::commit_root_issuer_renewal_template(template, now_ns)
    }

    pub(crate) fn root_issuer_renewal_status(
        request: RootIssuerRenewalStatusRequest,
    ) -> RootIssuerRenewalStatusResponse {
        root_issuer_renewal::root_issuer_renewal_status(request)
    }

    pub(crate) fn has_enabled_root_issuer_renewal_templates() -> bool {
        root_issuer_renewal::has_enabled_root_issuer_renewal_templates()
    }

    pub(crate) fn root_issuer_renewal_timing(
        now_ns: u64,
    ) -> Result<RootIssuerRenewalTiming, InternalError> {
        if !Self::has_enabled_root_issuer_renewal_templates() {
            return Ok(RootIssuerRenewalTiming {
                next_deadline_ns: None,
                earliest_active_proof_expires_at_ns: None,
            });
        }

        let (registry_epoch, registry_hash) = current_chain_key_registry_identity()?;
        let earliest_active_proof_expires_at_ns =
            root_issuer_renewal::earliest_active_root_issuer_proof_expiry_ns(
                now_ns,
                registry_epoch,
                registry_hash,
            );
        let next_deadline_ns = chain_key_batch::current_chain_key_batch_deadline_ns(
            now_ns,
            registry_epoch,
            registry_hash,
        )
        .or_else(|| {
            if root_issuer_renewal::all_enabled_root_issuer_proofs_match_registry(
                now_ns,
                registry_epoch,
                registry_hash,
            ) {
                root_issuer_renewal::next_root_issuer_renewal_template_deadline_ns(now_ns)
            } else {
                Some(now_ns)
            }
        });

        Ok(RootIssuerRenewalTiming {
            next_deadline_ns,
            earliest_active_proof_expires_at_ns,
        })
    }

    pub(crate) fn defer_retryable_chain_key_root_delegation_batch(
        now_ns: u64,
        retry_after_ns: u64,
    ) -> Result<bool, InternalError> {
        let (registry_epoch, registry_hash) = current_chain_key_registry_identity()?;
        Ok(chain_key_batch::defer_retryable_chain_key_batch(
            now_ns,
            retry_after_ns,
            registry_epoch,
            registry_hash,
        ))
    }

    pub(crate) fn plan_due_chain_key_root_delegation_batch(
        input: PrepareChainKeyRootDelegationBatchInput,
    ) -> Result<ChainKeyRootDelegationBatchPreparation, InternalError> {
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
                    "auth.delegated_tokens.chain_key_root_proof is required for delegated auth"
                        .to_string(),
                )
            })?
            .policy;
        AuthStateOps::advance_delegated_auth_registry_epoch_at_least(
            root_key_policy.min_accepted_registry_epoch,
        );
        let registry =
            chain_key_registry::current_chain_key_delegated_auth_registry(&root_key_policy)?;
        let max_revocation_latency_ns = required_chain_key_max_revocation_latency_ns(&config)?;
        chain_key_batch::plan_due_chain_key_root_delegation_batch(
            chain_key_batch::PrepareDueChainKeyRootDelegationBatchInput {
                signing_policy: &signing_policy,
                max_cert_ttl_ns: input.max_cert_ttl_ns,
                max_revocation_latency_ns,
                min_accepted_proof_epoch: input.min_accepted_proof_epoch,
                registry_epoch: registry.snapshot.registry_epoch,
                registry_hash: registry.hash,
                required_issuer_pid: input.required_issuer_pid,
                now_ns: input.now_ns,
            },
        )
    }

    pub(crate) fn commit_chain_key_root_delegation_batch(
        plan: ChainKeyRootDelegationBatchPreparePlan,
        approvals: Vec<ChainKeyRootDelegationIssuerApproval>,
    ) -> Result<ChainKeyRootDelegationBatchSweepResult, InternalError> {
        let result = chain_key_batch::commit_chain_key_root_delegation_batch(plan, approvals)?;
        Ok(chain_key_batch_sweep_result(result))
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

    pub(crate) async fn sign_chain_key_root_delegation_batch(
        build_network: BuildNetwork,
        batch_id: [u8; 32],
        now_ns: u64,
    ) -> Result<ChainKeyRootDelegationBatchSigningResult, InternalError> {
        let config = ConfigOps::delegated_tokens_config()?;
        let signing_policy =
            chain_key_signing_policy_from_config(&config, IcOps::canister_self(), build_network)?;
        let mut signer = ManagementCanisterChainKeySigner;
        let result = chain_key_batch::sign_chain_key_root_delegation_batch(
            &signing_policy,
            batch_id,
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
    ) -> Result<Option<ChainKeyRootDelegationBatchInstallPlan>, InternalError> {
        chain_key_batch::start_next_chain_key_root_delegation_batch_install(now_ns)
    }

    pub(crate) fn signed_chain_key_delegation_proof_for_issuer(
        issuer_pid: Principal,
        now_ns: u64,
    ) -> Result<Option<RootDelegationProofBatchProof>, InternalError> {
        let root_key_policy = Self::auth_proof_verifier_config()?
            .chain_key_root
            .ok_or_else(|| {
                AuthValidationError::Auth(
                    "auth.delegated_tokens.chain_key_root_proof is required for delegated auth"
                        .to_string(),
                )
            })?
            .policy;
        let registry =
            chain_key_registry::current_chain_key_delegated_auth_registry(&root_key_policy)?;
        Ok(
            chain_key_batch::signed_chain_key_delegation_proof_for_issuer(
                issuer_pid,
                now_ns,
                registry.snapshot.registry_epoch,
                registry.hash,
            ),
        )
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
        failure: ChainKeyRootDelegationInstallFailure,
    ) -> bool {
        chain_key_batch::record_chain_key_root_delegation_install_failure(
            batch_id, issuer_pid, cert_hash, failure,
        )
    }

    pub(crate) const fn chain_key_ecdsa_enabled() -> bool {
        cfg!(feature = "auth-chain-key-ecdsa")
    }

    pub(crate) const fn chain_key_root_sign_enabled() -> bool {
        cfg!(feature = "auth-chain-key-root-sign")
    }
}

fn current_chain_key_registry_identity() -> Result<(u64, [u8; 32]), InternalError> {
    let root_key_policy = AuthOps::auth_proof_verifier_config()?
        .chain_key_root
        .ok_or_else(|| {
            AuthValidationError::Auth(
                "auth.delegated_tokens.chain_key_root_proof is required for delegated auth"
                    .to_string(),
            )
        })?
        .policy;
    let registry = chain_key_registry::current_chain_key_delegated_auth_registry(&root_key_policy)?;
    Ok((registry.snapshot.registry_epoch, registry.hash))
}

const fn chain_key_batch_sweep_result(
    result: chain_key_batch::PrepareDueChainKeyRootDelegationBatchResult,
) -> ChainKeyRootDelegationBatchSweepResult {
    ChainKeyRootDelegationBatchSweepResult {
        batch_id: result.batch_id,
        prepared_issuers: result.prepared_issuers,
        skipped_templates: result.skipped_templates,
        reused_in_flight: result.reused_in_flight,
    }
}

fn required_chain_key_max_revocation_latency_ns(
    config: &DelegatedTokenConfig,
) -> Result<u64, InternalError> {
    let Some(max_revocation_latency_ns) = config.chain_key_root_proof.max_revocation_latency_ns
    else {
        return Err(AuthValidationError::Auth(
            "auth.delegated_tokens.chain_key_root_proof.max_revocation_latency_ns is required for delegated auth"
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
