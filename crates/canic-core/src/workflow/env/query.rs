use crate::{dto::env::EnvView, ops::runtime::env::EnvOps, workflow::env::EnvMapper};

///
/// EnvQuery
///

pub struct EnvQuery;

impl EnvQuery {
    pub fn view() -> EnvView {
        let snapshot = EnvOps::snapshot();
        EnvMapper::snapshot_to_view(snapshot)
    }
}
