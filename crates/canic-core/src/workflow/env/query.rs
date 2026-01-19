use crate::{dto::env::EnvSnapshotResponse, ops::runtime::env::EnvOps};

///
/// EnvQuery
///

pub struct EnvQuery;

impl EnvQuery {
    #[must_use]
    pub fn snapshot() -> EnvSnapshotResponse {
        EnvOps::snapshot_response()
    }
}
