use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    storage::{prelude::*, stable::memory::state::APP_STATE_ID},
};
use std::{
    cell::RefCell,
    fmt::{self, Display},
};

//
// APP_STATE
//

eager_static! {
    static APP_STATE: RefCell<Cell<AppStateRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            ic_memory!(AppState, APP_STATE_ID),
            AppStateRecord::default(),
        ));
}

///
/// AppMode
/// Application mode used by query/update guards.
///

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum AppMode {
    #[default]
    Enabled,
    Readonly,
    Disabled,
}

impl Display for AppMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Enabled => "Enabled",
            Self::Readonly => "Readonly",
            Self::Disabled => "Disabled",
        };

        f.write_str(label)
    }
}

///
/// AppStateRecord
///

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppStateRecord {
    pub mode: AppMode,
}

impl_storable_bounded!(AppStateRecord, 32, true);

///
/// AppState
///

pub struct AppState;

impl AppState {
    #[must_use]
    pub(crate) fn get_mode() -> AppMode {
        APP_STATE.with_borrow(|cell| cell.get().mode)
    }

    pub(crate) fn set_mode(mode: AppMode) {
        APP_STATE.with_borrow_mut(|cell| {
            let mut data = *cell.get();
            data.mode = mode;
            cell.set(data);
        });
    }

    pub(crate) fn import(data: AppStateRecord) {
        APP_STATE.with_borrow_mut(|cell| cell.set(data));
    }

    #[must_use]
    pub(crate) fn export() -> AppStateRecord {
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
        AppState::import(AppStateRecord { mode });
    }

    #[test]
    fn default_mode_is_enabled() {
        AppState::import(AppStateRecord::default());
        assert_eq!(AppState::get_mode(), AppMode::Enabled);
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
    fn import_and_export_state() {
        reset_state(AppMode::Disabled);

        let data = AppStateRecord {
            mode: AppMode::Readonly,
        };
        AppState::import(data);

        assert_eq!(AppState::export().mode, AppMode::Readonly);

        let exported = AppState::export();
        assert_eq!(exported, data);
    }
}
