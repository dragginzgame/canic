use super::{
    ATTESTATION_PATH_SEGMENT, DERIVATION_NAMESPACE, ROLE_ATTESTATION_KEY_ID_V1, ROOT_PATH_SEGMENT,
    SHARD_PATH_SEGMENT,
};
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::auth::{AttestationKey, AttestationKeyStatus},
    ops::{
        auth::DelegatedTokenOpsError, config::ConfigOps, ic::ecdsa::EcdsaOps,
        storage::auth::DelegationStateOps,
    },
};
use std::cmp::Reverse;

pub(super) fn attestation_keys_sorted() -> Vec<AttestationKey> {
    let mut keys = DelegationStateOps::attestation_keys();
    keys.sort_by_key(|entry| {
        let status_rank = match entry.status {
            AttestationKeyStatus::Current => 0u8,
            AttestationKeyStatus::Previous => 1u8,
        };
        (status_rank, Reverse(entry.key_id))
    });
    keys
}

pub(super) fn delegated_tokens_key_name() -> Result<String, InternalError> {
    let cfg = ConfigOps::delegated_tokens_config()?;
    if cfg.ecdsa_key_name.trim().is_empty() {
        return Err(DelegatedTokenOpsError::EcdsaKeyNameMissing.into());
    }

    Ok(cfg.ecdsa_key_name)
}

pub(super) fn attestation_key_name() -> Result<String, InternalError> {
    let cfg = ConfigOps::role_attestation_config()?;
    if cfg.ecdsa_key_name.trim().is_empty() {
        return Err(DelegatedTokenOpsError::AttestationKeyNameMissing.into());
    }

    Ok(cfg.ecdsa_key_name)
}

pub(super) fn root_derivation_path() -> Vec<Vec<u8>> {
    vec![DERIVATION_NAMESPACE.to_vec(), ROOT_PATH_SEGMENT.to_vec()]
}

pub(super) fn shard_derivation_path(shard_pid: Principal) -> Vec<Vec<u8>> {
    vec![
        DERIVATION_NAMESPACE.to_vec(),
        SHARD_PATH_SEGMENT.to_vec(),
        shard_pid.as_slice().to_vec(),
    ]
}

pub(super) fn attestation_derivation_path() -> Vec<Vec<u8>> {
    vec![
        DERIVATION_NAMESPACE.to_vec(),
        ATTESTATION_PATH_SEGMENT.to_vec(),
        ROOT_PATH_SEGMENT.to_vec(),
    ]
}

pub(super) async fn ensure_attestation_key_cached(
    key_name: &str,
    root_pid: Principal,
    now_secs: u64,
) -> Result<(), InternalError> {
    if DelegationStateOps::attestation_public_key_sec1(ROLE_ATTESTATION_KEY_ID_V1).is_some() {
        return Ok(());
    }

    let public_key =
        EcdsaOps::public_key_sec1(key_name, attestation_derivation_path(), root_pid).await?;
    DelegationStateOps::upsert_attestation_key(AttestationKey {
        key_id: ROLE_ATTESTATION_KEY_ID_V1,
        public_key,
        status: AttestationKeyStatus::Current,
        valid_from: Some(now_secs),
        valid_until: None,
    });

    Ok(())
}

pub(super) async fn ensure_root_public_key_cached(
    key_name: &str,
    root_pid: Principal,
) -> Result<(), InternalError> {
    if DelegationStateOps::root_public_key().is_some() {
        return Ok(());
    }

    let root_pk = EcdsaOps::public_key_sec1(key_name, root_derivation_path(), root_pid).await?;
    DelegationStateOps::set_root_public_key(root_pk);
    Ok(())
}

pub(super) async fn ensure_shard_public_key_cached(
    key_name: &str,
    shard_pid: Principal,
) -> Result<(), InternalError> {
    if DelegationStateOps::shard_public_key(shard_pid).is_some() {
        return Ok(());
    }

    let shard_pk =
        EcdsaOps::public_key_sec1(key_name, shard_derivation_path(shard_pid), shard_pid).await?;
    DelegationStateOps::set_shard_public_key(shard_pid, shard_pk);
    Ok(())
}
