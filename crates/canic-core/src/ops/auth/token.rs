use super::{
    AuthOps, DelegatedTokenVerifierConfig, PreparedDelegatedTokenSignature,
    SignDelegatedTokenInput, VerifyDelegatedTokenRuntimeInput,
    delegated::mint::{
        MintDelegatedTokenError, MintDelegatedTokenInput, finish_delegated_token,
        prepare_delegated_token,
    },
    delegated::{
        cache::{
            CachedDelegatedTokenProofValidity, delegated_token_cache_key, positive_cache_get,
            positive_cache_insert, positive_cache_remove,
        },
        canonical::{derivation_path_hash, key_name_hash},
        cert_rules::DelegatedAuthTtlLimits,
        verify::{
            VerifiedDelegatedToken, VerifyDelegatedTokenError, VerifyDelegatedTokenInput,
            verify_delegated_token, verify_delegated_token_without_signatures,
        },
    },
    keys,
    root_canister_sig::{IC_ROOT_PUBLIC_KEY_RAW_LENGTH, RootPayloadKind},
};
use crate::{
    InternalError,
    cdk::{types::Principal, utils::hash::decode_hex},
    config::schema::DelegatedTokenConfig,
    dto::auth::{DelegatedToken, ShardKeyBinding},
    ids::CanisterRole,
    ops::{
        auth::{AuthScopeError, AuthValidationError},
        config::ConfigOps,
        cost_guard::CostGuardPermit,
        ic::{IcOps, ecdsa::EcdsaOps},
        replay::model::{EcdsaPurpose, ExternalEffectDescriptor},
        runtime::{
            env::EnvOps,
            metrics::delegated_auth::{DelegatedAuthMetricReason, DelegatedAuthMetrics},
        },
    },
};

impl AuthOps {
    /// Prepare delegated-token claims before shard ECDSA signing.
    pub(crate) fn prepare_delegated_token_signature(
        input: SignDelegatedTokenInput,
    ) -> Result<PreparedDelegatedTokenSignature, InternalError> {
        let local = IcOps::canister_self();
        if input.proof.cert.shard_pid != local {
            return Err(AuthScopeError::ShardPidMismatch {
                expected: local,
                found: input.proof.cert.shard_pid,
            }
            .into());
        }

        let prepared = prepare_delegated_token(MintDelegatedTokenInput {
            proof: &input.proof,
            subject: input.subject,
            audience: input.audience,
            grants: input.grants,
            ttl_ns: input.ttl_ns,
            nonce: input.nonce,
            now_ns: IcOps::now_nanos(),
        })
        .map_err(map_mint_delegated_token_error)?;

        let key_name = keys::delegated_tokens_key_name()?;
        let derivation_path = keys::shard_derivation_path(local);
        Ok(PreparedDelegatedTokenSignature {
            message_hash: prepared.shard_token_hash,
            prepared,
            key_name,
            derivation_path,
        })
    }

    /// Sign prepared delegated-token claims with local shard threshold ECDSA material.
    pub(crate) async fn sign_prepared_delegated_token(
        permit: &CostGuardPermit,
        prepared: PreparedDelegatedTokenSignature,
    ) -> Result<DelegatedToken, InternalError> {
        let PreparedDelegatedTokenSignature {
            prepared,
            message_hash,
            key_name,
            derivation_path,
        } = prepared;
        DelegatedAuthMetrics::record_shard_token_sign_started();
        let shard_sig =
            match EcdsaOps::sign_bytes(permit, &key_name, derivation_path, message_hash).await {
                Ok(signature) => signature,
                Err(err) => {
                    DelegatedAuthMetrics::record_shard_token_sign_failed();
                    return Err(err);
                }
            };
        DelegatedAuthMetrics::record_shard_token_sign_completed();

        Ok(finish_delegated_token(prepared, shard_sig))
    }

    /// Describe the shard ECDSA effect for a prepared delegated-token signature.
    pub(crate) fn delegated_token_signing_effect(
        prepared: &PreparedDelegatedTokenSignature,
    ) -> ExternalEffectDescriptor {
        ExternalEffectDescriptor::ThresholdEcdsaSign {
            key_id_hash: key_name_hash(&prepared.key_name),
            purpose: EcdsaPurpose::DelegatedToken,
            message_hash: prepared.message_hash,
        }
    }

