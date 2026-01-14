use crate::{
    dto::state::{AppStateView, SubnetStateView},
    ops::storage::state::{app::AppStateOps, subnet::SubnetStateOps},
    workflow::state::mapper::{AppStateMapper, SubnetStateMapper},
};

///
/// AppStateQuery
///

pub struct AppStateQuery;

impl AppStateQuery {
    #[must_use]
    pub fn view() -> AppStateView {
        let data = AppStateOps::data();

        AppStateMapper::data_to_view(data)
    }
}

///
/// SubnetStateQuery
///

pub struct SubnetStateQuery;

impl SubnetStateQuery {
    #[must_use]
    pub fn view() -> SubnetStateView {
        let data = SubnetStateOps::data();

        SubnetStateMapper::data_to_view(data)
    }
}
