//! Module: ops::storage::state::mapper
//!
//! Responsibility: convert Fleet-state records to boundary inputs and views.
//! Does not own: stable state mutation, workflow orchestration, or DTO definitions.
//! Boundary: storage ops conversion layer for state records.

use crate::{
    dto::state::{FleetCommand, FleetStateInput, FleetStateResponse},
    ops::storage::state::fleet::FleetStateCommand,
    storage::stable::state::fleet::FleetStateRecord,
};

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

///
/// FleetStateMapper
///
/// Storage-ops mapper for Fleet-state records and boundary state shapes.
///

pub struct FleetStateMapper;

impl FleetStateMapper {
    // Map a stored Fleet-state snapshot into the DTO input shape.
    #[must_use]
    pub const fn record_to_input(data: FleetStateRecord) -> FleetStateInput {
        FleetStateInput {
            mode: data.mode,
            cycles_funding_enabled: data.cycles_funding_enabled,
        }
    }

    // Map a stored Fleet-state snapshot into the public response shape.
    #[must_use]
    pub const fn record_to_response(data: FleetStateRecord) -> FleetStateResponse {
        FleetStateResponse {
            mode: data.mode,
            cycles_funding_enabled: data.cycles_funding_enabled,
        }
    }

    // Map a DTO input snapshot back into the stored Fleet-state record.
    #[must_use]
    pub const fn input_to_record(view: FleetStateInput) -> FleetStateRecord {
        // Keep DTO-to-record conversion in ops so workflow never mutates storage records.
        FleetStateRecord {
            mode: view.mode,
            cycles_funding_enabled: view.cycles_funding_enabled,
        }
    }
}

///
/// FleetStateCommandMapper
///
/// Storage-ops mapper for Fleet-state commands.
///

pub struct FleetStateCommandMapper;

impl FleetStateCommandMapper {
    #[must_use]
    pub const fn dto_to_record(cmd: FleetCommand) -> FleetStateCommand {
        match cmd {
            FleetCommand::SetStatus(status) => FleetStateCommand::SetStatus(status),
            FleetCommand::SetCyclesFundingEnabled(enabled) => {
                FleetStateCommand::SetCyclesFundingEnabled(enabled)
            }
        }
    }
}
