use crate::{
    dto::state::{AppStateResponse, SubnetStateResponse},
    ops::storage::state::{app::AppStateOps, subnet::SubnetStateOps},
};

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

///
/// SubnetStateQuery
///

pub struct SubnetStateQuery;

impl SubnetStateQuery {
    #[must_use]
    pub fn snapshot() -> SubnetStateResponse {
        SubnetStateOps::snapshot_response()
    }
}
