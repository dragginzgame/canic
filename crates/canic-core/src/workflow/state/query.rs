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
    pub fn view() -> AppStateView {
        let snapshot = AppStateOps::snapshot();

        AppStateMapper::snapshot_to_view(snapshot)
    }
}

///
/// SubnetStateQuery
///

pub struct SubnetStateQuery;

impl SubnetStateQuery {
    pub fn view() -> SubnetStateView {
        let snapshot = SubnetStateOps::snapshot();

        SubnetStateMapper::snapshot_to_view(snapshot)
    }
}
