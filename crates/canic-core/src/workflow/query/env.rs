use crate::{
    dto::env::EnvView,
    ops::runtime::env::EnvOps,
    workflow::{env::env_snapshot_to_view, query::memory},
};

pub(crate) fn env_view() -> EnvView {
    let snapshot = EnvOps::snapshot();
    env_snapshot_to_view(snapshot)
}
