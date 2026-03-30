use crate::{dto::state::AppStateResponse, ops::storage::state::app::AppStateOps};

///
/// AppStateQuery
///

pub struct AppStateQuery;

impl AppStateQuery {
    #[must_use]
    pub fn snapshot() -> AppStateResponse {
        AppStateOps::snapshot_response()
    }
}
