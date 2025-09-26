use crate::{
    Error, Log,
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    icu_eager_static, icu_memory, impl_storable_bounded, log,
    memory::{MemoryError, id::state::APP_STATE_ID},
};
use candid::CandidType;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

// APP_STATE
icu_eager_static! {
    static APP_STATE: RefCell<Cell<AppStateData, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            icu_memory!(AppState, APP_STATE_ID),
            AppStateData::default(),
        ));
}

///
/// AppStateError
///

#[derive(Debug, ThisError)]
pub enum AppStateError {
    #[error("app is already in {0} mode")]
    AlreadyInMode(AppMode),
}

///
/// AppMode
/// used for the query/update guards
/// Eventually we'll have more granularity overall
///

#[derive(
    CandidType, Clone, Copy, Debug, Default, Display, Eq, PartialEq, Serialize, Deserialize,
)]
pub enum AppMode {
    Enabled,
    Readonly,
    #[default]
    Disabled,
}

impl_storable_bounded!(AppStateData, 32, true);

///
/// AppCommand
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Display, Eq, PartialEq)]
pub enum AppCommand {
    Start,
    Readonly,
    Stop,
}

///
/// AppStateData
///

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppStateData {
    pub mode: AppMode,
}

///
/// AppState
///

pub struct AppState;

impl AppState {
    #[must_use]
    pub fn get_mode() -> AppMode {
        APP_STATE.with_borrow(|cell| cell.get().mode)
    }

    pub fn set_mode(mode: AppMode) {
        APP_STATE.with_borrow_mut(|cell| {
            let mut data = *cell.get();
            data.mode = mode;
            cell.set(data);
        });
    }

    pub fn command(cmd: AppCommand) -> Result<(), Error> {
        APP_STATE.with_borrow_mut(|cell| {
            let old_mode = cell.get().mode;

            let new_mode = match cmd {
                AppCommand::Start => AppMode::Enabled,
                AppCommand::Readonly => AppMode::Readonly,
                AppCommand::Stop => AppMode::Disabled,
            };

            if old_mode == new_mode {
                return Err(MemoryError::from(AppStateError::AlreadyInMode(old_mode)))?;
            }

            let mut data = *cell.get();
            data.mode = new_mode;
            cell.set(data);

            log!(Log::Ok, "app: mode changed {old_mode} -> {new_mode}");
            Ok(())
        })
    }

    pub fn import(data: AppStateData) {
        APP_STATE.with_borrow_mut(|cell| cell.set(data));
    }

    #[must_use]
    pub fn export() -> AppStateData {
        APP_STATE.with_borrow(|cell| *cell.get())
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_state(mode: AppMode) {
        AppState::import(AppStateData { mode });
    }

    #[test]
    fn default_mode_is_disabled() {
        reset_state(AppMode::Disabled);
        assert_eq!(AppState::get_mode(), AppMode::Disabled);
    }

    #[test]
    fn can_set_mode() {
        reset_state(AppMode::Disabled);

        AppState::set_mode(AppMode::Enabled);
        assert_eq!(AppState::get_mode(), AppMode::Enabled);

        AppState::set_mode(AppMode::Readonly);
        assert_eq!(AppState::get_mode(), AppMode::Readonly);
    }

    #[test]
    fn command_changes_modes() {
        reset_state(AppMode::Disabled);

        // Start command sets to Enabled
        assert!(AppState::command(AppCommand::Start).is_ok());
        assert_eq!(AppState::get_mode(), AppMode::Enabled);

        // Readonly command sets to Readonly
        assert!(AppState::command(AppCommand::Readonly).is_ok());
        assert_eq!(AppState::get_mode(), AppMode::Readonly);

        // Stop command sets to Disabled
        assert!(AppState::command(AppCommand::Stop).is_ok());
        assert_eq!(AppState::get_mode(), AppMode::Disabled);
    }

    #[test]
    fn duplicate_command_fails() {
        reset_state(AppMode::Enabled);

        // Sending Start again when already Enabled should error
        let err = AppState::command(AppCommand::Start)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("app is already in Enabled mode"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn import_and_export_state() {
        reset_state(AppMode::Disabled);

        let data = AppStateData {
            mode: AppMode::Readonly,
        };
        AppState::import(data);

        assert_eq!(AppState::export().mode, AppMode::Readonly);

        // After export we can reuse
        let exported = AppState::export();
        assert_eq!(exported, data);
    }
}
