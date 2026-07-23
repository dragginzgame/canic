//! Module: config::validation::app
//!
//! Responsibility: validate App identity, mode and whitelist configuration.
//! Does not own: app runtime state, access checks, or schema definitions.
//! Boundary: config validation calls this before runtime installation.

use crate::{
    cdk::candid::Principal,
    config::schema::{AppConfig, ConfigSchemaError, Validate, Whitelist},
};

impl Validate for AppConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        let name = self.name.as_str();
        let valid = !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
        if !valid {
            return Err(ConfigSchemaError::ValidationError(format!(
                "invalid App name {name:?}; use letters, numbers, '-' or '_'"
            )));
        }
        if let Some(list) = &self.whitelist {
            list.validate()?;
        }
        Ok(())
    }
}

impl Validate for Whitelist {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        for (i, s) in self.principals.iter().enumerate() {
            if Principal::from_text(s).is_err() {
                return Err(ConfigSchemaError::ValidationError(format!(
                    "principal #{i} {s} is invalid"
                )));
            }
        }
        Ok(())
    }
}
