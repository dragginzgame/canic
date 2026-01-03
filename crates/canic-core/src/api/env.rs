use crate::{dto::env::EnvView, workflow};

#[must_use]
pub fn env() -> EnvView {
    workflow::env::query::env_view()
}
