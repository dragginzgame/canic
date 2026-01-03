use crate::{dto::env::EnvView, workflow};

///
/// Env API
///

#[must_use]
pub fn env() -> EnvView {
    workflow::env::query::env_view()
}
