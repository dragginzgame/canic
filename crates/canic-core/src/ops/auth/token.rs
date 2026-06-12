use super::{
    AuthOps, DelegatedTokenVerifierConfig, PreparedDelegatedTokenIssuerProof,
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
        cert_rules::DelegatedAuthTtlLimits,
        verify::{
            VerifiedDelegatedToken, VerifyDelegatedTokenError, VerifyDelegatedTokenInput,
            verify_delegated_token, verify_delegated_token_cached_proof_identity,
        },
    },
    issuer_canister_sig::IssuerPayloadKind,
    root_canister_sig::RootPayloadKind,
};
use crate::{
    InternalError,
    cdk::{types::Principal, utils::hash::decode_hex},
    config::schema::DelegatedTokenConfig,
    domain::auth::{
        DelegatedAuthNetwork, IC_ROOT_PUBLIC_KEY_RAW_LENGTH, is_mainnet_ic_root_public_key_raw,
    },
    dto::auth::DelegatedToken,
    ids::CanisterRole,
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
    static PENDING_DELEGATED_TOKENS: RefCell<BTreeMap<PendingDelegatedTokenKey, crate::ops::auth::delegated::mint::PreparedDelegatedToken>> =
        const { RefCell::new(BTreeMap::new()) };
}

impl AuthOps {
    /// Prepare delegated-token claims before issuer canister-signature retrieval.
    pub(crate) fn prepare_delegated_token_issuer_proof(
        input: SignDelegatedTokenInput,
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
        let active_proof = Self::active_delegation_proof(now_ns).ok_or_else(|| {
            AuthValidationError::Auth(
                "active delegation proof is unavailable or expired".to_string(),
            )
        })?;

        if active_proof.proof.cert.issuer_pid != local {
            return Err(AuthScopeError::IssuerPidMismatch {
                expected: local,
                found: active_proof.proof.cert.issuer_pid,
            }
            .into());
        }

        let prepared = prepare_delegated_token(MintDelegatedTokenInput {
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
        .map_err(map_mint_delegated_token_error)?;

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

        let verified = verify_with_embedded_proofs(&input, &ctx)?;
        insert_positive_verification_cache(&input, cache_key);
        DelegatedAuthMetrics::record_verify_completed();
        Ok(verified)
    }

    fn delegated_token_verifier_config_from(
        cfg: &DelegatedTokenConfig,
    ) -> Result<DelegatedTokenVerifierConfig, InternalError> {
        let network = configured_delegated_auth_network(cfg)?;
        Ok(DelegatedTokenVerifierConfig {
            network,
            root_canister_id: configured_root_canister_id(cfg)?,
            ic_root_public_key_raw: configured_ic_root_public_key_raw(cfg, network)?,
        })
    }
}

struct DelegatedTokenRuntimeContext {
    verifier_cfg: DelegatedTokenVerifierConfig,
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
        local_canister: IcOps::canister_self(),
        local_canic_subnet: EnvOps::subnet_pid().ok(),
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
    ctx: &'a DelegatedTokenRuntimeContext,
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
    ctx: &'a DelegatedTokenRuntimeContext,
) -> Result<VerifiedDelegatedToken, InternalError> {
    validate_network_root_key_pair(
        ctx.verifier_cfg.network,
        &ctx.verifier_cfg.ic_root_public_key_raw,
    )?;
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
        |claims_hash, issuer_proof, issuer_pid| {
            AuthOps::verify_issuer_canister_signature_proof(
                IssuerPayloadKind::DelegatedTokenClaims,
                claims_hash,
                issuer_proof,
                issuer_pid,
                &ctx.verifier_cfg.ic_root_public_key_raw,
            )
            .map_err(|err| err.to_string())
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

fn configured_ic_root_public_key_raw(
    cfg: &DelegatedTokenConfig,
    network: DelegatedAuthNetwork,
) -> Result<Vec<u8>, InternalError> {
    configured_ic_root_public_key_raw_with_provider(cfg, network, AuthOps::ic_root_public_key_raw)
}

fn configured_ic_root_public_key_raw_with_provider<F>(
    cfg: &DelegatedTokenConfig,
    network: DelegatedAuthNetwork,
    runtime_root_key: F,
) -> Result<Vec<u8>, InternalError>
where
    F: FnOnce() -> Result<Vec<u8>, InternalError>,
{
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
        None if network.is_mainnet() => {
            return Err(AuthValidationError::Auth(
                "auth.delegated_tokens.ic_root_public_key_raw_hex is required when auth.delegated_tokens.network=\"mainnet\"".to_string(),
            )
            .into());
        }
        None => runtime_root_key()?,
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
    use crate::domain::auth::MAINNET_IC_ROOT_PUBLIC_KEY_RAW;
    use std::fmt::Write as _;

    fn root_pid() -> Principal {
        Principal::from_slice(&[1; 29])
    }

    fn cfg(network: &str, root_key: Option<Vec<u8>>) -> DelegatedTokenConfig {
        DelegatedTokenConfig {
            enabled: true,
            root_canister_id: Some(root_pid().to_string()),
            ic_root_public_key_raw_hex: root_key.map(hex),
            network: network.to_string(),
            max_ttl_secs: None,
        }
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

    fn supplied_key(key: Vec<u8>) -> impl FnOnce() -> Result<Vec<u8>, InternalError> {
        || Ok(key)
    }

    #[test]
    fn delegated_token_verifier_config_accepts_mainnet_with_known_mainnet_root_key() {
        let cfg = cfg("mainnet", Some(mainnet_key()));

        let verifier =
            AuthOps::delegated_token_verifier_config_from(&cfg).expect("mainnet key should pass");

        assert_eq!(verifier.network, DelegatedAuthNetwork::Mainnet);
        assert_eq!(verifier.root_canister_id, root_pid());
        assert_eq!(verifier.ic_root_public_key_raw, mainnet_key());
    }

    #[test]
    fn delegated_token_verifier_config_rejects_mainnet_without_root_key() {
        let cfg = cfg("mainnet", None);

        let err = AuthOps::delegated_token_verifier_config_from(&cfg)
            .expect_err("mainnet requires explicit root key");

        assert!(
            err.to_string()
                .contains("ic_root_public_key_raw_hex is required"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn delegated_token_verifier_config_rejects_mainnet_with_local_root_key() {
        let cfg = cfg("mainnet", Some(local_key()));

        let err = AuthOps::delegated_token_verifier_config_from(&cfg)
            .expect_err("mainnet must reject local root key");

        assert!(
            err.to_string()
                .contains("requires the known mainnet raw IC root public key"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn local_verifier_config_can_use_runtime_root_key_provider() {
        let cfg = cfg("local", None);

        let root_key = configured_ic_root_public_key_raw_with_provider(
            &cfg,
            DelegatedAuthNetwork::Local,
            supplied_key(local_key()),
        )
        .expect("local runtime provider key should pass");

        assert_eq!(root_key, local_key());
    }

    #[test]
    fn pocketic_verifier_config_rejects_runtime_mainnet_root_key() {
        let cfg = cfg("pocketic", None);

        let err = configured_ic_root_public_key_raw_with_provider(
            &cfg,
            DelegatedAuthNetwork::PocketIc,
            supplied_key(mainnet_key()),
        )
        .expect_err("pocketic must not accept mainnet root key");

        assert!(
            err.to_string()
                .contains("network=\"pocketic\" must not use the mainnet IC root public key"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn local_verifier_config_rejects_explicit_mainnet_root_key() {
        let cfg = cfg("local", Some(mainnet_key()));

        let err = AuthOps::delegated_token_verifier_config_from(&cfg)
            .expect_err("local must reject explicit mainnet root key");

        assert!(
            err.to_string()
                .contains("network=\"local\" must not use the mainnet IC root public key"),
            "unexpected error: {err}"
        );
    }
}
