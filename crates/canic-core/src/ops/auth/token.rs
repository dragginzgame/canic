use super::{
    AuthOps, PreparedDelegatedTokenSignature, SignDelegatedTokenInput,
    VerifyDelegatedTokenRuntimeInput,
    delegated::mint::{
        MintDelegatedTokenError, MintDelegatedTokenInput, finish_delegated_token,
        prepare_delegated_token,
    },
    delegated::{
        canonical::{derivation_path_hash, key_name_hash, public_key_hash},
        cert_rules::DelegatedAuthTtlLimits,
        verify::{
            VerifiedDelegatedToken, VerifyDelegatedTokenError, VerifyDelegatedTokenInput,
            verify_delegated_token,
        },
    },
    keys,
};
use crate::{
    InternalError,
    dto::auth::{DelegatedToken, RootPublicKey, RootTrustAnchor, ShardKeyBinding},
    ops::{
        auth::{AuthScopeError, AuthSignatureError, AuthValidationError},
        config::ConfigOps,
        cost_guard::CostGuardPermit,
        ic::{IcOps, ecdsa::EcdsaOps},
        replay::model::{EcdsaPurpose, ExternalEffectDescriptor},
        runtime::{
            env::EnvOps,
            metrics::delegated_auth::{DelegatedAuthMetricReason, DelegatedAuthMetrics},
        },
        storage::state::subnet::SubnetStateOps,
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
            ttl_secs: input.ttl_secs,
            nonce: input.nonce,
            now_secs: IcOps::now_secs(),
        })
        .map_err(map_mint_delegated_token_error)?;

        let key_name = keys::delegated_tokens_key_name()?;
        let derivation_path = keys::shard_derivation_path(local);
        Ok(PreparedDelegatedTokenSignature {
            message_hash: prepared.claims_hash,
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
        let shard_sig =
            EcdsaOps::sign_bytes(permit, &key_name, derivation_path, message_hash).await?;

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

    /// Verify a self-contained delegated token without local proof lookup.
    pub fn verify_token(
        input: VerifyDelegatedTokenRuntimeInput<'_>,
    ) -> Result<VerifiedDelegatedToken, InternalError> {
        DelegatedAuthMetrics::record_verify_started();

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

        if let Err(err) = Self::verify_shard_key_binding(input.token) {
            DelegatedAuthMetrics::record_verify_failed(DelegatedAuthMetricReason::ShardKeyBinding);
            return Err(err);
        }
        let root_trust = match Self::root_trust_anchor(input.token, input.now_secs) {
            Ok(root_trust) => root_trust,
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

        let verified = verify_delegated_token(
            VerifyDelegatedTokenInput {
                token: input.token,
                root_trust: &root_trust,
                local_role: Some(&local_role),
                local_project: local_project.as_deref(),
                ttl_limits: DelegatedAuthTtlLimits {
                    max_cert_ttl_secs: input.max_cert_ttl_secs,
                    max_token_ttl_secs: input.max_token_ttl_secs,
                },
                required_scopes: input.required_scopes,
                now_secs: input.now_secs,
            },
            |public_key, hash, sig, _alg| {
                EcdsaOps::verify_signature(public_key, hash, sig).map_err(|err| err.to_string())
            },
        )
        .map_err(|err| {
            DelegatedAuthMetrics::record_verify_failed(delegated_auth_reason_from_verify_error(
                &err,
            ));
            map_verify_delegated_token_error(err)
        })?;

        DelegatedAuthMetrics::record_verify_completed();
        Ok(verified)
    }

    fn root_trust_anchor(
        token: &DelegatedToken,
        now_secs: u64,
    ) -> Result<RootTrustAnchor, InternalError> {
        let cert = &token.proof.cert;
        let key_name = keys::delegated_tokens_key_name()?;
        if cert.root_key_id != key_name {
            return Err(AuthValidationError::Auth(format!(
                "delegated auth root key id mismatch (expected {key_name}, found {})",
                cert.root_key_id
            ))
            .into());
        }

        let root_public_key = SubnetStateOps::delegated_root_public_key(&key_name)
            .ok_or(AuthSignatureError::RootPublicKeyUnavailable)?;
        let key_hash = public_key_hash(&root_public_key);

        Ok(RootTrustAnchor {
            root_pid: EnvOps::root_pid()?,
            root_key: RootPublicKey {
                root_pid: cert.root_pid,
                key_id: cert.root_key_id.clone(),
                alg: cert.alg,
                public_key_sec1: root_public_key,
                key_hash,
                not_before: cert.issued_at.min(now_secs),
                not_after: None,
            },
        })
    }

    fn verify_shard_key_binding(token: &DelegatedToken) -> Result<(), InternalError> {
        let cert = &token.proof.cert;
        let key_name = keys::delegated_tokens_key_name()?;
        let expected_derivation_path_hash =
            derivation_path_hash(&keys::shard_derivation_path(cert.shard_pid));
        match cert.shard_key_binding {
            ShardKeyBinding::IcThresholdEcdsa {
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
        VerifyDelegatedTokenError::RootKey(_) => DelegatedAuthMetricReason::RootKey,
        VerifyDelegatedTokenError::RootSignatureInvalid(_) => {
            DelegatedAuthMetricReason::RootSignatureInvalid
        }
        VerifyDelegatedTokenError::RootSignatureUnavailable => {
            DelegatedAuthMetricReason::RootSignatureUnavailable
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
        VerifyDelegatedTokenError::TokenVersionMismatch { .. } => {
            DelegatedAuthMetricReason::TokenVersionMismatch
        }
    }
}
