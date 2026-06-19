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
        DelegatedAuthNetwork, IC_ROOT_PUBLIC_KEY_RAW_LENGTH, is_mainnet_ic_root_public_key_raw,
    },
};

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

        Ok(())
    }
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