    /// Resolve verifier-local trust anchors for delegated-token verification.
    pub(crate) fn delegated_token_verifier_config()
    -> Result<DelegatedTokenVerifierConfig, InternalError> {
        let cfg = ConfigOps::delegated_tokens_config()?;
        Self::delegated_token_verifier_config_from(&cfg)
    }

    /// Verify a self-contained delegated token without local proof lookup.
    pub fn verify_token(
        input: VerifyDelegatedTokenRuntimeInput<'_>,
    ) -> Result<VerifiedDelegatedToken, InternalError> {
        DelegatedAuthMetrics::record_verify_started();

        let cfg = delegated_tokens_config_for_verification()?;
        let ctx = delegated_token_runtime_context(&cfg)?;

        if let Err(err) = Self::verify_shard_key_binding(input.token) {
            DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::ShardKeyBinding);
            return Err(err);
        }

        let cache_key = match delegated_token_cache_key(input.token, input.caller) {
            Ok(key) => key,
            Err(err) => {
                let err = VerifyDelegatedTokenError::Canonical(err);
                DelegatedAuthMetrics::record_verify_failed(
                    delegated_auth_reason_from_verify_error(&err),
                );
                return Err(map_verify_delegated_token_error(err));
            }
        };

        if let Some(verified) = verify_from_positive_cache(&input, &ctx, cache_key)? {
            DelegatedAuthMetrics::record_verify_completed();
            return Ok(verified);
        }

        let verified = verify_with_signatures(&input, &ctx)?;
        insert_positive_verification_cache(&input, cache_key);
        DelegatedAuthMetrics::record_verify_completed();
        Ok(verified)
    }

    fn delegated_token_verifier_config_from(
        cfg: &DelegatedTokenConfig,
    ) -> Result<DelegatedTokenVerifierConfig, InternalError> {
        Ok(DelegatedTokenVerifierConfig {
            root_canister_id: configured_root_canister_id(cfg)?,
            ic_root_public_key_raw: configured_ic_root_public_key_raw(cfg)?,
        })
    }

    fn verify_shard_key_binding(token: &DelegatedToken) -> Result<(), InternalError> {
        let cert = &token.proof.cert;
        let key_name = keys::delegated_tokens_key_name()?;
        let expected_derivation_path_hash =
            derivation_path_hash(&keys::shard_derivation_path(cert.shard_pid));
        match cert.shard_key_binding {
            ShardKeyBinding::IcThresholdEcdsaSecp256k1 {
                key_name_hash: observed_key_name_hash,
                derivation_path_hash: observed_derivation_path_hash,
            } => {
                if observed_key_name_hash != key_name_hash(&key_name)
                    || observed_derivation_path_hash != expected_derivation_path_hash
                {
                    return Err(AuthValidationError::Auth(
                        "delegated auth shard key binding mismatch".to_string(),
                    )
                    .into());
                }
            }
        }
        Ok(())
    }
}

struct DelegatedTokenRuntimeContext {
    verifier_cfg: DelegatedTokenVerifierConfig,
    local_role: CanisterRole,
    local_project: Option<String>,
}

fn delegated_tokens_config_for_verification() -> Result<DelegatedTokenConfig, InternalError> {
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

fn delegated_token_runtime_context(
    cfg: &DelegatedTokenConfig,
) -> Result<DelegatedTokenRuntimeContext, InternalError> {
    let verifier_cfg = match AuthOps::delegated_token_verifier_config_from(cfg) {
        Ok(verifier_cfg) => verifier_cfg,
        Err(err) => {
            DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::RootKey);
            return Err(err);
        }
    };
    let local_role = match EnvOps::canister_role() {
        Ok(local_role) => local_role,
        Err(err) => {
            DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::InvalidState);
            return Err(err);
        }
    };
    let local_project = match ConfigOps::get() {
        Ok(cfg) => cfg.fleet_name().map(ToOwned::to_owned),
        Err(err) => {
            DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::InvalidState);
            return Err(err);
        }
    };

    Ok(DelegatedTokenRuntimeContext {
        verifier_cfg,
        local_role,
        local_project,
    })
}

