use crate::{
    dto::state::{AppStateView, SubnetStateView},
    ops::storage::state::{app::AppStateOps, subnet::SubnetStateOps},
    workflow::state::mapper::{AppStateMapper, SubnetStateMapper},
};

pub fn app_state_view() -> AppStateView {
    let snapshot = AppStateOps::snapshot();
    AppStateMapper::snapshot_to_view(snapshot)
}

pub fn subnet_state_view() -> SubnetStateView {
    let snapshot = SubnetStateOps::snapshot();
    SubnetStateMapper::snapshot_to_view(snapshot)
}
