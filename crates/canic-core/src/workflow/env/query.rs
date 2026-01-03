use crate::{dto::env::EnvView, ops::runtime::env::EnvOps, workflow::env::EnvMapper};

pub fn env_view() -> EnvView {
    let snapshot = EnvOps::snapshot();
    EnvMapper::snapshot_to_view(snapshot)
}
