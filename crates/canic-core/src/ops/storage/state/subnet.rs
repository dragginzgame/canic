use crate::{
    dto::state::SubnetStateInput,
    ops::storage::state::mapper::SubnetStateInputMapper,
    storage::stable::state::subnet::{SubnetState, SubnetStateRecord},
};

///
/// SubnetStateOps
///

pub struct SubnetStateOps;

impl SubnetStateOps {
    // -------------------------------------------------------------
    // Canonical data access
    // -------------------------------------------------------------

    #[must_use]
    pub fn data() -> SubnetStateRecord {
        SubnetState::export()
    }

    /// Export the current subnet state as a DTO snapshot.
    #[must_use]
    pub fn snapshot_input() -> SubnetStateInput {
        SubnetStateInputMapper::record_to_view(SubnetState::export())
    }

    #[expect(dead_code)]
    pub fn import(data: SubnetStateRecord) {
        SubnetState::import(data);
    }

    pub fn import_input(view: SubnetStateInput) {
        let record = SubnetStateInputMapper::dto_to_record(view);
        SubnetState::import(record);
    }
}