fn delegated_token_verify_input<'a>(
    input: &'a VerifyDelegatedTokenRuntimeInput<'a>,
    ctx: &'a DelegatedTokenRuntimeContext,
) -> VerifyDelegatedTokenInput<'a> {
    VerifyDelegatedTokenInput {
        token: input.token,
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

fn verify_from_positive_cache<'a>(
    input: &'a VerifyDelegatedTokenRuntimeInput<'a>,
    ctx: &'a DelegatedTokenRuntimeContext,
    cache_key: [u8; 32],
) -> Result<Option<VerifiedDelegatedToken>, InternalError> {
    if positive_cache_get(cache_key, input.now_ns).is_none() {
        return Ok(None);
    }

    verify_delegated_token_without_signatures(delegated_token_verify_input(input, ctx))
        .map(Some)
        .map_err(|err| {
            positive_cache_remove(cache_key);
            DelegatedAuthMetrics::record_verify_failed(delegated_auth_reason_from_verify_error(
                &err,
            ));
            map_verify_delegated_token_error(err)
        })
}

fn verify_with_signatures<'a>(
    input: &'a VerifyDelegatedTokenRuntimeInput<'a>,
    ctx: &'a DelegatedTokenRuntimeContext,
) -> Result<VerifiedDelegatedToken, InternalError> {
    verify_delegated_token(
        delegated_token_verify_input(input, ctx),
        |cert_hash, root_proof, root_pid| {
            if root_pid != ctx.verifier_cfg.root_canister_id {
                return Err(AuthValidationError::InvalidRootAuthority {
                    expected: ctx.verifier_cfg.root_canister_id,
                    found: root_pid,
                }
                .to_string());
            }
            AuthOps::verify_root_canister_signature_proof(
                RootPayloadKind::DelegationCert,
                cert_hash,
                root_proof,
                ctx.verifier_cfg.root_canister_id,
                &ctx.verifier_cfg.ic_root_public_key_raw,
            )
            .map_err(|err| err.to_string())
        },
        |public_key, hash, sig| {
            if public_key.len() != 33 {
                return Err("delegated auth shard public key is not compressed SEC1".to_string());
            }
            EcdsaOps::verify_signature(public_key, hash, sig).map_err(|err| err.to_string())
        },
    )
    .map_err(|err| {
        DelegatedAuthMetrics::record_verify_failed(delegated_auth_reason_from_verify_error(&err));
        map_verify_delegated_token_error(err)
    })
}

