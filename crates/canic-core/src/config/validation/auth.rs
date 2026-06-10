use crate::cdk::{types::Principal, utils::hash::decode_hex};
use crate::config::schema::{
    AuthConfig, ConfigSchemaError, DelegatedTokenConfig, RoleAttestationConfig, Validate,
};

const IC_ROOT_PUBLIC_KEY_RAW_BYTES: usize = 96;

impl Validate for AuthConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        self.delegated_tokens.validate()?;
        self.role_attestation.validate()
    }
}

impl Validate for DelegatedTokenConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        if self.ecdsa_key_name.trim().is_empty() {
            return Err(ConfigSchemaError::ValidationError(
                "auth.delegated_tokens.ecdsa_key_name must not be empty".into(),
            ));
        }

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
            if root_key.len() != IC_ROOT_PUBLIC_KEY_RAW_BYTES {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "auth.delegated_tokens.ic_root_public_key_raw_hex must decode to {IC_ROOT_PUBLIC_KEY_RAW_BYTES} bytes"
                )));
            }
        }

        match self.network.as_str() {
            "mainnet" | "local" | "pocketic" | "testnet" => {}
            _ => {
                return Err(ConfigSchemaError::ValidationError(
                    "auth.delegated_tokens.network must be one of mainnet, local, pocketic, testnet"
                        .into(),
                ));
            }
        }

        Ok(())
    }
}

impl Validate for RoleAttestationConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        if self.ecdsa_key_name.trim().is_empty() {
            return Err(ConfigSchemaError::ValidationError(
                "auth.role_attestation.ecdsa_key_name must not be empty".into(),
            ));
        }

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
