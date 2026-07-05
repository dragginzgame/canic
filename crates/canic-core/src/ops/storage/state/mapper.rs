//! Module: ops::storage::state::mapper
//!
//! Responsibility: convert app/subnet state records to boundary inputs and views.
//! Does not own: stable state mutation, workflow orchestration, or DTO definitions.
//! Boundary: storage ops conversion layer for state records.

use crate::{
    dto::state::{
        AppCommand, AppStateInput, AppStateResponse, SubnetAuthStateInput, SubnetStateInput,
        SubnetStateResponse,
    },
    ops::storage::state::app::AppStateCommand,
    storage::stable::state::{
        app::AppStateRecord,
        subnet::{SubnetAuthStateRecord, SubnetStateRecord},
    },
};

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

///
/// AppStateMapper
///
/// Storage-ops mapper for app-state records and boundary state shapes.
///

pub struct AppStateMapper;

impl AppStateMapper {
    // Map a stored app-state snapshot into the DTO input shape.
    #[must_use]
    pub const fn record_to_input(data: AppStateRecord) -> AppStateInput {
        AppStateInput {
            mode: data.mode,
            cycles_funding_enabled: data.cycles_funding_enabled,
        }
    }

    // Map a stored app-state snapshot into the public response shape.
    #[must_use]
    pub const fn record_to_response(data: AppStateRecord) -> AppStateResponse {
        AppStateResponse {
            mode: data.mode,
            cycles_funding_enabled: data.cycles_funding_enabled,
        }
    }

    // Map a DTO input snapshot back into the stored app-state record.
    #[must_use]
    pub const fn input_to_record(view: AppStateInput) -> AppStateRecord {
        // Keep DTO-to-record conversion in ops so workflow never mutates storage records.
        AppStateRecord {
            mode: view.mode,
            cycles_funding_enabled: view.cycles_funding_enabled,
        }
    }
}

///
/// AppStateCommandMapper
///
/// Storage-ops mapper for app-state commands.
///

pub struct AppStateCommandMapper;

impl AppStateCommandMapper {
    #[must_use]
    pub const fn dto_to_record(cmd: AppCommand) -> AppStateCommand {
        match cmd {
            AppCommand::SetStatus(status) => AppStateCommand::SetStatus(status),
            AppCommand::SetCyclesFundingEnabled(enabled) => {
                AppStateCommand::SetCyclesFundingEnabled(enabled)
            }
        }
    }
}

///
/// SubnetStateMapper
///
/// Storage-ops mapper for subnet-state records and boundary state shapes.
///

pub struct SubnetStateMapper;

impl SubnetStateMapper {
    // Map a stored subnet-state snapshot into the DTO input shape.
    #[must_use]
    pub const fn record_to_input(data: SubnetStateRecord) -> SubnetStateInput {
        SubnetStateInput {
            auth: subnet_auth_record_to_input(data.auth),
        }
    }

    // Map a stored subnet-state snapshot into the public response shape.
    #[must_use]
    pub const fn record_to_response(data: SubnetStateRecord) -> SubnetStateResponse {
        SubnetStateResponse {
            auth: subnet_auth_record_to_input(data.auth),
        }
    }

    // Map a DTO input snapshot back into the stored subnet-state record.
    #[must_use]
    pub const fn input_to_record(view: SubnetStateInput) -> SubnetStateRecord {
        SubnetStateRecord {
            auth: subnet_auth_input_to_record(view.auth),
        }
    }
}

// Map stored subnet auth state into the DTO input shape.
const fn subnet_auth_record_to_input(data: SubnetAuthStateRecord) -> SubnetAuthStateInput {
    let _ = data;
    SubnetAuthStateInput {}
}

// Map subnet auth DTO state back into the stored record shape.
const fn subnet_auth_input_to_record(view: SubnetAuthStateInput) -> SubnetAuthStateRecord {
    let _ = view;
    SubnetAuthStateRecord {}
}
