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

///
/// AppStateInputMapper
///

pub struct AppStateInputMapper;

impl AppStateInputMapper {
    #[must_use]
    pub const fn record_to_view(data: AppStateRecord) -> AppStateInput {
        AppStateInput {
            mode: match data.mode {
                StorageAppMode::Enabled => AppModeDto::Enabled,
                StorageAppMode::Readonly => AppModeDto::Readonly,
                StorageAppMode::Disabled => AppModeDto::Disabled,
            },
            cycles_funding_enabled: data.cycles_funding_enabled,
        }
    }

    #[must_use]
    pub const fn dto_to_record(view: AppStateInput) -> AppStateRecord {
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
/// AppStateResponseMapper
///

pub struct AppStateResponseMapper;

impl AppStateResponseMapper {
    #[must_use]
    pub const fn record_to_view(data: AppStateRecord) -> AppStateResponse {
        AppStateResponse {
            mode: match data.mode {
                StorageAppMode::Enabled => AppModeDto::Enabled,
                StorageAppMode::Readonly => AppModeDto::Readonly,
                StorageAppMode::Disabled => AppModeDto::Disabled,
            },
            cycles_funding_enabled: data.cycles_funding_enabled,
        }
    }
}

///
/// SubnetStateInputMapper
///

pub struct SubnetStateInputMapper;

impl SubnetStateInputMapper {
    #[must_use]
    pub const fn record_to_view(_: SubnetStateRecord) -> SubnetStateInput {
        SubnetStateInput {}
    }

    #[must_use]
    pub const fn dto_to_record(_: SubnetStateInput) -> SubnetStateRecord {
        // TODO: mapping from DTO to storage record must remain in ops.
        SubnetStateRecord {}
    }
}

///
/// SubnetStateResponseMapper
///

pub struct SubnetStateResponseMapper;

impl SubnetStateResponseMapper {
    #[must_use]
    pub const fn record_to_view(_: SubnetStateRecord) -> SubnetStateResponse {
        SubnetStateResponse {}
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
