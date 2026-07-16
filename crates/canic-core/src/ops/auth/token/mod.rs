//! Module: ops::auth::token
//!
//! Responsibility: prepare issuer-local delegated tokens and verify delegated tokens.
//! Does not own: endpoint authorization, active-proof storage, or auth policy records.
//! Boundary: auth ops facade for delegated-token workflows, config, metrics, and proof helpers.

mod error;
mod verification;
mod verifier_config;

use super::{
    AuthChainKeyRootVerifierConfig, AuthOps, AuthProofVerifierConfig,
    PrepareDelegatedTokenIssuerProofInput, PreparedDelegatedTokenIssuerProof,
    VerifyDelegatedTokenRuntimeInput,
    delegated::prepare::{
        PrepareDelegatedTokenError, PrepareDelegatedTokenInput, finish_delegated_token,
        prepare_delegated_token,
    },
    delegated::{
        cache::{
            CachedDelegatedTokenProofValidity, delegated_token_cache_key, positive_cache_get,
            positive_cache_insert, positive_cache_remove,
        },
        cert_rules::DelegatedAuthTtlLimits,
        chain_key::{
            ChainKeyRootProofError, ChainKeyRootVerifierPolicy, VerifyChainKeyBatchRootProofInput,
            verify_chain_key_batch_root_proof, verify_chain_key_ecdsa_public_key_shape,
            verify_chain_key_ecdsa_signature,
        },
        verify::{
            VerifiedDelegatedToken, VerifyDelegatedTokenError, VerifyDelegatedTokenInput,
            verify_delegated_token, verify_delegated_token_cached_proof_identity,
        },
    },
    issuer_canister_sig::IssuerPayloadKind,
};
use crate::{
    InternalError,
    cdk::{types::Principal, utils::hash::decode_hex},
    config::schema::DelegatedTokenConfig,
    domain::auth::{
        DelegatedAuthNetwork, IC_ROOT_PUBLIC_KEY_RAW_LENGTH, is_mainnet_ic_root_public_key_raw,
    },
    dto::auth::{
        ActiveDelegationProofStatus, ChainKeyAlgorithm, ChainKeyKeyId, DelegatedToken,
        DelegationCert, RootKeyPolicyV1, RootProof, RootProofMode,
    },
    ids::{BuildNetwork, CanisterRole},
    ops::{
        auth::{AuthScopeError, AuthValidationError},
        config::ConfigOps,
        ic::IcOps,
        runtime::{
            env::EnvOps,
            metrics::delegated_auth::{DelegatedAuthMetricReason, DelegatedAuthMetrics},
        },
    },
};
use error::{
    active_delegation_proof_unavailable_error, delegated_auth_reason_from_verify_error,
    map_prepare_delegated_token_error, map_verify_delegated_token_error,
};
use std::{cell::RefCell, collections::BTreeMap};
#[cfg(test)]
use verification::map_chain_key_root_proof_error;
use verification::{
    auth_proof_verifier_config_for_verification, delegated_token_local_context,
    delegated_tokens_config_for_verification, insert_positive_verification_cache,
    require_current_canister_delegated_token_verifier, verify_from_positive_cache,
    verify_with_embedded_proofs,
};
use verifier_config::{
    configured_chain_key_root_verifier, configured_delegated_auth_network,
    configured_ic_root_public_key_raw, configured_root_canister_id, configured_root_proof_mode,
};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PendingDelegatedTokenKey {
    claims_hash: [u8; 32],
    prepared_by: Vec<u8>,
}

impl PendingDelegatedTokenKey {
    fn new(claims_hash: [u8; 32], prepared_by: Principal) -> Self {
        Self {
            claims_hash,
            prepared_by: prepared_by.as_slice().to_vec(),
        }
    }
}

thread_local! {
    static PENDING_DELEGATED_TOKENS: RefCell<BTreeMap<PendingDelegatedTokenKey, crate::ops::auth::delegated::prepare::PreparedDelegatedToken>> =
        const { RefCell::new(BTreeMap::new()) };
}

