use crate::{
    dto::state::{
        AppCommand, AppMode as AppModeDto, AppStateInput, AppStateResponse, SubnetStateInput,
        SubnetStateResponse, WasmStoreStateInput, WasmStoreStateResponse,
    },
    dto::template::WasmStorePublicationStateResponse,
    ops::storage::state::app::AppStateCommand,
    storage::stable::state::{
        app::{AppMode as StorageAppMode, AppStateRecord},
        subnet::{PublicationStoreStateRecord, SubnetStateRecord, WasmStoreRecord},
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
/// SubnetStateMapper
///

pub struct SubnetStateMapper;

impl SubnetStateMapper {
    // Map one stored wasm-store record into the DTO input shape.
    #[must_use]
    pub fn wasm_store_record_to_input(data: WasmStoreRecord) -> WasmStoreStateInput {
        WasmStoreStateInput {
            binding: data.binding,
            pid: data.pid,
            created_at: data.created_at,
        }
    }

    // Map one stored wasm-store record into the DTO response shape.
    #[must_use]
    pub fn wasm_store_record_to_response(data: WasmStoreRecord) -> WasmStoreStateResponse {
        WasmStoreStateResponse {
            binding: data.binding,
            pid: data.pid,
            created_at: data.created_at,
        }
    }

    // Map one DTO input snapshot back into the stored wasm-store record.
    #[must_use]
    pub fn wasm_store_input_to_record(data: WasmStoreStateInput) -> WasmStoreRecord {
        WasmStoreRecord {
            binding: data.binding,
            pid: data.pid,
            created_at: data.created_at,
        }
    }

    // Map the stored subnet-state snapshot into the DTO input shape.
    #[must_use]
    pub fn record_to_input(data: SubnetStateRecord) -> SubnetStateInput {
        SubnetStateInput {
            wasm_stores: Some(
                data.wasm_stores
                    .into_iter()
                    .map(Self::wasm_store_record_to_input)
                    .collect(),
            ),
        }
    }

    // Map the stored subnet-state snapshot into the public response shape.
    #[must_use]
    pub fn record_to_response(data: SubnetStateRecord) -> SubnetStateResponse {
        SubnetStateResponse {
            wasm_stores: data
                .wasm_stores
                .into_iter()
                .map(Self::wasm_store_record_to_response)
                .collect(),
        }
    }

    // Map the stored publication lifecycle record into the template response shape.
    #[must_use]
    pub fn publication_store_record_to_response(
        data: PublicationStoreStateRecord,
    ) -> WasmStorePublicationStateResponse {
        WasmStorePublicationStateResponse {
            active_binding: data.active_binding,
            detached_binding: data.detached_binding,
            retired_binding: data.retired_binding,
            generation: data.generation,
            changed_at: data.changed_at,
            retired_at: data.retired_at,
        }
    }

    // Map a DTO input snapshot back into the stored subnet-state record.
    #[must_use]
    pub fn input_to_record(view: SubnetStateInput) -> SubnetStateRecord {
        SubnetStateRecord {
            publication_store: PublicationStoreStateRecord {
                active_binding: None,
                detached_binding: None,
                retired_binding: None,
                generation: 0,
                changed_at: 0,
                retired_at: 0,
            },
            wasm_stores: view
                .wasm_stores
                .unwrap_or_default()
                .into_iter()
                .map(Self::wasm_store_input_to_record)
                .collect(),
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
