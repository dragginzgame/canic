use crate::{
    Error, Log,
    cdk::structures::{DefaultMemoryImpl, Memory, cell::Cell, memory::VirtualMemory},
    icu_register_memory, impl_storable_unbounded, log,
    memory::{APP_STATE_MEMORY_ID, MemoryError},
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
    pub static APP_STATE: RefCell<AppStateCore<VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(AppStateCore::new(Cell::init(
            icu_register_memory!(APP_STATE_MEMORY_ID),
            AppStateData::default(),
        )));
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
/// AppState
///

pub struct AppState;

impl AppState {
    #[must_use]
    pub fn get_mode() -> AppMode {
        APP_STATE.with_borrow(AppStateCore::get_mode)
    }

    pub fn set_mode(mode: AppMode) {
        APP_STATE.with_borrow_mut(|core| core.set_mode(mode));
    }

    pub fn command(cmd: AppCommand) -> Result<(), Error> {
        APP_STATE.with_borrow_mut(|core| core.command(cmd))
    }

    pub fn import(data: AppStateData) {
        APP_STATE.with_borrow_mut(|core| core.import(data));
    }

    #[must_use]
    pub fn export() -> AppStateData {
        APP_STATE.with_borrow(AppStateCore::export)
    }
}

///
/// AppStateData
///

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppStateData {
    mode: AppMode,
}

impl_storable_unbounded!(AppStateData);

///
/// AppStateCore
///

pub struct AppStateCore<M: Memory> {
    cell: Cell<AppStateData, M>,
}

impl<M: Memory> AppStateCore<M> {
    pub const fn new(cell: Cell<AppStateData, M>) -> Self {
        Self { cell }
    }

    pub fn get_mode(&self) -> AppMode {
        self.cell.get().mode
    }

    pub fn set_mode(&mut self, mode: AppMode) {
        let mut cur = *self.cell.get(); // copy out the value
        cur.mode = mode;
        self.cell.set(cur);
    }

    pub fn command(&mut self, cmd: AppCommand) -> Result<(), Error> {
        let old_mode = self.cell.get().mode;

        let new_mode = match cmd {
            AppCommand::Start => AppMode::Enabled,
            AppCommand::Readonly => AppMode::Readonly,
            AppCommand::Stop => AppMode::Disabled,
        };

        if old_mode == new_mode {
            return Err(MemoryError::from(AppStateError::AlreadyInMode(old_mode)))?;
        }

        self.set_mode(new_mode);

        log!(Log::Ok, "app: mode changed {old_mode} -> {new_mode}");
        Ok(())
    }

    pub fn import(&mut self, data: AppStateData) {
        self.cell.set(data);
    }

    pub fn export(&self) -> AppStateData {
        *self.cell.get()
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    fn core() -> AppStateCore<DefaultMemoryImpl> {
        let cell = Cell::init(DefaultMemoryImpl::default(), AppStateData::default());
        AppStateCore::new(cell)
    }

    #[test]
    fn default_mode_is_disabled() {
        let core = core();
        assert_eq!(core.get_mode(), AppMode::Disabled);
    }

    #[test]
    fn can_set_mode() {
        let mut core = core();
        core.set_mode(AppMode::Enabled);
        assert_eq!(core.get_mode(), AppMode::Enabled);

        core.set_mode(AppMode::Readonly);
        assert_eq!(core.get_mode(), AppMode::Readonly);
    }

    #[test]
    fn command_changes_modes() {
        let mut core = core();

        // Start command sets to Enabled
        assert!(core.command(AppCommand::Start).is_ok());
        assert_eq!(core.get_mode(), AppMode::Enabled);

        // Readonly command sets to Readonly
        assert!(core.command(AppCommand::Readonly).is_ok());
        assert_eq!(core.get_mode(), AppMode::Readonly);

        // Stop command sets to Disabled
        assert!(core.command(AppCommand::Stop).is_ok());
        assert_eq!(core.get_mode(), AppMode::Disabled);
    }

    #[test]
    fn duplicate_command_fails() {
        let mut core = core();
        core.set_mode(AppMode::Enabled);

        // Sending Start again when already Enabled should error
        let err = core.command(AppCommand::Start).unwrap_err().to_string();
        assert!(
            err.contains("app is already in Enabled mode"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn import_and_export_state() {
        let mut core = core();
        let data = AppStateData {
            mode: AppMode::Readonly,
        };

        core.import(data);
        assert_eq!(core.export().mode, AppMode::Readonly);

        // After export we can reuse
        let exported = core.export();
        assert_eq!(exported, data);
    }
}
