//! Module: ops::auth::token::verification
//!
//! Responsibility: execute cached and embedded-proof delegated-token verification.
//! Does not own: endpoint authorization, verifier configuration storage, or token preparation.
//! Boundary: runtime verification context, cache handling, proof callbacks, and typed metrics.

use super::{
    error::{delegated_auth_reason_from_verify_error, map_verify_delegated_token_error},
    verifier_config::{chain_key_policy_from_config, validate_build_network_root_key_pair},
    *,
};

pub(super) struct DelegatedTokenLocalContext {
    local_canister: Principal,
    local_canic_subnet: Principal,
    local_role: CanisterRole,
    local_project: Option<String>,
}

pub(super) fn delegated_tokens_config_for_verification()
-> Result<DelegatedTokenConfig, InternalError> {
    let cfg = match ConfigOps::delegated_tokens_config() {
        Ok(cfg) => cfg,
        Err(err) => {
            DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::InvalidState);
            return Err(err);
        }
    };
    if !cfg.enabled {
        DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::Disabled);
        return Err(AuthValidationError::DelegatedTokenAuthDisabled.into());
    }
    Ok(cfg)
}

pub(super) fn require_current_canister_delegated_token_verifier() -> Result<(), InternalError> {
    let canister_cfg = match ConfigOps::current_canister() {
        Ok(canister_cfg) => canister_cfg,
        Err(err) => {
            DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::InvalidState);
            return Err(err);
        }
    };

    if canister_cfg.auth.delegated_token_verifier {
        return Ok(());
    }

    DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::InvalidState);
    Err(AuthValidationError::Auth(
        "delegated token verifier disabled for this canister; set subnets.<subnet>.canisters.<role>.auth.delegated_token_verifier=true in canic.toml"
            .to_string(),
    )
    .into())
}

pub(super) fn auth_proof_verifier_config_for_verification(
    cfg: &DelegatedTokenConfig,
) -> Result<AuthProofVerifierConfig, InternalError> {
    match AuthOps::auth_proof_verifier_config_from(cfg) {
        Ok(verifier_cfg) => Ok(verifier_cfg),
        Err(err) => {
            DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::RootKey);
            Err(err)
        }
    }
}

pub(super) fn delegated_token_local_context() -> Result<DelegatedTokenLocalContext, InternalError> {
    let local_role = match EnvOps::canister_role() {
        Ok(local_role) => local_role,
        Err(err) => {
            DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::InvalidState);
            return Err(err);
        }
    };
    let local_project = match ConfigOps::get() {
        Ok(cfg) => Some(cfg.app_id().to_string()),
        Err(err) => {
            DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::InvalidState);
            return Err(err);
        }
    };
    let local_canic_subnet = match EnvOps::subnet_pid() {
        Ok(pid) => pid,
        Err(err) => {
            DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::InvalidState);
            return Err(err);
        }
    };

    Ok(DelegatedTokenLocalContext {
        local_canister: IcOps::canister_self(),
        local_canic_subnet,
        local_role,
        local_project,
    })
}

fn delegated_token_verify_input<'a>(
    input: &'a VerifyDelegatedTokenRuntimeInput<'a>,
    ctx: &'a DelegatedTokenLocalContext,
) -> VerifyDelegatedTokenInput<'a> {
    VerifyDelegatedTokenInput {
        token: input.token,
        local_canister: ctx.local_canister,
        local_canic_subnet: Some(ctx.local_canic_subnet),
        local_role: Some(&ctx.local_role),
        local_project: ctx.local_project.as_deref(),
        ttl_limits: DelegatedAuthTtlLimits {
            max_cert_ttl_ns: input.max_cert_ttl_ns,
            max_token_ttl_ns: input.max_token_ttl_ns,
        },
        required_scopes: input.required_scopes,
        now_ns: input.now_ns,
    }
}

