use crate::config::schema::{ConfigSchemaError, FleetConfig, Validate};

impl Validate for FleetConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
        let Some(name) = self.name.as_deref() else {
            return Ok(());
        };
        let valid = !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
        if valid {
            Ok(())
        } else {
            Err(ConfigSchemaError::ValidationError(format!(
                "invalid fleet name {name:?}; use letters, numbers, '-' or '_'"
            )))
        }
    }
}
