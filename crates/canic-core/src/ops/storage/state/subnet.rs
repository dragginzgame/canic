use crate::{
    dto::state::{SubnetStateInput, SubnetStateResponse},
    ops::storage::state::mapper::SubnetStateMapper,
    storage::stable::state::subnet::{SubnetState, SubnetStateRecord},
};

///
/// SubnetStateOps
///

pub struct SubnetStateOps;

impl SubnetStateOps {
    /// Export the current subnet state as a DTO snapshot.
    #[must_use]
    pub fn snapshot_input() -> SubnetStateInput {
        SubnetStateMapper::record_to_input(SubnetState::export())
    }

    /// Export the current subnet state as a response snapshot.
    #[must_use]
    pub fn snapshot_response() -> SubnetStateResponse {
        SubnetStateMapper::record_to_response(SubnetState::export())
    }

    /// Import subnet state from an operational snapshot.
    #[allow(dead_code)]
    pub fn import(data: SubnetStateRecord) {
        SubnetState::import(data);
    }

    /// Import subnet state from a DTO snapshot.
    pub fn import_input(view: SubnetStateInput) {
        let record = SubnetStateMapper::input_to_record(view);
        SubnetState::import(record);
    }
}
