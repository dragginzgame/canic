use crate::{
    Log, ic::structures::Cell, icu_register_memory, impl_storable_unbounded, log,
    memory::APP_STATE_MEMORY_ID,
};
use candid::CandidType;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use thiserror::Error as ThisError;

//
// APP_STATE
//

thread_local! {
    pub static APP_STATE: RefCell<Cell<AppStateData>> = RefCell::new(Cell::init(
        icu_register_memory!(AppStateData, APP_STATE_MEMORY_ID),
        AppStateData::default(),
    ));
}

///
/// AppStateError
///

#[derive(CandidType, Debug, Deserialize, Serialize, ThisError)]
pub enum AppStateError {
    #[error("app is already in {0} mode")]
    AlreadyInMode(AppMode),
}

///
/// AppState
///

pub struct AppState();

impl AppState {
    pub fn with<R>(f: impl FnOnce(&Cell<AppStateData>) -> R) -> R {
        APP_STATE.with_borrow(|s| f(s))
    }

    pub fn with_mut<R>(f: impl FnOnce(&mut Cell<AppStateData>) -> R) -> R {
        APP_STATE.with_borrow_mut(|s| f(s))
    }

    #[must_use]
    pub fn get_data() -> AppStateData {
        Self::with(Cell::get)
    }

    // set_data
    pub fn set_data(data: AppStateData) {
        Self::with_mut(|cell| cell.set(data));
    }

    #[must_use]
    pub fn get_mode() -> AppMode {
        Self::with(|cell| cell.get().mode)
    }

    pub fn set_mode(mode: AppMode) {
        Self::with_mut(|cell| {
            let mut cur = cell.get();
            cur.mode = mode;

            cell.set(cur);
        });
    }

    // command
    pub fn command(cmd: AppCommand) -> Result<(), AppStateError> {
        let old_mode = Self::with(|cell| cell.get().mode);

        let new_mode = match cmd {
            AppCommand::Start => AppMode::Enabled,
            AppCommand::Readonly => AppMode::Readonly,
            AppCommand::Stop => AppMode::Disabled,
        };

        if old_mode == new_mode {
            return Err(AppStateError::AlreadyInMode(old_mode));
        }

        Self::set_mode(new_mode);

        log!(Log::Ok, "app: mode changed {old_mode} -> {new_mode}");

        Ok(())
    }
}

///
/// AppStateData
///

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct AppStateData {
    mode: AppMode,
}

impl_storable_unbounded!(AppStateData);

///
/// AppCommand
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Display, Eq, PartialEq, Serialize)]
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
