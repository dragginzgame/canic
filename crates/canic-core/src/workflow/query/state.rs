use crate::{
    dto::state::{AppStateView, SubnetStateView},
    ops::storage::state::{app::AppStateOps, subnet::SubnetStateOps},
};

pub(crate) fn app_state_view() -> AppStateView {
    AppStateOps::export_view()
}

pub(crate) fn subnet_state_view() -> SubnetStateView {
    SubnetStateOps::export_view()
}
