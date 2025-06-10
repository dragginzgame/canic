use crate::{
    Log,
    ic::structures::{Cell, DefaultMemory, cell::CellError},
    impl_storable_unbounded, log,
};
use candid::CandidType;
use derive_more::{Deref, DerefMut, Display};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// AppStateError
///

#[derive(CandidType, Debug, Serialize, Deserialize, ThisError)]
pub enum AppStateError {
    #[error("app is already in {0} mode")]
    AlreadyInMode(AppMode),

    #[error(transparent)]
    CellError(#[from] CellError),
}

///
/// AppState
///

#[derive(Deref, DerefMut)]
pub struct AppState(Cell<AppStateData>);

impl AppState {
    // init
    #[must_use]
    pub fn init(memory: DefaultMemory) -> Self {
        let cell = Cell::init(memory, AppStateData::default()).unwrap();

        Self(cell)
    }

    // get_data
    #[must_use]
    pub fn get_data(&self) -> AppStateData {
        self.get()
    }

    // set_data
    pub fn set_data(&mut self, data: AppStateData) -> Result<(), AppStateError> {
        self.set(data)?;

        Ok(())
    }

    // get_mode
    #[must_use]
    pub fn get_mode(&self) -> AppMode {
        self.get().mode
    }

    // set_mode
    pub fn set_mode(&mut self, mode: AppMode) -> Result<(), AppStateError> {
        let mut cur_state = self.get();
        cur_state.mode = mode;
        self.set(cur_state)?;

        Ok(())
    }

    // command
    pub fn command(&mut self, cmd: AppCommand) -> Result<(), AppStateError> {
        let old_mode = self.get().mode;
        let new_mode = match cmd {
            AppCommand::Start => AppMode::Enabled,
            AppCommand::Readonly => AppMode::Readonly,
            AppCommand::Stop => AppMode::Disabled,
        };

        // already in mode?
        if old_mode == new_mode {
            Err(AppStateError::AlreadyInMode(old_mode))?;
        }

        // set mode
        self.set_mode(new_mode)?;

        log!(Log::Ok, "app: mode changed {old_mode} -> {new_mode}");

        Ok(())
    }
}

///
/// AppStateData
///

#[derive(CandidType, Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct AppStateData {
    mode: AppMode,
}

impl_storable_unbounded!(AppStateData);

///
/// AppCommand
///

#[derive(CandidType, Clone, Copy, Debug, Display, Eq, PartialEq, Serialize, Deserialize)]
pub enum AppCommand {
    Start,
    Readonly,
    Stop,
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
