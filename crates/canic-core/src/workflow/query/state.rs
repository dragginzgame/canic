use crate::{
    dto::state::{AppStateView, SubnetStateView},
    ops::storage::state::{app::AppStateOps, subnet::SubnetStateOps},
    workflow::state::adapter::{app_state_view_from_snapshot, subnet_state_view_from_snapshot},
};

pub(crate) fn app_state_view() -> AppStateView {
    let snapshot = AppStateOps::snapshot();
    app_state_view_from_snapshot(snapshot)
}

pub(crate) fn subnet_state_view() -> SubnetStateView {
    let snapshot = SubnetStateOps::snapshot();
    subnet_state_view_from_snapshot(snapshot)
}
