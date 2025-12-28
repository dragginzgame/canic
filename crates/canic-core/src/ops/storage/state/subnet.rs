use crate::{
    dto::state::SubnetStateView,
    model::memory::state::{SubnetState, SubnetStateData},
    ops::adapter::state::subnet_state_to_view,
};

///
/// SubnetStateOps
///

pub struct SubnetStateOps;

impl SubnetStateOps {
    pub fn import(data: SubnetStateData) {
        SubnetState::import(data);
    }

    #[must_use]
    pub fn export() -> SubnetStateData {
        SubnetState::export()
    }

    /// Export subnet state as a public view.
    #[must_use]
    pub fn export_view() -> SubnetStateView {
        let data = SubnetState::export();

        subnet_state_to_view(data)
    }
}
