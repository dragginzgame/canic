use crate::{
    dto::state::SubnetStateView,
    ops::adapter::state::{subnet_state_from_view, subnet_state_to_view},
    storage::memory::state::subnet::SubnetState,
};

///
/// SubnetStateOps
///

pub struct SubnetStateOps;

impl SubnetStateOps {
    /// Import subnet state from a public view.
    pub fn import_view(view: SubnetStateView) {
        let data = subnet_state_from_view(view);
        SubnetState::import(data);
    }

    /// Export subnet state as a public view.
    #[must_use]
    pub fn export_view() -> SubnetStateView {
        let data = SubnetState::export();

        subnet_state_to_view(data)
    }
}
