//! Module: ops::auth::token
//!
//! Responsibility: prepare issuer-local delegated tokens and verify delegated tokens.
//! Does not own: endpoint authorization, active-proof storage, or auth policy records.
//! Boundary: auth ops facade for delegated-token workflows, config, metrics, and proof helpers.

mod error;
#[cfg(test)]
mod preparation_tests;
mod retention;
pub(in crate::ops::auth) mod verification;
mod verifier_config;

use super::{
    AuthChainKeyRootVerifierConfig, AuthOps, AuthProofVerifierConfig,
    PrepareDelegatedTokenIssuerProofInput, PreparedDelegatedTokenIssuerProof,
    VerifyDelegatedTokenRuntimeInput,
    delegated::{
        audience::{MAX_DELEGATED_ROLE_GRANTS, MAX_SCOPES_PER_ROLE_GRANT},
        protocol,
    },
};
use crate::{
    InternalError,
    cdk::{types::Principal, utils::hash::decode_hex},
    config::schema::DelegatedTokenConfig,
    domain::auth::{IC_ROOT_PUBLIC_KEY_RAW_LENGTH, is_mainnet_ic_root_public_key_raw},
    dto::auth::{
        ActiveDelegationProofStatus, ChainKeyAlgorithm, ChainKeyKeyId, DelegatedToken,
        RootKeyPolicyV1,
    },
    model::auth::application_authorization::VerifiedApplicationAuthority,
    ops::{
        auth::{AuthScopeError, AuthValidationError},
        config::ConfigOps,
        ic::IcOps,
        runtime::{
            env::EnvOps,
            metrics::delegated_auth::{DelegatedAuthMetricReason, DelegatedAuthMetrics},
        },
        storage::{StorageOpsError, fleet_activation::FleetActivationOps},
    },
};
use canic_contracts::ids::{BuildNetwork, CanisterRole, FleetKey};
use error::{active_delegation_proof_unavailable_error, map_prepare_delegated_token_error};
use ic_auth::token_preparation::{
    PrepareDelegatedTokenError, PrepareDelegatedTokenInput, TokenPreparationLimits,
    finish_delegated_token, prepare_delegated_token,
};
#[cfg(test)]
use verification::map_root_delegation_proof_error;
use verification::{
    auth_proof_verifier_config_for_verification, delegated_token_local_context,
    delegated_tokens_config_for_verification, require_current_canister_delegated_token_verifier,
    verify_with_embedded_proofs,
};
use verifier_config::{
    configured_chain_key_root_verifier, configured_ic_root_public_key_raw,
    configured_root_canister_id,
};

pub use error::DelegatedTokenIssuerPrepareError;

const DEFAULT_DELEGATED_AUTH_MAX_TTL_SECS: u64 = 24 * 60 * 60;
const NS_PER_SEC: u64 = 1_000_000_000;

impl AuthOps {
    /// Resolve the current delegated-token verifier lifetime ceiling.
    pub(crate) fn delegated_token_max_ttl_ns() -> Result<u64, InternalError> {
        let cfg = ConfigOps::delegated_tokens_config()?;
        if !cfg.enabled {
            return Err(AuthValidationError::DelegatedTokenAuthDisabled.into());
        }
        cfg.max_ttl_secs
            .unwrap_or(DEFAULT_DELEGATED_AUTH_MAX_TTL_SECS)
            .checked_mul(NS_PER_SEC)
            .ok_or_else(|| InternalError::public(crate::diagnostics::codes::TIME_CAPACITY))
    }

    pub(crate) const fn delegated_token_replay_retention_limits()
    -> crate::ops::replay::receipt::ReplayReceiptRetentionLimits {
        retention::DELEGATED_TOKEN_REPLAY_RETENTION_LIMITS
    }

