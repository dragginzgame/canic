use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    eager_static, ic_memory,
    model::memory::id::state::APP_STATE_ID,
    utils::impl_storable_bounded,
};
use candid::CandidType;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

//
// APP_STATE
//

eager_static! {
    static APP_STATE: RefCell<Cell<AppStateData, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            ic_memory!(AppState, APP_STATE_ID),
            AppStateData::default(),
        ));
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
/// AppStateData
///

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppStateData {
    pub mode: AppMode,
}

impl_storable_bounded!(AppStateData, 32, true);

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

    pub(crate) fn import(data: AppStateData) {
        APP_STATE.with_borrow_mut(|cell| cell.set(data));
    }

    #[must_use]
    pub(crate) fn export() -> AppStateData {
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
