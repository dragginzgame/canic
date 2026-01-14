use crate::{
    InternalError,
    ops::{prelude::*, storage::StorageOpsError},
    storage::stable::state::app::{AppMode, AppState, AppStateData},
};
use thiserror::Error as ThisError;

///
/// AppStateCommand
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppStateCommand {
    Start,
    Readonly,
    Stop,
}

///
/// AppStateOpsError
///

#[derive(Debug, ThisError)]
pub enum AppStateOpsError {
    #[error("app is already in {0} mode")]
    AlreadyInMode(AppMode),
}

impl From<AppStateOpsError> for InternalError {
    fn from(err: AppStateOpsError) -> Self {
        StorageOpsError::from(err).into()
    }
}

///
/// AppStateOps
///

pub struct AppStateOps;

impl AppStateOps {
    // -------------------------------------------------------------
    // Getters
    // -------------------------------------------------------------

    #[must_use]
    pub(crate) fn get_mode() -> AppMode {
        AppState::get_mode()
    }

    // -------------------------------------------------------------
    // Commands
    // -------------------------------------------------------------

    pub fn execute_command(cmd: AppStateCommand) -> Result<(), InternalError> {
        let old_mode = AppState::get_mode();

        let new_mode = match cmd {
            AppStateCommand::Start => AppMode::Enabled,
            AppStateCommand::Readonly => AppMode::Readonly,
            AppStateCommand::Stop => AppMode::Disabled,
        };

        if old_mode == new_mode {
            return Err(AppStateOpsError::AlreadyInMode(old_mode).into());
        }

        AppState::set_mode(new_mode);

        log!(Topic::App, Ok, "app: mode changed {old_mode} -> {new_mode}");

        Ok(())
    }

    // -------------------------------------------------------------
    // Data / Import
    // -------------------------------------------------------------

    /// Export the current application state as an operational snapshot.
    #[must_use]
    pub fn data() -> AppStateData {
        AppState::export()
    }

    /// Import application state from an operational snapshot.
    ///
    /// Validation occurs during snapshot → data conversion.
    pub fn import(data: AppStateData) {
        AppState::import(data);
    }
}
