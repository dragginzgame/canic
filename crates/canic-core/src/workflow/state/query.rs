use crate::{
    dto::state::{AppStateView, SubnetStateView},
    ops::storage::state::{app::AppStateOps, subnet},
    workflow::state::mapper::{AppStateMapper, SubnetStateMapper},
};

pub(crate) fn app_state_view() -> AppStateView {
    let snapshot = AppStateOps::snapshot();
    AppStateMapper::snapshot_to_view(snapshot)
}

pub(crate) fn subnet_state_view() -> SubnetStateView {
    let snapshot = subnet::snapshot();
    SubnetStateMapper::snapshot_to_view(snapshot)
}