fn insert_positive_verification_cache(
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

fn configured_root_canister_id(cfg: &DelegatedTokenConfig) -> Result<Principal, InternalError> {
    let Some(root_canister_id) = cfg.root_canister_id.as_deref() else {
        return EnvOps::root_pid();
    };
    let root_canister_id = root_canister_id.trim();
    if root_canister_id.is_empty() {
        return Err(AuthValidationError::Auth(
            "auth.delegated_tokens.root_canister_id must not be empty when set".to_string(),
        )
        .into());
    }

    Principal::from_text(root_canister_id).map_err(|err| {
        AuthValidationError::Auth(format!(
            "auth.delegated_tokens.root_canister_id is not a valid principal: {err}"
        ))
        .into()
    })
}

fn configured_ic_root_public_key_raw(cfg: &DelegatedTokenConfig) -> Result<Vec<u8>, InternalError> {
    let ic_root_public_key_raw = match cfg.ic_root_public_key_raw_hex.as_deref() {
        Some(root_key_hex) => {
            let root_key_hex = root_key_hex.trim();
            if root_key_hex.is_empty() {
                return Err(AuthValidationError::Auth(
                    "auth.delegated_tokens.ic_root_public_key_raw_hex must not be empty when set"
                        .to_string(),
                )
                .into());
            }
            decode_hex(root_key_hex).map_err(|err| {
                AuthValidationError::Auth(format!(
                    "auth.delegated_tokens.ic_root_public_key_raw_hex is not valid hex: {err}"
                ))
            })?
        }
        None => AuthOps::ic_root_public_key_raw()?,
    };

    if ic_root_public_key_raw.len() != IC_ROOT_PUBLIC_KEY_RAW_LENGTH {
        return Err(AuthValidationError::Auth(format!(
            "auth.delegated_tokens.ic_root_public_key_raw must be {IC_ROOT_PUBLIC_KEY_RAW_LENGTH} raw bytes"
        ))
        .into());
    }

    Ok(ic_root_public_key_raw)
}

fn map_mint_delegated_token_error(err: MintDelegatedTokenError) -> InternalError {
    AuthValidationError::Auth(err.to_string()).into()
}

fn map_verify_delegated_token_error(err: VerifyDelegatedTokenError) -> InternalError {
    AuthValidationError::Auth(err.to_string()).into()
}

// Convert typed verifier failures into bounded metric reasons.
const fn delegated_auth_reason_from_verify_error(
    err: &VerifyDelegatedTokenError,
) -> DelegatedAuthMetricReason {
    match err {
        VerifyDelegatedTokenError::Audience(_) => DelegatedAuthMetricReason::Audience,
        VerifyDelegatedTokenError::AudienceNotSubset => {
            DelegatedAuthMetricReason::AudienceNotSubset
        }
        VerifyDelegatedTokenError::Canonical(_) => DelegatedAuthMetricReason::Canonical,
        VerifyDelegatedTokenError::CertAudienceRejected => {
            DelegatedAuthMetricReason::CertAudienceRejected
        }
        VerifyDelegatedTokenError::CertExpired => DelegatedAuthMetricReason::CertExpired,
        VerifyDelegatedTokenError::CertHashMismatch => DelegatedAuthMetricReason::CertHashMismatch,
        VerifyDelegatedTokenError::CertNotYetValid => DelegatedAuthMetricReason::CertNotYetValid,
        VerifyDelegatedTokenError::CertRules(_) => DelegatedAuthMetricReason::CertPolicy,
        VerifyDelegatedTokenError::IssuerShardPidMismatch => {
            DelegatedAuthMetricReason::IssuerShardPidMismatch
        }
        VerifyDelegatedTokenError::GrantsNotSubset => DelegatedAuthMetricReason::GrantsNotSubset,
        VerifyDelegatedTokenError::MissingLocalRole => DelegatedAuthMetricReason::MissingLocalRole,
        VerifyDelegatedTokenError::RootSignatureInvalid(_) => {
            DelegatedAuthMetricReason::RootSignatureInvalid
        }
        VerifyDelegatedTokenError::ScopeRejected { .. } => DelegatedAuthMetricReason::ScopeRejected,
        VerifyDelegatedTokenError::ShardSignatureInvalid(_) => {
            DelegatedAuthMetricReason::ShardSignatureInvalid
        }
        VerifyDelegatedTokenError::ShardSignatureUnavailable => {
            DelegatedAuthMetricReason::ShardSignatureUnavailable
        }
        VerifyDelegatedTokenError::TokenAudienceRejected => {
            DelegatedAuthMetricReason::TokenAudienceRejected
        }
        VerifyDelegatedTokenError::TokenExpired => DelegatedAuthMetricReason::TokenExpired,
        VerifyDelegatedTokenError::TokenGrantRejected => {
            DelegatedAuthMetricReason::TokenGrantRejected
        }
        VerifyDelegatedTokenError::TokenInvalidWindow => {
            DelegatedAuthMetricReason::TokenInvalidWindow
        }
        VerifyDelegatedTokenError::TokenIssuedBeforeCert => {
            DelegatedAuthMetricReason::TokenIssuedBeforeCert
        }
        VerifyDelegatedTokenError::TokenNotYetValid => DelegatedAuthMetricReason::TokenNotYetValid,
        VerifyDelegatedTokenError::TokenOutlivesCert => {
            DelegatedAuthMetricReason::TokenOutlivesCert
        }
        VerifyDelegatedTokenError::TokenTtlExceeded { .. } => {
            DelegatedAuthMetricReason::TokenTtlExceeded
        }
    }
}
