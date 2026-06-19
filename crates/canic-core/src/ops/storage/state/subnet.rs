//! Module: ops::storage::state::subnet
//!
//! Responsibility: import and snapshot deterministic subnet-state records.
//! Does not own: endpoint authorization, workflow orchestration, or DTO schemas.
//! Boundary: storage ops facade over the stable subnet-state record.

use crate::{
    dto::state::{SubnetStateInput, SubnetStateResponse},
    ops::storage::state::mapper::SubnetStateMapper,
    storage::stable::state::subnet::{SubnetState, SubnetStateRecord},
};

///
/// SubnetStateOps
///
/// Storage-ops facade for subnet-state imports and snapshots.
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

    // Import sanitized subnet state from an operational snapshot.
    fn import_record(data: SubnetStateRecord) {
        SubnetState::import(sanitized_subnet_state(data));
    }

    /// Import subnet state from a DTO snapshot.
    pub fn import_input(view: SubnetStateInput) {
        let record = SubnetStateMapper::input_to_record(view);
        Self::import_record(record);
    }
}

// Keep subnet auth snapshots constrained to the active state shape.
const fn sanitized_subnet_state(data: SubnetStateRecord) -> SubnetStateRecord {
    data
}