pub(super) fn verify_from_positive_cache<'a>(
    input: &'a VerifyDelegatedTokenRuntimeInput<'a>,
    ctx: &'a DelegatedTokenLocalContext,
    cache_key: [u8; 32],
) -> Result<Option<VerifiedDelegatedToken>, InternalError> {
    if positive_cache_get(cache_key, input.now_ns).is_none() {
        return Ok(None);
    }

    verify_delegated_token_cached_proof_identity(delegated_token_verify_input(input, ctx))
        .map(Some)
        .map_err(|err| {
            positive_cache_remove(cache_key);
            DelegatedAuthMetrics::record_verify_failed(delegated_auth_reason_from_verify_error(
                &err,
            ));
            map_verify_delegated_token_error(err)
        })
}

pub(super) fn verify_with_embedded_proofs<'a>(
    input: &'a VerifyDelegatedTokenRuntimeInput<'a>,
    ctx: &'a DelegatedTokenLocalContext,
    verifier_cfg: &'a AuthProofVerifierConfig,
) -> Result<VerifiedDelegatedToken, InternalError> {
    validate_build_network_root_key_pair(
        verifier_cfg.build_network,
        &verifier_cfg.ic_root_public_key_raw,
    )?;
    verify_delegated_token(
        delegated_token_verify_input(input, ctx),
        |cert, root_proof| {
            AuthOps::verify_delegation_root_proof(cert, root_proof, verifier_cfg, input.now_ns)
        },
        |claims_hash, issuer_proof, issuer_pid| {
            AuthOps::verify_issuer_canister_signature_proof(
                claims_hash,
                issuer_proof,
                issuer_pid,
                &verifier_cfg.ic_root_public_key_raw,
            )
        },
    )
    .map_err(|err| {
        DelegatedAuthMetrics::record_verify_failed(delegated_auth_reason_from_verify_error(&err));
        map_verify_delegated_token_error(err)
    })
}

impl AuthOps {
    pub(crate) fn verify_delegation_root_proof(
        cert: &DelegationCert,
        root_proof: &RootProof,
        verifier_cfg: &AuthProofVerifierConfig,
        now_ns: u64,
    ) -> Result<(), InternalError> {
        let root_pid = cert.root_pid;
        if root_pid != verifier_cfg.root_canister_id {
            return Err(InternalError::invalid_input(
                AuthValidationError::InvalidRootAuthority {
                    expected: verifier_cfg.root_canister_id,
                    found: root_pid,
                }
                .to_string(),
            ));
        }

        let Some(chain_key_root) = verifier_cfg.chain_key_root.as_ref() else {
            return Err(InternalError::auth_material_stale(
                "chain-key root verifier policy is not configured",
            ));
        };
        let policy = chain_key_policy_from_config(chain_key_root);
        verify_chain_key_batch_root_proof(
            VerifyChainKeyBatchRootProofInput {
                cert,
                root_proof,
                policy: &policy,
                now_ns,
            },
            verify_chain_key_ecdsa_signature,
        )
        .map_err(map_chain_key_root_proof_error)
    }
}

pub(super) fn map_chain_key_root_proof_error(err: ChainKeyRootProofError) -> InternalError {
    match err {
        err @ (ChainKeyRootProofError::Expired {
            target: "root_key_policy",
        }
        | ChainKeyRootProofError::PolicyMismatch { .. }
        | ChainKeyRootProofError::ProofEpochTooOld { .. }
        | ChainKeyRootProofError::KeyVersionTooOld { .. }
        | ChainKeyRootProofError::RegistryEpochTooOld { .. }) => {
            InternalError::auth_material_stale(err.to_string())
        }
        err @ ChainKeyRootProofError::Expired { .. } => {
            InternalError::auth_proof_expired(err.to_string())
        }
        err @ ChainKeyRootProofError::NotYetValid { .. } => {
            InternalError::auth_proof_pending(err.to_string())
        }
        err => InternalError::invalid_input(err.to_string()),
    }
}

pub(super) fn insert_positive_verification_cache(
    input: &VerifyDelegatedTokenRuntimeInput<'_>,
    cache_key: [u8; 32],
) {
    let valid_until_ns = input
        .token
        .claims
        .expires_at_ns
        .min(input.token.proof.cert.expires_at_ns);
    positive_cache_insert(
        cache_key,
        CachedDelegatedTokenProofValidity {
            valid_until_ns,
            verified_at_ns: input.now_ns,
        },
    );
}
