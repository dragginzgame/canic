use super::{
    DelegatedTokenOps, SignDelegatedTokenV2Input, VerifyDelegatedTokenV2RuntimeInput, keys,
    v2::mint::{
        MintDelegatedTokenV2Error, MintDelegatedTokenV2Input, finish_delegated_token_v2,
        prepare_delegated_token_v2,
    },
    v2::{
        canonical::{derivation_path_hash, key_name_hash, public_key_hash},
        policy::DelegatedAuthTtlPolicyV2,
        verify::{
            VerifiedDelegationV2, VerifyDelegatedTokenV2Error, VerifyDelegatedTokenV2Input,
            verify_delegated_token_v2,
        },
    },
};
use crate::{
    InternalError,
    dto::auth::{
        DelegatedTokenV2, RootKeySetV2, RootPublicKeyV2, RootTrustAnchorV2, ShardKeyBindingV2,
    },
    ops::{
        auth::{DelegationScopeError, DelegationSignatureError, DelegationValidationError},
        config::ConfigOps,
        ic::{IcOps, ecdsa::EcdsaOps},
        runtime::env::EnvOps,
        storage::auth::DelegationStateOps,
    },
};

impl DelegatedTokenOps {
    /// Sign a self-contained V2 delegated token with local shard threshold ECDSA material.
    pub async fn sign_token_v2(
        input: SignDelegatedTokenV2Input,
    ) -> Result<DelegatedTokenV2, InternalError> {
        let local = IcOps::canister_self();
        if input.proof.cert.shard_pid != local {
            return Err(DelegationScopeError::ShardPidMismatch {
                expected: local,
                found: input.proof.cert.shard_pid,
            }
            .into());
        }

        let prepared = prepare_delegated_token_v2(MintDelegatedTokenV2Input {
            proof: &input.proof,
            subject: input.subject,
            audience: input.audience,
            scopes: input.scopes,
            ttl_secs: input.ttl_secs,
            nonce: input.nonce,
            now_secs: IcOps::now_secs(),
        })
        .map_err(map_mint_delegated_token_v2_error)?;

        let key_name = keys::delegated_tokens_key_name()?;
        keys::ensure_shard_public_key_cached(&key_name, local).await?;
        let shard_sig = EcdsaOps::sign_bytes(
            &key_name,
            keys::shard_derivation_path(local),
            prepared.claims_hash,
        )
        .await?;

        Ok(finish_delegated_token_v2(prepared, shard_sig))
    }

    /// Verify a self-contained V2 delegated token without local proof lookup.
    pub fn verify_token_v2(
        input: VerifyDelegatedTokenV2RuntimeInput<'_>,
    ) -> Result<VerifiedDelegationV2, InternalError> {
        let cfg = ConfigOps::delegated_tokens_config()?;
        if !cfg.enabled {
            return Err(DelegationValidationError::DelegatedTokenAuthDisabled.into());
        }

        Self::verify_v2_shard_key_binding(input.token)?;
        let root_trust = Self::root_trust_anchor_v2(input.token, input.now_secs)?;
        let local_role = EnvOps::canister_role()?;

        verify_delegated_token_v2(
            VerifyDelegatedTokenV2Input {
                token: input.token,
                root_trust: &root_trust,
                local_principal: IcOps::canister_self(),
                local_role: Some(&local_role),
                ttl_policy: DelegatedAuthTtlPolicyV2 {
                    max_cert_ttl_secs: input.max_cert_ttl_secs,
                    max_token_ttl_secs: input.max_token_ttl_secs,
                },
                expected_shard_key_hash: input.token.proof.cert.shard_key_hash,
                required_scopes: input.required_scopes,
                now_secs: input.now_secs,
            },
            |public_key, hash, sig, _alg| {
                EcdsaOps::verify_signature(public_key, hash, sig).map_err(|err| err.to_string())
            },
        )
        .map_err(map_verify_delegated_token_v2_error)
    }

    /// Ensure the expected root public key for a V2 token is cached locally.
    pub async fn ensure_v2_root_public_key_cached(
        token: &DelegatedTokenV2,
    ) -> Result<(), InternalError> {
        let cert = &token.proof.cert;
        let expected_root = EnvOps::root_pid()?;
        if cert.root_pid != expected_root {
            return Err(DelegationValidationError::DelegatedAuthV2(format!(
                "delegated auth v2 root pid mismatch (expected {expected_root}, found {})",
                cert.root_pid
            ))
            .into());
        }

        let key_name = keys::delegated_tokens_key_name()?;
        if cert.root_key_id != key_name {
            return Err(DelegationValidationError::DelegatedAuthV2(format!(
                "delegated auth v2 root key id mismatch (expected {key_name}, found {})",
                cert.root_key_id
            ))
            .into());
        }

        if let Some(root_public_key) = DelegationStateOps::root_public_key()
            && public_key_hash(&root_public_key) == cert.root_key_hash
        {
            return Ok(());
        }

        let root_public_key =
            EcdsaOps::public_key_sec1(&key_name, keys::root_derivation_path(), cert.root_pid)
                .await?;
        if public_key_hash(&root_public_key) != cert.root_key_hash {
            return Err(DelegationValidationError::DelegatedAuthV2(
                "delegated auth v2 fetched root public key hash mismatch".to_string(),
            )
            .into());
        }

        DelegationStateOps::set_root_public_key(root_public_key);
        Ok(())
    }

    fn root_trust_anchor_v2(
        token: &DelegatedTokenV2,
        now_secs: u64,
    ) -> Result<RootTrustAnchorV2, InternalError> {
        let cert = &token.proof.cert;
        let root_public_key = DelegationStateOps::root_public_key()
            .ok_or(DelegationSignatureError::RootPublicKeyUnavailable)?;
        let key_hash = public_key_hash(&root_public_key);

        Ok(RootTrustAnchorV2 {
            root_pid: EnvOps::root_pid()?,
            trusted_root_keys: RootKeySetV2 {
                keys: vec![RootPublicKeyV2 {
                    root_pid: cert.root_pid,
                    key_id: cert.root_key_id.clone(),
                    alg: cert.alg,
                    public_key_sec1: root_public_key,
                    key_hash,
                    not_before: cert.issued_at.min(now_secs),
                    not_after: None,
                }],
            },
            key_authority: None,
        })
    }

    fn verify_v2_shard_key_binding(token: &DelegatedTokenV2) -> Result<(), InternalError> {
        let cert = &token.proof.cert;
        let key_name = keys::delegated_tokens_key_name()?;
        let expected_derivation_path_hash =
            derivation_path_hash(&keys::shard_derivation_path(cert.shard_pid));
        match cert.shard_key_binding {
            ShardKeyBindingV2::IcThresholdEcdsa {
                key_name_hash: observed_key_name_hash,
                derivation_path_hash: observed_derivation_path_hash,
            } => {
                if observed_key_name_hash != key_name_hash(&key_name)
                    || observed_derivation_path_hash != expected_derivation_path_hash
                {
                    return Err(DelegationValidationError::DelegatedAuthV2(
                        "delegated auth v2 shard key binding mismatch".to_string(),
                    )
                    .into());
                }
            }
        }
        Ok(())
    }
}

fn map_mint_delegated_token_v2_error(err: MintDelegatedTokenV2Error) -> InternalError {
    DelegationValidationError::DelegatedAuthV2(err.to_string()).into()
}

fn map_verify_delegated_token_v2_error(err: VerifyDelegatedTokenV2Error) -> InternalError {
    DelegationValidationError::DelegatedAuthV2(err.to_string()).into()
}
