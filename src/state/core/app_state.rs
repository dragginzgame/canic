use crate::{
    Log,
    ic::structures::{Cell, DefaultMemory, cell::CellError, memory::MemoryId},
    impl_storable_unbounded, log,
    state::{APP_STATE_MEMORY_ID, MEMORY_MANAGER},
};
use candid::CandidType;
use derive_more::{Deref, DerefMut, Display};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
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

//
// APP_STATE
//
// a Cell that's only really meant for small data structures used for global app state
//
// defaults to Enabled as then it's possible for non-controllers to call
// endpoints in order to initialise
//

thread_local! {
    pub static APP_STATE: RefCell<AppState> = RefCell::new(AppState::init(
        MEMORY_MANAGER.with_borrow(|this| this.get(MemoryId::new(APP_STATE_MEMORY_ID))),
        AppMode::Enabled,
    ));
}

///
/// AppState
///

#[derive(Deref, DerefMut)]
pub struct AppState(Cell<AppStateData>);

impl AppState {
    // init
    #[must_use]
    pub fn init(memory: DefaultMemory, mode: AppMode) -> Self {
        let cell = Cell::init(memory, AppStateData { mode }).unwrap();

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

#[derive(CandidType, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct AppStateData {
    mode: AppMode,
}

impl_storable_unbounded!(AppStateData);

///
/// Command
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

#[derive(CandidType, Clone, Copy, Debug, Display, Eq, PartialEq, Serialize, Deserialize)]
pub enum AppMode {
    Enabled,
    Readonly,
    Disabled,
}
