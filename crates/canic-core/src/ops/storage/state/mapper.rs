use crate::{
    dto::state::{
        AppCommand, AppMode as AppModeDto, AppStateInput, AppStateResponse, SubnetStateInput,
        SubnetStateResponse,
    },
    ops::storage::state::app::AppStateCommand,
    storage::stable::state::{
        app::{AppMode as StorageAppMode, AppStateRecord},
        subnet::SubnetStateRecord,
    },
};

// --- Helpers ---------------------------------------------------------------

// Map stored app mode values into the shared DTO enum.
const fn app_mode_to_dto(mode: StorageAppMode) -> AppModeDto {
    match mode {
        StorageAppMode::Enabled => AppModeDto::Enabled,
        StorageAppMode::Readonly => AppModeDto::Readonly,
        StorageAppMode::Disabled => AppModeDto::Disabled,
    }
}

///
/// AppStateMapper
///

pub struct AppStateMapper;

impl AppStateMapper {
    // Map a stored app-state snapshot into the DTO input shape.
    #[must_use]
    pub const fn record_to_input(data: AppStateRecord) -> AppStateInput {
        AppStateInput {
            mode: app_mode_to_dto(data.mode),
            cycles_funding_enabled: data.cycles_funding_enabled,
        }
    }

    // Map a stored app-state snapshot into the public response shape.
    #[must_use]
    pub const fn record_to_response(data: AppStateRecord) -> AppStateResponse {
        AppStateResponse {
            mode: app_mode_to_dto(data.mode),
            cycles_funding_enabled: data.cycles_funding_enabled,
        }
    }

    // Map a DTO input snapshot back into the stored app-state record.
    #[must_use]
    pub const fn input_to_record(view: AppStateInput) -> AppStateRecord {
        // TODO: mapping from DTO to storage record must remain in ops.
        AppStateRecord {
            mode: match view.mode {
                AppModeDto::Enabled => StorageAppMode::Enabled,
                AppModeDto::Readonly => StorageAppMode::Readonly,
                AppModeDto::Disabled => StorageAppMode::Disabled,
            },
            cycles_funding_enabled: view.cycles_funding_enabled,
        }
    }
}

///
/// AppStateCommandMapper
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

pub struct SubnetStateMapper;

impl SubnetStateMapper {
    // Map a stored subnet-state snapshot into the DTO input shape.
    #[must_use]
    pub const fn record_to_input(_: SubnetStateRecord) -> SubnetStateInput {
        SubnetStateInput
    }

    // Map a stored subnet-state snapshot into the public response shape.
    #[must_use]
    pub const fn record_to_response(_: SubnetStateRecord) -> SubnetStateResponse {
        SubnetStateResponse
    }

    // Map a DTO input snapshot back into the stored subnet-state record.
    #[must_use]
    pub const fn input_to_record(_: SubnetStateInput) -> SubnetStateRecord {
        SubnetStateRecord
    }
}