impl AuthOps {
    /// Prepare delegated-token claims before issuer canister-signature retrieval.
    pub(crate) fn prepare_delegated_token_issuer_proof(
        input: PrepareDelegatedTokenIssuerProofInput,
        operation_id: [u8; 32],
        prepared_by: Principal,
    ) -> Result<PreparedDelegatedTokenIssuerProof, InternalError> {
        if input.subject != prepared_by {
            return Err(AuthValidationError::Auth(
                "delegated token prepare subject must match caller".to_string(),
            )
            .into());
        }

        let local = IcOps::canister_self();
        let now_ns = IcOps::now_nanos();
        let active_proof = Self::active_delegation_proof(now_ns)
            .ok_or_else(|| active_delegation_proof_unavailable_error(now_ns))?;

        if active_proof.proof.cert.issuer_pid != local {
            return Err(AuthScopeError::IssuerPidMismatch {
                expected: local,
                found: active_proof.proof.cert.issuer_pid,
            }
            .into());
        }

        let prepared = prepare_delegated_token(PrepareDelegatedTokenInput {
            proof: &active_proof.proof,
            operation_id,
            prepared_by,
            subject: input.subject,
            audience: input.audience,
            grants: input.grants,
            ttl_ns: input.ttl_ns,
            ext: input.ext,
            now_ns,
        })
        .map_err(map_prepare_delegated_token_error)?;

        let claims_hash = prepared.claims_hash;
        let issuer_proof_prepare = Self::prepare_issuer_canister_signature(
            IssuerPayloadKind::DelegatedTokenClaims,
            operation_id,
            claims_hash,
            prepared_by,
            now_ns,
        )?;

        PENDING_DELEGATED_TOKENS.with_borrow_mut(|pending| {
            pending.insert(
                PendingDelegatedTokenKey::new(claims_hash, prepared_by),
                prepared.clone(),
            );
        });

        Ok(PreparedDelegatedTokenIssuerProof {
            prepared,
            claims_hash,
            retrieval_expires_at_ns: issuer_proof_prepare.retrieval_expires_at_ns,
        })
    }

    /// Retrieve a prepared delegated token with its issuer canister-signature proof.
    pub(crate) fn get_delegated_token_issuer_proof(
        claims_hash: [u8; 32],
        prepared_by: Principal,
    ) -> Result<DelegatedToken, InternalError> {
        let key = PendingDelegatedTokenKey::new(claims_hash, prepared_by);
        let prepared = PENDING_DELEGATED_TOKENS.with_borrow(|pending| pending.get(&key).cloned());
        let prepared = prepared.ok_or_else(|| {
            AuthValidationError::Auth(
                "delegated token was not prepared or has been pruned".to_string(),
            )
        })?;
        let issuer_proof = Self::get_issuer_canister_signature_proof(
            IssuerPayloadKind::DelegatedTokenClaims,
            claims_hash,
            prepared_by,
            IcOps::canister_self(),
            IcOps::now_nanos(),
        )?;

        Ok(finish_delegated_token(prepared, issuer_proof))
    }

    /// Resolve verifier-local trust anchors for canister-signature auth proofs.
    pub(crate) fn auth_proof_verifier_config() -> Result<AuthProofVerifierConfig, InternalError> {
        let cfg = ConfigOps::delegated_tokens_config()?;
        Self::auth_proof_verifier_config_from(&cfg)
    }

    /// Verify a self-contained delegated token without local proof lookup.
    pub fn verify_token(
        input: VerifyDelegatedTokenRuntimeInput<'_>,
    ) -> Result<VerifiedDelegatedToken, InternalError> {
        DelegatedAuthMetrics::record_verify_started();

        let cfg = delegated_tokens_config_for_verification()?;
        require_current_canister_delegated_token_verifier()?;
        let ctx = delegated_token_local_context()?;
        crate::perf!("delegated_token_resolve_context");

        let cache_key = match delegated_token_cache_key(input.token, input.caller) {
            Ok(key) => key,
            Err(err) => {
                let err: VerifyDelegatedTokenError = VerifyDelegatedTokenError::Canonical(err);
                DelegatedAuthMetrics::record_verify_failed(
                    delegated_auth_reason_from_verify_error(&err),
                );
                return Err(map_verify_delegated_token_error(err));
            }
        };
        crate::perf!("delegated_token_hash_cache_key");

        if let Some(verified) = verify_from_positive_cache(&input, &ctx, cache_key)? {
            crate::perf!("delegated_token_verify_cached");
            DelegatedAuthMetrics::record_verify_completed();
            return Ok(verified);
        }

        let verifier_cfg = auth_proof_verifier_config_for_verification(&cfg)?;
        crate::perf!("delegated_token_resolve_root_policy");
        let verified = verify_with_embedded_proofs(&input, &ctx, &verifier_cfg)?;
        crate::perf!("delegated_token_verify_embedded_proofs");
        insert_positive_verification_cache(&input, cache_key);
        crate::perf!("delegated_token_cache_verified");
        DelegatedAuthMetrics::record_verify_completed();
        Ok(verified)
    }

    fn auth_proof_verifier_config_from(
        cfg: &DelegatedTokenConfig,
    ) -> Result<AuthProofVerifierConfig, InternalError> {
        let network = configured_delegated_auth_network(cfg)?;
        let root_canister_id = configured_root_canister_id(cfg)?;
        let root_proof_mode = configured_root_proof_mode(cfg)?;
        let chain_key_root = configured_chain_key_root_verifier(cfg, root_canister_id, network)?;
        Ok(AuthProofVerifierConfig {
            network,
            root_canister_id,
            ic_root_public_key_raw: configured_ic_root_public_key_raw(cfg, network)?,
            root_proof_mode,
            chain_key_root,
        })
    }
}

#[cfg(test)]
mod tests;
