//! Module: config::validation::auth
//!
//! Responsibility: validate delegated-token and role-attestation configuration.
//! Does not own: auth runtime state, token verification, or schema definitions.
//! Boundary: config validation calls this before runtime installation.

use crate::config::schema::{
    AuthConfig, ConfigSchemaError, DelegatedTokenConfig, RoleAttestationConfig, Validate,
};
use crate::{
    cdk::{types::Principal, utils::hash::decode_hex},
    domain::auth::{
        DelegatedAuthNetwork, IC_ROOT_PUBLIC_KEY_RAW_LENGTH, chain_key_derivation_path_hash,
        is_mainnet_ic_root_public_key_raw,
    },
};
use k256::ecdsa::VerifyingKey as K256VerifyingKey;

impl Validate for AuthConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        self.delegated_tokens.validate()?;
        self.role_attestation.validate()
    }
}

impl Validate for DelegatedTokenConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        if let Some(max_ttl_secs) = self.max_ttl_secs
            && max_ttl_secs == 0
        {
            return Err(ConfigSchemaError::ValidationError(
                "auth.delegated_tokens.max_ttl_secs must be greater than zero".into(),
            ));
        }

        if !self.enabled {
            return Ok(());
        }

        if let Some(root_canister_id) = self.root_canister_id.as_deref() {
            if root_canister_id.trim().is_empty() {
                return Err(ConfigSchemaError::ValidationError(
                    "auth.delegated_tokens.root_canister_id must not be empty when set".into(),
                ));
            }
            Principal::from_text(root_canister_id).map_err(|err| {
                ConfigSchemaError::ValidationError(format!(
                    "auth.delegated_tokens.root_canister_id is not a valid principal: {err}"
                ))
            })?;
        }

        let network = DelegatedAuthNetwork::parse(self.network.trim()).ok_or_else(|| {
            ConfigSchemaError::ValidationError(
                "auth.delegated_tokens.network must be one of mainnet, local, pocketic, testnet"
                    .into(),
            )
        })?;
        validate_root_proof_mode(self.root_proof_mode.trim())?;

        if let Some(root_key_hex) = self.ic_root_public_key_raw_hex.as_deref() {
            if root_key_hex.trim().is_empty() {
                return Err(ConfigSchemaError::ValidationError(
                    "auth.delegated_tokens.ic_root_public_key_raw_hex must not be empty when set"
                        .into(),
                ));
            }
            let root_key = decode_hex(root_key_hex.trim()).map_err(|err| {
                ConfigSchemaError::ValidationError(format!(
                    "auth.delegated_tokens.ic_root_public_key_raw_hex is not valid hex: {err}"
                ))
            })?;
            if root_key.len() != IC_ROOT_PUBLIC_KEY_RAW_LENGTH {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "auth.delegated_tokens.ic_root_public_key_raw_hex must decode to {IC_ROOT_PUBLIC_KEY_RAW_LENGTH} bytes"
                )));
            }

            let is_mainnet_key = is_mainnet_ic_root_public_key_raw(&root_key);
            if network.is_mainnet() && !is_mainnet_key {
                return Err(ConfigSchemaError::ValidationError(
                    "auth.delegated_tokens.network=\"mainnet\" requires the known mainnet raw IC root public key"
                        .into(),
                ));
            }
            if !network.is_mainnet() && is_mainnet_key {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "auth.delegated_tokens.network=\"{}\" must not use the mainnet IC root public key",
                    network.label()
                )));
            }
        }

        validate_chain_key_root_proof_config(self, network)?;

        Ok(())
    }
}

fn validate_root_proof_mode(mode: &str) -> Result<(), ConfigSchemaError> {
    if mode == "chain_key_batch" {
        Ok(())
    } else {
        Err(ConfigSchemaError::ValidationError(
            "auth.delegated_tokens.root_proof_mode must be chain_key_batch in 0.76".into(),
        ))
    }
}

