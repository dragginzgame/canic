//! Module: ops::auth::token
//!
//! Responsibility: prepare issuer-local delegated tokens and verify delegated tokens.
//! Does not own: endpoint authorization, active-proof storage, or auth policy records.
//! Boundary: auth ops facade for delegated-token workflows, config, metrics, and proof helpers.

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
            ChainKeyRootVerifierPolicy, VerifyChainKeyBatchRootProofInput,
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
use std::{cell::RefCell, collections::BTreeMap};

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

        let verifier_cfg = auth_proof_verifier_config_for_verification(&cfg)?;
        let verified = verify_with_embedded_proofs(&input, &ctx, &verifier_cfg)?;
        insert_positive_verification_cache(&input, cache_key);
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

struct DelegatedTokenLocalContext {
    local_canister: Principal,
    local_canic_subnet: Option<Principal>,
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

fn require_current_canister_delegated_token_verifier() -> Result<(), InternalError> {
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

fn auth_proof_verifier_config_for_verification(
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

fn delegated_token_local_context() -> Result<DelegatedTokenLocalContext, InternalError> {
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

    Ok(DelegatedTokenLocalContext {
        local_canister: IcOps::canister_self(),
        local_canic_subnet: EnvOps::subnet_pid().ok(),
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
        local_canic_subnet: ctx.local_canic_subnet,
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

fn verify_with_embedded_proofs<'a>(
    input: &'a VerifyDelegatedTokenRuntimeInput<'a>,
    ctx: &'a DelegatedTokenLocalContext,
    verifier_cfg: &'a AuthProofVerifierConfig,
) -> Result<VerifiedDelegatedToken, InternalError> {
    validate_network_root_key_pair(verifier_cfg.network, &verifier_cfg.ic_root_public_key_raw)?;
    verify_delegated_token(
        delegated_token_verify_input(input, ctx),
        |cert, cert_hash, root_proof| {
            AuthOps::verify_delegation_root_proof(
                cert,
                cert_hash,
                root_proof,
                verifier_cfg,
                input.now_ns,
            )
        },
        |claims_hash, issuer_proof, issuer_pid| {
            AuthOps::verify_issuer_canister_signature_proof(
                IssuerPayloadKind::DelegatedTokenClaims,
                claims_hash,
                issuer_proof,
                issuer_pid,
                &verifier_cfg.ic_root_public_key_raw,
            )
            .map_err(|err| err.to_string())
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
        _cert_hash: [u8; 32],
        root_proof: &RootProof,
        verifier_cfg: &AuthProofVerifierConfig,
        now_ns: u64,
    ) -> Result<(), String> {
        let root_pid = cert.root_pid;
        if root_pid != verifier_cfg.root_canister_id {
            return Err(AuthValidationError::InvalidRootAuthority {
                expected: verifier_cfg.root_canister_id,
                found: root_pid,
            }
            .to_string());
        }

        if verifier_cfg.root_proof_mode != RootProofMode::ChainKeyBatch {
            return Err("0.76 delegated auth requires chain_key_batch root proofs".to_string());
        }
        let Some(chain_key_root) = verifier_cfg.chain_key_root.as_ref() else {
            return Err("chain-key root verifier policy is not configured".to_string());
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
        .map_err(|err| err.to_string())
    }
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

fn configured_delegated_auth_network(
    cfg: &DelegatedTokenConfig,
) -> Result<DelegatedAuthNetwork, InternalError> {
    DelegatedAuthNetwork::parse(cfg.network.trim()).ok_or_else(|| {
        AuthValidationError::Auth(
            "auth.delegated_tokens.network must be one of mainnet, local, pocketic, testnet"
                .to_string(),
        )
        .into()
    })
}

fn configured_root_proof_mode(cfg: &DelegatedTokenConfig) -> Result<RootProofMode, InternalError> {
    match cfg.root_proof_mode.trim() {
        "chain_key_batch" => Ok(RootProofMode::ChainKeyBatch),
        _ => Err(AuthValidationError::Auth(
            "auth.delegated_tokens.root_proof_mode must be chain_key_batch in 0.76".to_string(),
        )
        .into()),
    }
}

fn configured_chain_key_root_verifier(
    cfg: &DelegatedTokenConfig,
    root_canister_id: Principal,
    network: DelegatedAuthNetwork,
) -> Result<Option<AuthChainKeyRootVerifierConfig>, InternalError> {
    let chain_key = &cfg.chain_key_root_proof;
    let key_id = required_chain_key_field(chain_key.key_id.as_deref(), "key_id")?;
    let derivation_path_hash = required_fixed_32_chain_key_hex(
        chain_key.derivation_path_hash_hex.as_deref(),
        "derivation_path_hash_hex",
    )?;
    let public_key_hex =
        required_chain_key_field(chain_key.public_key_hex.as_deref(), "public_key_hex")?;
    let public_key = decode_hex(public_key_hex).map_err(|err| {
        AuthValidationError::Auth(format!(
            "auth.delegated_tokens.chain_key_root_proof.public_key_hex is not valid hex: {err}"
        ))
    })?;
    verify_chain_key_ecdsa_public_key_shape(&public_key).map_err(|err| {
        AuthValidationError::Auth(format!(
            "auth.delegated_tokens.chain_key_root_proof.public_key_hex must be a secp256k1 SEC1 public key: {err}"
        ))
    })?;
    let key_version = required_chain_key_u64(chain_key.key_version, "key_version")?;
    let min_accepted_key_version = required_chain_key_u64(
        chain_key.min_accepted_key_version,
        "min_accepted_key_version",
    )?;
    let min_accepted_proof_epoch = required_chain_key_u64(
        chain_key.min_accepted_proof_epoch,
        "min_accepted_proof_epoch",
    )?;
    let min_accepted_registry_epoch = required_chain_key_u64(
        chain_key.min_accepted_registry_epoch,
        "min_accepted_registry_epoch",
    )?;
    let valid_from_ns = required_chain_key_u64(chain_key.valid_from_ns, "valid_from_ns")?;
    let accept_until_ns = required_chain_key_u64(chain_key.accept_until_ns, "accept_until_ns")?;
    let max_revocation_latency_ns = required_chain_key_u64(
        chain_key.max_revocation_latency_ns,
        "max_revocation_latency_ns",
    )?;
    if valid_from_ns >= accept_until_ns {
        return Err(AuthValidationError::Auth(
            "auth.delegated_tokens.chain_key_root_proof.valid_from_ns must be before accept_until_ns"
                .to_string(),
        )
        .into());
    }
    if max_revocation_latency_ns == 0 {
        return Err(AuthValidationError::Auth(
            "auth.delegated_tokens.chain_key_root_proof.max_revocation_latency_ns must be greater than zero"
                .to_string(),
        )
        .into());
    }
    if network.is_mainnet() && key_id == "test_key_1" {
        return Err(AuthValidationError::Auth(
            "auth.delegated_tokens.chain_key_root_proof.key_id must not be test_key_1 on network=\"mainnet\""
                .to_string(),
        )
        .into());
    }
    if !network.is_mainnet() && key_id == "test_key_1" && !chain_key.allow_test_key {
        return Err(AuthValidationError::Auth(
            "auth.delegated_tokens.chain_key_root_proof.allow_test_key must be true to use test_key_1 outside mainnet"
                .to_string(),
        )
        .into());
    }

    Ok(Some(AuthChainKeyRootVerifierConfig {
        policy: RootKeyPolicyV1 {
            root_canister_id,
            proof_mode: RootProofMode::ChainKeyBatch,
            algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
            key_id: ChainKeyKeyId {
                name: key_id.to_string(),
            },
            derivation_path_hash,
            public_key,
            key_version,
            min_accepted_key_version,
            min_accepted_proof_epoch,
            min_accepted_registry_epoch,
            max_revocation_latency_ns,
            valid_from_ns,
            accept_until_ns,
            build_network: build_network_for_delegated_auth(network),
        },
        allow_test_chain_key: chain_key.allow_test_key,
    }))
}

fn required_chain_key_field<'a>(
    value: Option<&'a str>,
    field: &'static str,
) -> Result<&'a str, InternalError> {
    let Some(value) = value else {
        return Err(AuthValidationError::Auth(format!(
            "auth.delegated_tokens.chain_key_root_proof.{field} is required when root_proof_mode=\"chain_key_batch\""
        ))
        .into());
    };
    let value = value.trim();
    if value.is_empty() {
        return Err(AuthValidationError::Auth(format!(
            "auth.delegated_tokens.chain_key_root_proof.{field} must not be empty"
        ))
        .into());
    }
    Ok(value)
}

fn required_chain_key_u64(value: Option<u64>, field: &'static str) -> Result<u64, InternalError> {
    value.ok_or_else(|| {
        AuthValidationError::Auth(format!(
            "auth.delegated_tokens.chain_key_root_proof.{field} is required when root_proof_mode=\"chain_key_batch\""
        ))
        .into()
    })
}

fn required_fixed_32_chain_key_hex(
    value: Option<&str>,
    field: &'static str,
) -> Result<[u8; 32], InternalError> {
    let value = required_chain_key_field(value, field)?;
    let decoded = decode_hex(value).map_err(|err| {
        AuthValidationError::Auth(format!(
            "auth.delegated_tokens.chain_key_root_proof.{field} is not valid hex: {err}"
        ))
    })?;
    decoded.try_into().map_err(|decoded: Vec<u8>| {
        AuthValidationError::Auth(format!(
            "auth.delegated_tokens.chain_key_root_proof.{field} must decode to 32 bytes, got {}",
            decoded.len()
        ))
        .into()
    })
}

const fn build_network_for_delegated_auth(network: DelegatedAuthNetwork) -> BuildNetwork {
    if network.is_mainnet() {
        BuildNetwork::Ic
    } else {
        BuildNetwork::Local
    }
}

fn chain_key_policy_from_config(
    config: &AuthChainKeyRootVerifierConfig,
) -> ChainKeyRootVerifierPolicy {
    ChainKeyRootVerifierPolicy {
        root_canister_id: config.policy.root_canister_id,
        algorithm: config.policy.algorithm,
        key_id: config.policy.key_id.clone(),
        derivation_path_hash: config.policy.derivation_path_hash,
        public_key: config.policy.public_key.clone(),
        key_version: config.policy.key_version,
        min_accepted_key_version: config.policy.min_accepted_key_version,
        min_accepted_proof_epoch: config.policy.min_accepted_proof_epoch,
        min_accepted_registry_epoch: config.policy.min_accepted_registry_epoch,
        valid_from_ns: config.policy.valid_from_ns,
        accept_until_ns: config.policy.accept_until_ns,
        build_network: config.policy.build_network,
        allow_test_chain_key: config.allow_test_chain_key,
        max_revocation_latency_ns: config.policy.max_revocation_latency_ns,
    }
}

fn configured_ic_root_public_key_raw(
    cfg: &DelegatedTokenConfig,
    network: DelegatedAuthNetwork,
) -> Result<Vec<u8>, InternalError> {
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
        None => {
            return Err(AuthValidationError::Auth(format!(
                "auth.delegated_tokens.ic_root_public_key_raw_hex is required when delegated-token verification is enabled for network=\"{}\"",
                network.label()
            ))
            .into());
        }
    };

    if ic_root_public_key_raw.len() != IC_ROOT_PUBLIC_KEY_RAW_LENGTH {
        return Err(AuthValidationError::Auth(format!(
            "auth.delegated_tokens.ic_root_public_key_raw must be {IC_ROOT_PUBLIC_KEY_RAW_LENGTH} raw bytes"
        ))
        .into());
    }

    validate_network_root_key_pair(network, &ic_root_public_key_raw)?;
    Ok(ic_root_public_key_raw)
}

fn validate_network_root_key_pair(
    network: DelegatedAuthNetwork,
    ic_root_public_key_raw: &[u8],
) -> Result<(), InternalError> {
    let is_mainnet_key = is_mainnet_ic_root_public_key_raw(ic_root_public_key_raw);
    if network.is_mainnet() {
        if is_mainnet_key {
            return Ok(());
        }
        return Err(AuthValidationError::Auth(
            "auth.delegated_tokens.network=\"mainnet\" requires the known mainnet raw IC root public key".to_string(),
        )
        .into());
    }

    if is_mainnet_key {
        return Err(AuthValidationError::Auth(format!(
            "auth.delegated_tokens.network=\"{}\" must not use the mainnet IC root public key",
            network.label()
        ))
        .into());
    }

    Ok(())
}

fn active_delegation_proof_unavailable_error(now_ns: u64) -> InternalError {
    let status = AuthOps::active_delegation_proof_status(now_ns).status;
    match status {
        ActiveDelegationProofStatus::Expired => InternalError::auth_proof_expired(
            "active delegation proof expired; reprovision auth proof",
        ),
        ActiveDelegationProofStatus::Missing => InternalError::auth_material_stale(
            "active delegation proof is unavailable; provision auth proof",
        ),
        ActiveDelegationProofStatus::RefreshNeeded | ActiveDelegationProofStatus::Valid => {
            InternalError::auth_material_stale(
                "active delegation proof is unavailable or stale; reprovision auth proof",
            )
        }
    }
}

fn map_prepare_delegated_token_error(err: PrepareDelegatedTokenError) -> InternalError {
    match err {
        PrepareDelegatedTokenError::CertExpired => InternalError::auth_proof_expired(
            "active delegation proof expired; reprovision auth proof",
        ),
        PrepareDelegatedTokenError::TokenOutlivesCert => InternalError::auth_material_stale(
            "active delegation proof is too close to expiry; reprovision auth proof",
        ),
        err => AuthValidationError::Auth(err.to_string()).into(),
    }
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
        VerifyDelegatedTokenError::GrantsNotSubset => DelegatedAuthMetricReason::GrantsNotSubset,
        VerifyDelegatedTokenError::IssuerProofInvalid(_) => {
            DelegatedAuthMetricReason::IssuerProofInvalid
        }
        VerifyDelegatedTokenError::IssuerProofUnavailable => {
            DelegatedAuthMetricReason::IssuerProofUnavailable
        }
        VerifyDelegatedTokenError::IssuerPidMismatch => {
            DelegatedAuthMetricReason::IssuerPidMismatch
        }
        VerifyDelegatedTokenError::MissingLocalRole => DelegatedAuthMetricReason::MissingLocalRole,
        VerifyDelegatedTokenError::RootProofInvalid(_) => {
            DelegatedAuthMetricReason::RootProofInvalid
        }
        VerifyDelegatedTokenError::ScopeRejected { .. } => DelegatedAuthMetricReason::ScopeRejected,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{
            Config,
            schema::{CanisterAuthConfig, CanisterKind, ChainKeyRootProofConfig},
        },
        domain::auth::MAINNET_IC_ROOT_PUBLIC_KEY_RAW,
        dto::error::ErrorCode,
        ids::SubnetRole,
        ops::auth::delegated::chain_key::ChainKeySignatureVerificationInput,
        storage::stable::env::{Env, EnvRecord},
        test::config::ConfigTestBuilder,
    };
    use k256::ecdsa::{
        Signature as K256TestSignature, SigningKey as K256SigningKey,
        signature::hazmat::PrehashSigner,
    };
    use std::fmt::Write as _;

    fn root_pid() -> Principal {
        Principal::from_slice(&[1; 29])
    }

    fn cfg(network: &str, root_key: Option<Vec<u8>>) -> DelegatedTokenConfig {
        let mut cfg = DelegatedTokenConfig {
            enabled: true,
            root_canister_id: Some(root_pid().to_string()),
            ic_root_public_key_raw_hex: root_key.map(hex),
            root_proof_mode: "chain_key_batch".to_string(),
            chain_key_root_proof: ChainKeyRootProofConfig::default(),
            network: network.to_string(),
            max_ttl_secs: None,
        };
        install_chain_key_policy(&mut cfg, "key_1");
        cfg
    }

    fn chain_key_cfg(network: &str, root_key: Vec<u8>, key_id: &str) -> DelegatedTokenConfig {
        let mut cfg = cfg(network, Some(root_key));
        install_chain_key_policy(&mut cfg, key_id);
        cfg
    }

    fn install_chain_key_policy(cfg: &mut DelegatedTokenConfig, key_id: &str) {
        cfg.root_proof_mode = "chain_key_batch".to_string();
        cfg.chain_key_root_proof.key_id = Some(key_id.to_string());
        cfg.chain_key_root_proof.derivation_path_hash_hex =
            Some("fe51a87b988d221227b134c48f36787e891a902dcb5d48ea5f94cff8bfed5a16".to_string());
        cfg.chain_key_root_proof.derivation_path_hex = Some(vec![
            "63616e6963".to_string(),
            "64656c65676174696f6e".to_string(),
        ]);
        cfg.chain_key_root_proof.public_key_hex = Some("02".repeat(33));
        cfg.chain_key_root_proof.key_version = Some(4);
        cfg.chain_key_root_proof.min_accepted_key_version = Some(4);
        cfg.chain_key_root_proof.min_accepted_proof_epoch = Some(7);
        cfg.chain_key_root_proof.min_accepted_registry_epoch = Some(8);
        cfg.chain_key_root_proof.valid_from_ns = Some(10);
        cfg.chain_key_root_proof.accept_until_ns = Some(1_000);
        cfg.chain_key_root_proof.max_revocation_latency_ns = Some(600);
    }

    fn hex(bytes: Vec<u8>) -> String {
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            write!(&mut out, "{byte:02x}").expect("hex write should not fail");
        }
        out
    }

    fn local_key() -> Vec<u8> {
        vec![7; IC_ROOT_PUBLIC_KEY_RAW_LENGTH]
    }

    fn mainnet_key() -> Vec<u8> {
        MAINNET_IC_ROOT_PUBLIC_KEY_RAW.to_vec()
    }

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn chain_key_ecdsa_signature_verifier_accepts_valid_prehash_signature() {
        let signing_key =
            K256SigningKey::from_slice(&[7; 32]).expect("test signing key should parse");
        let public_key = signing_key.verifying_key().to_encoded_point(true);
        let message_hash = [42; 32];
        let signature: K256TestSignature = signing_key
            .sign_prehash(&message_hash)
            .expect("test prehash signature should sign");
        let signature_bytes = signature.to_bytes();
        let key_id = ChainKeyKeyId {
            name: "key_1".to_string(),
        };
        let derivation_path = Vec::new();

        verify_chain_key_ecdsa_signature(ChainKeySignatureVerificationInput {
            algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
            key_id: &key_id,
            derivation_path: &derivation_path,
            public_key: public_key.as_bytes(),
            message_hash,
            signature: signature_bytes.as_ref(),
        })
        .expect("valid chain-key ECDSA prehash signature should verify");
    }

    #[test]
    fn chain_key_ecdsa_signature_verifier_rejects_altered_signature() {
        let signing_key =
            K256SigningKey::from_slice(&[7; 32]).expect("test signing key should parse");
        let public_key = signing_key.verifying_key().to_encoded_point(true);
        let message_hash = [42; 32];
        let signature: K256TestSignature = signing_key
            .sign_prehash(&message_hash)
            .expect("test prehash signature should sign");
        let mut signature_bytes = signature.to_bytes().to_vec();
        signature_bytes[0] ^= 1;
        let key_id = ChainKeyKeyId {
            name: "key_1".to_string(),
        };
        let derivation_path = Vec::new();

        let err = verify_chain_key_ecdsa_signature(ChainKeySignatureVerificationInput {
            algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
            key_id: &key_id,
            derivation_path: &derivation_path,
            public_key: public_key.as_bytes(),
            message_hash,
            signature: &signature_bytes,
        })
        .expect_err("altered chain-key ECDSA signature must reject");

        assert!(
            err.contains("signature verification failed"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn active_delegation_proof_unavailable_maps_to_auth_material_stale() {
        crate::ops::storage::auth::AuthStateOps::clear_active_delegation_proof();

        let err = active_delegation_proof_unavailable_error(20);
        let public = err
            .public_error()
            .expect("missing active proof must be public");

        assert_eq!(public.code, ErrorCode::AuthMaterialStale);
        assert!(public.message.contains("provision auth proof"));
    }

    #[test]
    fn token_prepare_outliving_active_proof_maps_to_auth_material_stale() {
        let err = map_prepare_delegated_token_error(PrepareDelegatedTokenError::TokenOutlivesCert);
        let public = err
            .public_error()
            .expect("stale active proof must be public");

        assert_eq!(public.code, ErrorCode::AuthMaterialStale);
        assert!(public.message.contains("too close to expiry"));
    }

    #[test]
    fn token_prepare_expired_active_proof_maps_to_auth_proof_expired() {
        let err = map_prepare_delegated_token_error(PrepareDelegatedTokenError::CertExpired);
        let public = err
            .public_error()
            .expect("expired active proof must be public");

        assert_eq!(public.code, ErrorCode::AuthProofExpired);
        assert!(public.message.contains("expired"));
    }

    #[test]
    fn auth_proof_verifier_config_accepts_mainnet_with_known_mainnet_root_key() {
        let cfg = cfg("mainnet", Some(mainnet_key()));

        let verifier =
            AuthOps::auth_proof_verifier_config_from(&cfg).expect("mainnet key should pass");

        assert_eq!(verifier.network, DelegatedAuthNetwork::Mainnet);
        assert_eq!(verifier.root_canister_id, root_pid());
        assert_eq!(verifier.ic_root_public_key_raw, mainnet_key());
        assert_eq!(verifier.root_proof_mode, RootProofMode::ChainKeyBatch);
        assert!(verifier.chain_key_root.is_some());
    }

    #[test]
    fn auth_proof_verifier_config_rejects_non_chain_key_root_proof_mode() {
        let mut cfg = cfg("mainnet", Some(mainnet_key()));
        cfg.root_proof_mode = "canister_signature".to_string();

        let err = AuthOps::auth_proof_verifier_config_from(&cfg)
            .expect_err("0.76 must reject non-chain-key root proof mode");

        assert!(
            err.to_string().contains("must be chain_key_batch"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn auth_proof_verifier_config_rejects_mainnet_without_root_key() {
        let cfg = cfg("mainnet", None);

        let err = AuthOps::auth_proof_verifier_config_from(&cfg)
            .expect_err("mainnet requires explicit root key");

        assert!(
            err.to_string()
                .contains("ic_root_public_key_raw_hex is required"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn auth_proof_verifier_config_rejects_mainnet_with_local_root_key() {
        let cfg = cfg("mainnet", Some(local_key()));

        let err = AuthOps::auth_proof_verifier_config_from(&cfg)
            .expect_err("mainnet must reject local root key");

        assert!(
            err.to_string()
                .contains("requires the known mainnet raw IC root public key"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn auth_proof_verifier_config_local_requires_explicit_root_key() {
        let cfg = cfg("local", None);

        let err = AuthOps::auth_proof_verifier_config_from(&cfg)
            .expect_err("local verifier requires explicit root key");

        assert!(
            err.to_string()
                .contains("ic_root_public_key_raw_hex is required"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn auth_proof_verifier_config_local_accepts_explicit_local_root_key() {
        let cfg = cfg("local", Some(local_key()));

        let verifier =
            AuthOps::auth_proof_verifier_config_from(&cfg).expect("local key should pass");

        assert_eq!(verifier.network, DelegatedAuthNetwork::Local);
        assert_eq!(verifier.ic_root_public_key_raw, local_key());
    }

    #[test]
    fn auth_proof_verifier_config_pocketic_requires_explicit_root_key() {
        let cfg = cfg("pocketic", None);

        let err = AuthOps::auth_proof_verifier_config_from(&cfg)
            .expect_err("pocketic verifier requires explicit root key");

        assert!(
            err.to_string()
                .contains("ic_root_public_key_raw_hex is required"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn auth_proof_verifier_config_pocketic_rejects_explicit_mainnet_root_key() {
        let cfg = cfg("pocketic", Some(mainnet_key()));

        let err = AuthOps::auth_proof_verifier_config_from(&cfg)
            .expect_err("pocketic must not accept mainnet root key");

        assert!(
            err.to_string()
                .contains("network=\"pocketic\" must not use the mainnet IC root public key"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn auth_proof_verifier_config_pocketic_accepts_explicit_pocketic_root_key() {
        let cfg = cfg("pocketic", Some(local_key()));

        let verifier =
            AuthOps::auth_proof_verifier_config_from(&cfg).expect("pocketic key should pass");

        assert_eq!(verifier.network, DelegatedAuthNetwork::PocketIc);
        assert_eq!(verifier.ic_root_public_key_raw, local_key());
    }

    #[test]
    fn auth_proof_verifier_config_local_rejects_explicit_mainnet_root_key() {
        let cfg = cfg("local", Some(mainnet_key()));

        let err = AuthOps::auth_proof_verifier_config_from(&cfg)
            .expect_err("local must reject explicit mainnet root key");

        assert!(
            err.to_string()
                .contains("network=\"local\" must not use the mainnet IC root public key"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn auth_proof_verifier_config_testnet_requires_explicit_root_key() {
        let cfg = cfg("testnet", None);

        let err = AuthOps::auth_proof_verifier_config_from(&cfg)
            .expect_err("testnet verifier requires explicit root key");

        assert!(
            err.to_string()
                .contains("ic_root_public_key_raw_hex is required"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn auth_proof_verifier_config_testnet_accepts_explicit_test_root_key() {
        let cfg = cfg("testnet", Some(local_key()));

        let verifier =
            AuthOps::auth_proof_verifier_config_from(&cfg).expect("testnet key should pass");

        assert_eq!(verifier.network, DelegatedAuthNetwork::Testnet);
        assert_eq!(verifier.ic_root_public_key_raw, local_key());
    }

    #[test]
    fn auth_proof_verifier_config_chain_key_local_accepts_test_key_when_allowed() {
        let mut cfg = chain_key_cfg("local", local_key(), "test_key_1");
        cfg.chain_key_root_proof.allow_test_key = true;

        let verifier =
            AuthOps::auth_proof_verifier_config_from(&cfg).expect("chain-key config should pass");
        let chain_key_root = verifier
            .chain_key_root
            .as_ref()
            .expect("chain-key policy should be configured");

        assert_eq!(verifier.root_proof_mode, RootProofMode::ChainKeyBatch);
        assert_eq!(chain_key_root.policy.root_canister_id, root_pid());
        assert_eq!(chain_key_root.policy.key_id.name, "test_key_1");
        assert_eq!(
            chain_key_root.policy.derivation_path_hash,
            [
                0xfe, 0x51, 0xa8, 0x7b, 0x98, 0x8d, 0x22, 0x12, 0x27, 0xb1, 0x34, 0xc4, 0x8f, 0x36,
                0x78, 0x7e, 0x89, 0x1a, 0x90, 0x2d, 0xcb, 0x5d, 0x48, 0xea, 0x5f, 0x94, 0xcf, 0xf8,
                0xbf, 0xed, 0x5a, 0x16,
            ]
        );
        assert_eq!(chain_key_root.policy.public_key, vec![0x02; 33]);
        assert_eq!(chain_key_root.policy.max_revocation_latency_ns, 600);
        assert_eq!(chain_key_root.policy.build_network, BuildNetwork::Local);
        assert!(chain_key_root.allow_test_chain_key);
    }

    #[test]
    fn auth_proof_verifier_config_chain_key_rejects_invalid_public_key() {
        let mut cfg = chain_key_cfg("local", local_key(), "test_key_1");
        cfg.chain_key_root_proof.allow_test_key = true;
        cfg.chain_key_root_proof.public_key_hex = Some("00".repeat(33));

        let err = AuthOps::auth_proof_verifier_config_from(&cfg)
            .expect_err("invalid chain-key public key must reject");

        assert!(
            err.to_string()
                .contains("must be a secp256k1 SEC1 public key"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn auth_proof_verifier_config_chain_key_rejects_mainnet_test_key() {
        let cfg = chain_key_cfg("mainnet", mainnet_key(), "test_key_1");

        let err = AuthOps::auth_proof_verifier_config_from(&cfg)
            .expect_err("mainnet must reject test_key_1");

        assert!(
            err.to_string().contains("must not be test_key_1"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn auth_proof_verifier_config_chain_key_rejects_unapproved_local_test_key() {
        let cfg = chain_key_cfg("local", local_key(), "test_key_1");

        let err = AuthOps::auth_proof_verifier_config_from(&cfg)
            .expect_err("local test key requires explicit opt-in");

        assert!(
            err.to_string().contains("allow_test_key must be true"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn delegated_token_verifier_gate_rejects_issuer_only_canister() {
        install_verifier_test_config(false, true, false);

        let err = require_current_canister_delegated_token_verifier()
            .expect_err("issuer-only canister must not verify delegated tokens");

        assert!(
            err.to_string()
                .contains("delegated token verifier disabled for this canister"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn delegated_token_verifier_gate_rejects_role_attestation_cache_without_verifier_flag() {
        install_verifier_test_config(false, false, true);

        let err = require_current_canister_delegated_token_verifier()
            .expect_err("role-attestation cache must not enable delegated-token verification");

        assert!(
            err.to_string()
                .contains("delegated token verifier disabled for this canister"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn delegated_token_verifier_gate_accepts_current_canister_verifier() {
        install_verifier_test_config(true, false, false);

        require_current_canister_delegated_token_verifier()
            .expect("explicit verifier canister should pass the execution gate");
    }

    fn install_verifier_test_config(
        delegated_token_verifier: bool,
        delegated_token_issuer: bool,
        role_attestation_cache: bool,
    ) {
        let mut canister_cfg = ConfigTestBuilder::canister_config(CanisterKind::Service);
        canister_cfg.auth = CanisterAuthConfig {
            delegated_token_issuer,
            delegated_token_verifier,
            role_attestation_cache,
        };

        let mut cfg = ConfigTestBuilder::new()
            .with_prime_canister("project_instance", canister_cfg)
            .build();
        cfg.auth.delegated_tokens.network = "local".to_string();
        cfg.auth.delegated_tokens.root_proof_mode = "chain_key_batch".to_string();
        cfg.auth.delegated_tokens.root_canister_id = Some(root_pid().to_string());
        cfg.auth.delegated_tokens.ic_root_public_key_raw_hex = Some(hex(local_key()));
        cfg.auth.delegated_tokens.chain_key_root_proof.key_id = Some("key_1".to_string());
        cfg.auth
            .delegated_tokens
            .chain_key_root_proof
            .derivation_path_hash_hex =
            Some("fe51a87b988d221227b134c48f36787e891a902dcb5d48ea5f94cff8bfed5a16".to_string());
        cfg.auth
            .delegated_tokens
            .chain_key_root_proof
            .derivation_path_hex = Some(vec![
            "63616e6963".to_string(),
            "64656c65676174696f6e".to_string(),
        ]);
        cfg.auth
            .delegated_tokens
            .chain_key_root_proof
            .public_key_hex = Some("02".repeat(33));
        cfg.auth.delegated_tokens.chain_key_root_proof.key_version = Some(4);
        cfg.auth
            .delegated_tokens
            .chain_key_root_proof
            .min_accepted_key_version = Some(4);
        cfg.auth
            .delegated_tokens
            .chain_key_root_proof
            .min_accepted_proof_epoch = Some(7);
        cfg.auth
            .delegated_tokens
            .chain_key_root_proof
            .min_accepted_registry_epoch = Some(8);
        cfg.auth.delegated_tokens.chain_key_root_proof.valid_from_ns = Some(10);
        cfg.auth
            .delegated_tokens
            .chain_key_root_proof
            .accept_until_ns = Some(1_000);
        cfg.auth
            .delegated_tokens
            .chain_key_root_proof
            .max_revocation_latency_ns = Some(600);
        Config::reset_for_tests();
        Config::init_from_model_for_tests(cfg).expect("test config should install");

        Env::import(EnvRecord {
            prime_root_pid: Some(root_pid()),
            subnet_role: Some(SubnetRole::PRIME),
            subnet_pid: Some(p(9)),
            root_pid: Some(root_pid()),
            canister_role: Some(CanisterRole::new("project_instance")),
            parent_pid: Some(root_pid()),
        });
    }
}
