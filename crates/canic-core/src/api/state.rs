use crate::{
    PublicError,
    dto::state::{AppStateView, SubnetStateView},
    workflow,
};

pub fn canic_app_state() -> Result<AppStateView, PublicError> {
    Ok(workflow::state::query::app_state_view())
}

pub fn canic_subnet_state() -> Result<SubnetStateView, PublicError> {
    Ok(workflow::state::query::subnet_state_view())
}
