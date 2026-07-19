//! Module: ops::auth::token::verifier_config
//!
//! Responsibility: derive delegated-token trust anchors from canonical configuration.
//! Does not own: token verification flow, metrics, cache mutation, or proof storage.
//! Boundary: deterministic config validation and verifier-policy construction.

use super::*;

pub(super) fn configured_root_canister_id(
    cfg: &DelegatedTokenConfig,
) -> Result<Principal, InternalError> {
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

pub(super) fn configured_root_proof_mode(
    cfg: &DelegatedTokenConfig,
) -> Result<RootProofMode, InternalError> {
    match cfg.root_proof_mode.trim() {
        "chain_key_batch" => Ok(RootProofMode::ChainKeyBatch),
        _ => Err(AuthValidationError::Auth(
            "auth.delegated_tokens.root_proof_mode must be chain_key_batch".to_string(),
        )
        .into()),
    }
}

pub(super) fn configured_chain_key_root_verifier(
    cfg: &DelegatedTokenConfig,
    root_canister_id: Principal,
    build_network: BuildNetwork,
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
    if build_network == BuildNetwork::Ic && key_id == "test_key_1" {
        return Err(AuthValidationError::Auth(
            "auth.delegated_tokens.chain_key_root_proof.key_id must not be test_key_1 on build_network=\"ic\""
                .to_string(),
        )
        .into());
    }
    if build_network == BuildNetwork::Local && key_id == "test_key_1" && !chain_key.allow_test_key {
        return Err(AuthValidationError::Auth(
            "auth.delegated_tokens.chain_key_root_proof.allow_test_key must be true to use test_key_1 on build_network=\"local\""
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
            build_network,
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

pub(super) fn chain_key_policy_from_config(
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

pub(super) fn configured_ic_root_public_key_raw(
    cfg: &DelegatedTokenConfig,
    build_network: BuildNetwork,
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
                "auth.delegated_tokens.ic_root_public_key_raw_hex is required when delegated-token verification is enabled for build_network=\"{build_network}\""
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

    validate_build_network_root_key_pair(build_network, &ic_root_public_key_raw)?;
    Ok(ic_root_public_key_raw)
}

pub(super) fn validate_build_network_root_key_pair(
    build_network: BuildNetwork,
    ic_root_public_key_raw: &[u8],
) -> Result<(), InternalError> {
    let is_mainnet_key = is_mainnet_ic_root_public_key_raw(ic_root_public_key_raw);
    if build_network == BuildNetwork::Ic {
        if is_mainnet_key {
            return Ok(());
        }
        return Err(AuthValidationError::Auth(
            "auth.delegated_tokens.build_network=\"ic\" requires the known mainnet raw IC root public key".to_string(),
        )
        .into());
    }

    if is_mainnet_key {
        return Err(AuthValidationError::Auth(
            "auth.delegated_tokens.build_network=\"local\" must not use the mainnet IC root public key"
                .to_string(),
        )
        .into());
    }

    Ok(())
}
