use crate::storage::stable::state::subnet::SubnetState;
pub use crate::storage::stable::state::subnet::SubnetStateData;

///
/// SubnetStateOps
///

pub struct SubnetStateOps;

impl SubnetStateOps {
    // -------------------------------------------------------------
    // Canonical data access
    // -------------------------------------------------------------

    #[must_use]
    pub fn data() -> SubnetStateData {
        SubnetState::export()
    }

    pub fn import(data: SubnetStateData) {
        SubnetState::import(data);
    }
}
