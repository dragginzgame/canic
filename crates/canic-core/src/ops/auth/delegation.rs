use super::{
    DelegatedTokenOps, SignDelegationProofV2Input, keys,
    v2::{
        canonical::{derivation_path_hash, key_name_hash},
        issue::{
            IssueDelegationProofV2Error, IssueDelegationProofV2Input, finish_delegation_proof_v2,
            prepare_delegation_cert_v2,
        },
        policy::DelegatedAuthTtlPolicyV2,
    },
};
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{DelegationProofV2, ShardKeyBindingV2},
    ops::{
        auth::DelegationValidationError,
        ic::{IcOps, ecdsa::EcdsaOps},
    },
};

impl DelegatedTokenOps {
    /// Sign a V2 delegation proof with local root threshold ECDSA material.
    pub(crate) async fn sign_delegation_proof_v2(
        input: SignDelegationProofV2Input,
    ) -> Result<DelegationProofV2, InternalError> {
        let root_pid = IcOps::canister_self();
        let key_name = keys::delegated_tokens_key_name()?;
        let root_derivation_path = keys::root_derivation_path();
        let shard_derivation_path = keys::shard_derivation_path(input.shard_pid);

        let root_public_key_sec1 = Self::local_root_public_key_sec1(root_pid).await?;
        let shard_public_key_sec1 = Self::local_shard_public_key_sec1(input.shard_pid).await?;
        let prepared = prepare_delegation_cert_v2(IssueDelegationProofV2Input {
            root_pid,
            root_key_id: key_name.clone(),
            root_public_key_sec1,
            root_key_cert: input.root_key_cert,
            shard_pid: input.shard_pid,
            shard_key_id: key_name.clone(),
            shard_public_key_sec1,
            shard_key_binding: ShardKeyBindingV2::IcThresholdEcdsa {
                key_name_hash: key_name_hash(&key_name),
                derivation_path_hash: derivation_path_hash(&shard_derivation_path),
            },
            issued_at: input.issued_at,
            cert_ttl_secs: input.cert_ttl_secs,
            max_token_ttl_secs: input.max_token_ttl_secs,
            scopes: input.scopes,
            audience: input.audience,
            ttl_policy: DelegatedAuthTtlPolicyV2 {
                max_cert_ttl_secs: input.max_cert_ttl_secs,
                max_token_ttl_secs: input.max_token_ttl_secs,
            },
        })
        .map_err(map_issue_delegation_proof_v2_error)?;

        let root_sig =
            EcdsaOps::sign_bytes(&key_name, root_derivation_path, prepared.cert_hash).await?;
        Ok(finish_delegation_proof_v2(prepared, root_sig).proof)
    }

    /// Resolve the local shard public key, fetching and caching it on demand.
    pub(crate) async fn local_shard_public_key_sec1(
        shard_pid: Principal,
    ) -> Result<Vec<u8>, InternalError> {
        if let Some(shard_public_key) =
            crate::ops::storage::auth::DelegationStateOps::shard_public_key(shard_pid)
        {
            return Ok(shard_public_key);
        }

        let key_name = keys::delegated_tokens_key_name()?;
        let shard_public_key =
            EcdsaOps::public_key_sec1(&key_name, keys::shard_derivation_path(shard_pid), shard_pid)
                .await?;
        crate::ops::storage::auth::DelegationStateOps::set_shard_public_key(
            shard_pid,
            shard_public_key.clone(),
        );

        Ok(shard_public_key)
    }

    /// Resolve the local root public key, fetching and caching it on demand.
    pub(crate) async fn local_root_public_key_sec1(
        root_pid: Principal,
    ) -> Result<Vec<u8>, InternalError> {
        let local = IcOps::canister_self();
        if root_pid != local {
            return Err(DelegationValidationError::InvalidRootAuthority {
                expected: local,
                found: root_pid,
            }
            .into());
        }

        if let Some(root_public_key) =
            crate::ops::storage::auth::DelegationStateOps::root_public_key()
        {
            return Ok(root_public_key);
        }

        let key_name = keys::delegated_tokens_key_name()?;
        keys::ensure_root_public_key_cached(&key_name, root_pid).await?;
        crate::ops::storage::auth::DelegationStateOps::root_public_key()
            .ok_or_else(|| super::DelegationSignatureError::RootPublicKeyUnavailable.into())
    }
}

fn map_issue_delegation_proof_v2_error(err: IssueDelegationProofV2Error) -> InternalError {
    DelegationValidationError::DelegatedAuthV2(err.to_string()).into()
}
