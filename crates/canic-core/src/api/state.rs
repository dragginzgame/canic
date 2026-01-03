use crate::{
    dto::state::{AppStateView, SubnetStateView},
    workflow,
};

///
/// State API
///

#[must_use]
pub fn app_state() -> AppStateView {
    workflow::state::query::app_state_view()
}

#[must_use]
pub fn subnet_state() -> SubnetStateView {
    workflow::state::query::subnet_state_view()
}
