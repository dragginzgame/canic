use crate::{
    cdk::candid::Principal,
    config::schema::{AppConfig, ConfigSchemaError, Validate, Whitelist},
};

impl Validate for AppConfig {
    fn validate(&self) -> Result<(), ConfigSchemaError> {
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
