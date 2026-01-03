use crate::{PublicError, dto::env::EnvView, workflow};

pub fn canic_env() -> Result<EnvView, PublicError> {
    Ok(workflow::env::query::env_view())
}
