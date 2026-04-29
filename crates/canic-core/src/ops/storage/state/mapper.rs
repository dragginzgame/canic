use crate::{
    dto::state::{
        AppCommand, AppMode as AppModeDto, AppStateInput, AppStateResponse, SubnetAuthStateInput,
        SubnetRootPublicKeyInput, SubnetStateInput, SubnetStateResponse,
    },
    ops::storage::state::app::AppStateCommand,
    storage::stable::state::{
        app::{AppMode as StorageAppMode, AppStateRecord},
        subnet::{RootPublicKeyRecord, SubnetAuthStateRecord, SubnetStateRecord},
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
    pub fn record_to_input(data: SubnetStateRecord) -> SubnetStateInput {
        SubnetStateInput {
            auth: subnet_auth_record_to_input(data.auth),
        }
    }

    // Map a stored subnet-state snapshot into the public response shape.
    #[must_use]
    pub fn record_to_response(data: SubnetStateRecord) -> SubnetStateResponse {
        SubnetStateResponse {
            auth: subnet_auth_record_to_input(data.auth),
        }
    }

    // Map a DTO input snapshot back into the stored subnet-state record.
    #[must_use]
    pub fn input_to_record(view: SubnetStateInput) -> SubnetStateRecord {
        SubnetStateRecord {
            auth: subnet_auth_input_to_record(view.auth),
        }
    }
}

// Map stored subnet auth state into the DTO input shape.
fn subnet_auth_record_to_input(data: SubnetAuthStateRecord) -> SubnetAuthStateInput {
    SubnetAuthStateInput {
        delegated_root_public_key: data.delegated_root_public_key.map(root_key_record_to_input),
    }
}

// Map subnet auth DTO state back into the stored record shape.
fn subnet_auth_input_to_record(view: SubnetAuthStateInput) -> SubnetAuthStateRecord {
    SubnetAuthStateRecord {
        delegated_root_public_key: view.delegated_root_public_key.map(root_key_input_to_record),
    }
}

// Map one root public-key record into the subnet-state DTO shape.
fn root_key_record_to_input(record: RootPublicKeyRecord) -> SubnetRootPublicKeyInput {
    SubnetRootPublicKeyInput {
        public_key_sec1: record.public_key_sec1,
        key_name: record.key_name,
        key_hash: record.key_hash,
    }
}

// Map one root public-key DTO into the subnet-state record shape.
fn root_key_input_to_record(input: SubnetRootPublicKeyInput) -> RootPublicKeyRecord {
    RootPublicKeyRecord {
        public_key_sec1: input.public_key_sec1,
        key_name: input.key_name,
        key_hash: input.key_hash,
    }
}
