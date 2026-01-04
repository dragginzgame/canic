use crate::{dto::env::EnvView, ops::runtime::env, workflow::env::EnvMapper};

pub fn env_view() -> EnvView {
    let snapshot = env::snapshot();
    EnvMapper::snapshot_to_view(snapshot)
}
