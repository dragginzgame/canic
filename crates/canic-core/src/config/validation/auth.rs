use crate::config::schema::{
    AuthConfig, ConfigSchemaError, DelegatedTokenConfig, RoleAttestationConfig, Validate,
};

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
