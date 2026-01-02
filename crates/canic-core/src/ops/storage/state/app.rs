use crate::{
    Error, ThisError,
    dto::state::AppCommand,
    ops::{prelude::*, storage::state::StateOpsError},
    storage::memory::state::app::{AppMode as ModelAppMode, AppState, AppStateData},
};
use derive_more::Display;

///
/// AppStateSnapshot
/// Internal, operational snapshot of application state.
///
/// - Used by workflows and state cascades
/// - May be partially populated in the future
/// - Not serialized or exposed externally
///

#[derive(Clone, Debug, Default)]
pub struct AppStateSnapshot {
    pub mode: Option<AppMode>,
}

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq)]
pub enum AppMode {
    Enabled,
    Readonly,
    Disabled,
}

impl From<AppStateData> for AppStateSnapshot {
    fn from(data: AppStateData) -> Self {
        Self {
            mode: Some(AppMode::from_model(data.mode)),
        }
    }
}

impl TryFrom<AppStateSnapshot> for AppStateData {
    type Error = AppStateOpsError;

    fn try_from(snapshot: AppStateSnapshot) -> Result<Self, Self::Error> {
        let Some(mode) = snapshot.mode else {
            return Err(AppStateOpsError::MissingField("mode"));
        };

        Ok(Self {
            mode: AppMode::to_model(mode),
        })
    }
}

///
/// AppStateOpsError
///

#[derive(Debug, ThisError)]
pub enum AppStateOpsError {
    #[error("app is already in {0} mode")]
    AlreadyInMode(AppMode),

    #[error("app state snapshot missing required field: {0}")]
    MissingField(&'static str),
}

impl From<AppStateOpsError> for Error {
    fn from(err: AppStateOpsError) -> Self {
        StateOpsError::from(err).into()
    }
}

///
/// AppStateOps
///

pub struct AppStateOps;

impl AppStateOps {
    // -------------------------------------------------------------
    // Commands
    // -------------------------------------------------------------

    pub fn command(cmd: AppCommand) -> Result<(), Error> {
        let old_mode = AppMode::from_model(AppState::get_mode());

        let new_mode = match cmd {
            AppCommand::Start => AppMode::Enabled,
            AppCommand::Readonly => AppMode::Readonly,
            AppCommand::Stop => AppMode::Disabled,
        };

        if old_mode == new_mode {
            return Err(AppStateOpsError::AlreadyInMode(old_mode).into());
        }

        AppState::set_mode(AppMode::to_model(new_mode));

        log!(Topic::App, Ok, "app: mode changed {old_mode} -> {new_mode}");

        Ok(())
    }

    // -------------------------------------------------------------
    // Snapshot / Import
    // -------------------------------------------------------------

    /// Export the current application state as an operational snapshot.
    #[must_use]
    pub fn snapshot() -> AppStateSnapshot {
        AppState::export().into()
    }

    /// Import application state from an operational snapshot.
    ///
    /// Validation occurs during snapshot → data conversion.
    pub fn import(snapshot: AppStateSnapshot) -> Result<(), Error> {
        let data: AppStateData = snapshot.try_into()?;
        AppState::import(data);

        Ok(())
    }
}

impl AppMode {
    const fn from_model(mode: ModelAppMode) -> Self {
        match mode {
            ModelAppMode::Enabled => Self::Enabled,
            ModelAppMode::Readonly => Self::Readonly,
            ModelAppMode::Disabled => Self::Disabled,
        }
    }

    const fn to_model(mode: Self) -> ModelAppMode {
        match mode {
            Self::Enabled => ModelAppMode::Enabled,
            Self::Readonly => ModelAppMode::Readonly,
            Self::Disabled => ModelAppMode::Disabled,
        }
    }
}
