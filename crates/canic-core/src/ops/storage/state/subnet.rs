use crate::storage::memory::state::subnet::{SubnetState, SubnetStateData};

///
/// SubnetStateSnapshot
/// Internal, operational snapshot of subnet state.
///
/// NOTE:
/// - Not serialized
/// - Not stable
/// - May grow over time as subnet state evolves
///

#[derive(Clone, Debug)]
pub struct SubnetStateSnapshot {
    // currently empty; add fields as needed
}

impl From<SubnetStateData> for SubnetStateSnapshot {
    fn from(_data: SubnetStateData) -> Self {
        Self {}
    }
}

impl From<SubnetStateSnapshot> for SubnetStateData {
    fn from(_snapshot: SubnetStateSnapshot) -> Self {
        Self {}
    }
}

// -------------------------------------------------------------
// Snapshot
// -------------------------------------------------------------

#[must_use]
pub fn snapshot() -> SubnetStateSnapshot {
    SubnetState::export().into()
}

// -------------------------------------------------------------
// Import
// -------------------------------------------------------------

pub fn import(snapshot: SubnetStateSnapshot) {
    let data: SubnetStateData = snapshot.into();
    SubnetState::import(data);
}
