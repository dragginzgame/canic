use crate::{
    dto::state::{AppStateResponse, SubnetStateResponse},
    ops::storage::state::{
        app::AppStateOps,
        mapper::{AppStateResponseMapper, SubnetStateResponseMapper},
        subnet::SubnetStateOps,
    },
};

///
/// AppStateQuery
///

pub struct AppStateQuery;

impl AppStateQuery {
    #[must_use]
    pub fn snapshot() -> AppStateResponse {
        let data = AppStateOps::data();

        AppStateResponseMapper::record_to_view(data)
    }
}

///
/// SubnetStateQuery
///

pub struct SubnetStateQuery;

impl SubnetStateQuery {
    #[must_use]
    pub fn snapshot() -> SubnetStateResponse {
        let data = SubnetStateOps::data();

        SubnetStateResponseMapper::record_to_view(data)
    }
}