    /// Prepare delegated-token claims before issuer canister-signature retrieval.
    pub(crate) fn prepare_delegated_token_issuer_proof(
        input: PrepareDelegatedTokenIssuerProofInput,
        operation_id: [u8; 32],
        prepared_by: Principal,
    ) -> Result<PreparedDelegatedTokenIssuerProof, DelegatedTokenIssuerPrepareError> {
        let local = IcOps::canister_self();
        let now_ns = IcOps::now_nanos();
        retention::prune_and_admit(prepared_by, now_ns)?;
        let active_proof = Self::active_delegation_proof(now_ns)?;
        let Some(active_proof) = active_proof else {
            let status = Self::active_delegation_proof_status(now_ns)?.status;
            return Err(match status {
                ActiveDelegationProofStatus::Missing => {
                    DelegatedTokenIssuerPrepareError::MissingProof
                }
                _ => active_delegation_proof_unavailable_error(status).into(),
            });
        };

        if active_proof.proof.cert.issuer_pid != local {
            return Err(InternalError::from(AuthScopeError::IssuerPidMismatch {
                expected: local,
                found: active_proof.proof.cert.issuer_pid,
            })
            .into());
        }

        let proof = protocol::proof(&active_proof.proof)
            .map_err(|_| InternalError::public(crate::diagnostics::codes::SECURITY_INVALID))?;
        let grants = protocol::grants(&input.grants)
            .map_err(|_| InternalError::public(crate::diagnostics::codes::SECURITY_INVALID))?;
        let prepared = prepare_delegated_token(PrepareDelegatedTokenInput {
            proof: &proof,
            authority_expires_at_ns: active_proof.expires_at_ns,
            operation_id,
            prepared_by,
            audience: protocol::audience(&input.audience),
            grants,
            ttl_ns: input.ttl_ns,
            ext: input.ext,
            now_ns,
            limits: TokenPreparationLimits {
                max_grants: MAX_DELEGATED_ROLE_GRANTS,
                max_scopes_per_grant: MAX_SCOPES_PER_ROLE_GRANT,
            },
        })
        .map_err(map_prepare_delegated_token_error)?;

        let claims_hash = prepared.claims_hash();
        let issuer_proof_prepare = Self::prepare_issuer_canister_signature(claims_hash, now_ns)?;

        let claims = protocol::claims_data(prepared.claims().clone());
        retention::insert(
            retention::RetainedDelegatedTokenKey::new(claims_hash, prepared_by),
            retention::RetainedDelegatedToken {
                prepared,
                retrieval_expires_at_ns: issuer_proof_prepare.retrieval_expires_at_ns,
            },
        );

        Ok(PreparedDelegatedTokenIssuerProof {
            claims,
            claims_hash,
            retrieval_expires_at_ns: issuer_proof_prepare.retrieval_expires_at_ns,
        })
    }

    /// Retrieve a prepared delegated token with its issuer canister-signature proof.
    pub(crate) fn get_delegated_token_issuer_proof(
        claims_hash: [u8; 32],
        prepared_by: Principal,
    ) -> Result<DelegatedToken, InternalError> {
        let retained = retention::get(
            &retention::RetainedDelegatedTokenKey::new(claims_hash, prepared_by),
            IcOps::now_nanos(),
        )
        .map_err(|err| AuthValidationError::Auth(err.to_string()))?;
        let issuer_proof =
            Self::get_issuer_canister_signature_proof(claims_hash, IcOps::canister_self())?;

        Ok(protocol::token_data(finish_delegated_token(
            retained.prepared,
            issuer_proof,
        )))
    }

    /// Resolve verifier-local trust anchors for canister-signature auth proofs.
    pub(crate) fn auth_proof_verifier_config() -> Result<AuthProofVerifierConfig, InternalError> {
        let cfg = ConfigOps::delegated_tokens_config()?;
        Self::auth_proof_verifier_config_from(&cfg)
    }

    /// Verify a self-contained delegated token without local proof lookup.
    pub fn verify_token(
        input: VerifyDelegatedTokenRuntimeInput<'_>,
    ) -> Result<VerifiedApplicationAuthority, InternalError> {
        DelegatedAuthMetrics::record_verify_started();

        let cfg = delegated_tokens_config_for_verification()?;
        require_current_canister_delegated_token_verifier()?;
        let ctx = delegated_token_local_context()?;
        crate::perf!("delegated_token_resolve_context");

        let verifier_cfg = auth_proof_verifier_config_for_verification(&cfg)?;
        crate::perf!("delegated_token_resolve_root_policy");
        let verified = verify_with_embedded_proofs(&input, &ctx, &verifier_cfg)?;
        crate::perf!("delegated_token_verify_embedded_proofs");
        DelegatedAuthMetrics::record_verify_completed();
        Ok(verified)
    }

    fn auth_proof_verifier_config_from(
        cfg: &DelegatedTokenConfig,
    ) -> Result<AuthProofVerifierConfig, InternalError> {
        let build_network = cfg.build_network;
        let root_canister_id = configured_root_canister_id(cfg)?;
        let chain_key_root =
            configured_chain_key_root_verifier(cfg, root_canister_id, build_network)?;
        Ok(AuthProofVerifierConfig {
            build_network,
            root_canister_id,
            ic_root_public_key_raw: configured_ic_root_public_key_raw(cfg, build_network)?,
            chain_key_root,
        })
    }
}

#[cfg(test)]
mod tests;