fn validate_chain_key_root_proof_config(
    config: &DelegatedTokenConfig,
    network: DelegatedAuthNetwork,
) -> Result<(), ConfigSchemaError> {
    let chain_key = &config.chain_key_root_proof;
    let key_id = required_chain_key_string(
        chain_key.key_id.as_deref(),
        "auth.delegated_tokens.chain_key_root_proof.key_id",
    )?;
    let derivation_path_hash = validate_fixed_hex(
        chain_key.derivation_path_hash_hex.as_deref(),
        "auth.delegated_tokens.chain_key_root_proof.derivation_path_hash_hex",
        32,
    )?;
    let derivation_path = validate_chain_key_derivation_path_hex(
        chain_key.derivation_path_hex.as_deref(),
        "auth.delegated_tokens.chain_key_root_proof.derivation_path_hex",
    )?;
    if chain_key_derivation_path_hash(&derivation_path).as_slice()
        != derivation_path_hash.as_slice()
    {
        return Err(ConfigSchemaError::ValidationError(
            "auth.delegated_tokens.chain_key_root_proof.derivation_path_hash_hex does not match derivation_path_hex"
                .into(),
        ));
    }
    let public_key = required_chain_key_string(
        chain_key.public_key_hex.as_deref(),
        "auth.delegated_tokens.chain_key_root_proof.public_key_hex",
    )?;
    validate_chain_key_public_key_hex(public_key)?;

    validate_required_u64(
        chain_key.key_version,
        "auth.delegated_tokens.chain_key_root_proof.key_version",
    )?;
    validate_required_u64(
        chain_key.min_accepted_key_version,
        "auth.delegated_tokens.chain_key_root_proof.min_accepted_key_version",
    )?;
    validate_required_u64(
        chain_key.min_accepted_proof_epoch,
        "auth.delegated_tokens.chain_key_root_proof.min_accepted_proof_epoch",
    )?;
    validate_required_u64(
        chain_key.min_accepted_registry_epoch,
        "auth.delegated_tokens.chain_key_root_proof.min_accepted_registry_epoch",
    )?;
    let valid_from_ns = validate_required_u64(
        chain_key.valid_from_ns,
        "auth.delegated_tokens.chain_key_root_proof.valid_from_ns",
    )?;
    let accept_until_ns = validate_required_u64(
        chain_key.accept_until_ns,
        "auth.delegated_tokens.chain_key_root_proof.accept_until_ns",
    )?;
    let max_revocation_latency_ns = validate_required_u64(
        chain_key.max_revocation_latency_ns,
        "auth.delegated_tokens.chain_key_root_proof.max_revocation_latency_ns",
    )?;

    if valid_from_ns >= accept_until_ns {
        return Err(ConfigSchemaError::ValidationError(
            "auth.delegated_tokens.chain_key_root_proof.valid_from_ns must be before accept_until_ns"
                .into(),
        ));
    }
    if max_revocation_latency_ns == 0 {
        return Err(ConfigSchemaError::ValidationError(
            "auth.delegated_tokens.chain_key_root_proof.max_revocation_latency_ns must be greater than zero"
                .into(),
        ));
    }
    if network.is_mainnet() && key_id == "test_key_1" {
        return Err(ConfigSchemaError::ValidationError(
            "auth.delegated_tokens.chain_key_root_proof.key_id must not be test_key_1 on network=\"mainnet\""
                .into(),
        ));
    }
    if !network.is_mainnet() && key_id == "test_key_1" && !chain_key.allow_test_key {
        return Err(ConfigSchemaError::ValidationError(
            "auth.delegated_tokens.chain_key_root_proof.allow_test_key must be true to use test_key_1 outside mainnet"
                .into(),
        ));
    }
    Ok(())
}

fn required_chain_key_string<'a>(
    value: Option<&'a str>,
    field: &'static str,
) -> Result<&'a str, ConfigSchemaError> {
    let Some(value) = value else {
        return Err(ConfigSchemaError::ValidationError(format!(
            "{field} is required when auth.delegated_tokens.root_proof_mode=\"chain_key_batch\""
        )));
    };
    let value = value.trim();
    if value.is_empty() {
        return Err(ConfigSchemaError::ValidationError(format!(
            "{field} must not be empty when set"
        )));
    }
    Ok(value)
}

fn validate_chain_key_public_key_hex(value: &str) -> Result<(), ConfigSchemaError> {
    let public_key = decode_hex(value).map_err(|err| {
        ConfigSchemaError::ValidationError(format!(
            "auth.delegated_tokens.chain_key_root_proof.public_key_hex is not valid hex: {err}"
        ))
    })?;
    K256VerifyingKey::from_sec1_bytes(&public_key).map_err(|err| {
        ConfigSchemaError::ValidationError(format!(
            "auth.delegated_tokens.chain_key_root_proof.public_key_hex must be a secp256k1 SEC1 public key: {err}"
        ))
    })?;
    Ok(())
}

fn validate_fixed_hex(
    value: Option<&str>,
    field: &'static str,
    expected_len: usize,
) -> Result<Vec<u8>, ConfigSchemaError> {
    let value = required_chain_key_string(value, field)?;
    let decoded = decode_hex(value).map_err(|err| {
        ConfigSchemaError::ValidationError(format!("{field} is not valid hex: {err}"))
    })?;
    if decoded.len() != expected_len {
        return Err(ConfigSchemaError::ValidationError(format!(
            "{field} must decode to {expected_len} bytes"
        )));
    }
    Ok(decoded)
}

fn validate_chain_key_derivation_path_hex(
    value: Option<&[String]>,
    field: &'static str,
) -> Result<Vec<Vec<u8>>, ConfigSchemaError> {
    let Some(path) = value else {
        return Err(ConfigSchemaError::ValidationError(format!(
            "{field} is required when auth.delegated_tokens.root_proof_mode=\"chain_key_batch\""
        )));
    };
    path.iter()
        .enumerate()
        .map(|(index, component)| {
            decode_hex(component.trim()).map_err(|err| {
                ConfigSchemaError::ValidationError(format!(
                    "{field}[{index}] is not valid hex: {err}"
                ))
            })
        })
        .collect()
}

fn validate_required_u64(
    value: Option<u64>,
    field: &'static str,
) -> Result<u64, ConfigSchemaError> {
    value.ok_or_else(|| {
        ConfigSchemaError::ValidationError(format!(
            "{field} is required when auth.delegated_tokens.root_proof_mode=\"chain_key_batch\""
        ))
    })
}

impl Validate for RoleAttestationConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        if self.max_ttl_secs == 0 {
            return Err(ConfigSchemaError::ValidationError(
                "auth.role_attestation.max_ttl_secs must be greater than zero".into(),
            ));
        }

        for role in self.min_accepted_epoch_by_role.keys() {
            if role.trim().is_empty() {
                return Err(ConfigSchemaError::ValidationError(
                    "auth.role_attestation.min_accepted_epoch_by_role keys must not be empty"
                        .into(),
                ));
            }
        }

        Ok(())
    }
}
