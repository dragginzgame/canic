//! Module: config::validation::app
//!
//! Responsibility: validate App identity and mode configuration.
//! Does not own: app runtime state, access checks, or schema definitions.
//! Boundary: config validation calls this before runtime installation.

use crate::config::schema::{
    AppConfig, AppNameIssue, ConfigSchemaError, Validate, validate_app_name,
};

impl Validate for AppConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        let name = self.name.as_str();
        if let Err(issue) = validate_app_name(name) {
            let guidance = match issue {
                AppNameIssue::Empty | AppNameIssue::InvalidCharacters => {
                    "use letters, numbers, '-' or '_'".to_string()
                }
                AppNameIssue::TooLong { max_bytes } => {
                    format!("names must not exceed {max_bytes} bytes")
                }
            };
            return Err(ConfigSchemaError::ValidationError(format!(
                "invalid App name {name:?}; {guidance}"
            )));
        }
        Ok(())
    }
}
