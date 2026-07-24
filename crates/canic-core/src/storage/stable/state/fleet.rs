use crate::{
    cdk::structures::{DefaultMemoryImpl, cell::Cell, memory::VirtualMemory},
    role_contract::allocation::memory::env::FLEET_STATE_ID,
    storage::prelude::*,
};
use std::cell::RefCell;

pub use crate::domain::state::FleetMode;

//
// FLEET_STATE
//

eager_static! {
    static FLEET_STATE: RefCell<Cell<FleetStateRecord, VirtualMemory<DefaultMemoryImpl>>> =
        RefCell::new(Cell::init(
            crate::ic_memory_key!(authority = CANIC_CORE_MEMORY_AUTHORITY, key = "canic.core.fleet_state.v1", ty = FleetState, id = FLEET_STATE_ID),
            FleetStateRecord::default(),
        ));
}

///
/// FleetStateRecord
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FleetStateRecord {
    pub mode: FleetMode,
    pub cycles_funding_enabled: bool,
}

impl_storable_bounded!(FleetStateRecord, 32, true);

impl FleetStateRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "FleetStateRecord";
}

const fn default_cycles_funding_enabled() -> bool {
    true
}

impl Default for FleetStateRecord {
    fn default() -> Self {
        Self {
            mode: FleetMode::default(),
            cycles_funding_enabled: default_cycles_funding_enabled(),
        }
    }
}

///
/// FleetStateData
///
/// Canonical Fleet-state import/export snapshot.
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FleetStateData {
    pub record: FleetStateRecord,
}

impl FleetStateData {
    pub const STATE_CONTRACT_NAME: &'static str = "FleetStateData";
}

///
/// FleetState
///

pub struct FleetState;

impl FleetState {
    #[must_use]
    pub(crate) fn get_mode() -> FleetMode {
        FLEET_STATE.with_borrow(|cell| cell.get().mode)
    }

    pub(crate) fn set_mode(mode: FleetMode) {
        FLEET_STATE.with_borrow_mut(|cell| {
            let mut data = *cell.get();
            data.mode = mode;
            cell.set(data);
        });
    }

    #[must_use]
    pub(crate) fn cycles_funding_enabled() -> bool {
        FLEET_STATE.with_borrow(|cell| cell.get().cycles_funding_enabled)
    }

    pub(crate) fn set_cycles_funding_enabled(enabled: bool) {
        FLEET_STATE.with_borrow_mut(|cell| {
            let mut data = *cell.get();
            data.cycles_funding_enabled = enabled;
            cell.set(data);
        });
    }

    pub(crate) fn import(data: FleetStateData) {
        FLEET_STATE.with_borrow_mut(|cell| cell.set(data.record));
    }

    #[must_use]
    pub(crate) fn export() -> FleetStateData {
        FleetStateData {
            record: FLEET_STATE.with_borrow(|cell| *cell.get()),
        }
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdk::structures::storable::Storable;

    fn reset_state(mode: FleetMode) {
        FleetState::import(FleetStateData {
            record: FleetStateRecord {
                mode,
                cycles_funding_enabled: true,
            },
        });
    }

    #[test]
    fn default_mode_is_enabled() {
        FleetState::import(FleetStateData::default());
        assert_eq!(FleetState::get_mode(), FleetMode::Enabled);
    }

    #[test]
    fn can_set_mode() {
        reset_state(FleetMode::Disabled);

        FleetState::set_mode(FleetMode::Enabled);
        assert_eq!(FleetState::get_mode(), FleetMode::Enabled);

        FleetState::set_mode(FleetMode::Readonly);
        assert_eq!(FleetState::get_mode(), FleetMode::Readonly);
    }

    #[test]
    fn import_and_export_state() {
        reset_state(FleetMode::Disabled);

        let data = FleetStateRecord {
            mode: FleetMode::Readonly,
            cycles_funding_enabled: false,
        };
        FleetState::import(FleetStateData { record: data });

        assert_eq!(FleetState::export().record.mode, FleetMode::Readonly);
        assert!(!FleetState::export().record.cycles_funding_enabled);

        let exported = FleetState::export();
        assert_eq!(exported, FleetStateData { record: data });
    }

    #[test]
    fn fleet_state_record_storable_roundtrips_shared_fleet_mode() {
        let data = FleetStateRecord {
            mode: FleetMode::Readonly,
            cycles_funding_enabled: false,
        };

        let bytes = data.to_bytes();
        let decoded = FleetStateRecord::from_bytes(bytes);

        assert_eq!(decoded, data);
    }

    #[test]
    fn cycles_funding_switch_round_trip() {
        FleetState::import(FleetStateData::default());
        assert!(FleetState::cycles_funding_enabled());

        FleetState::set_cycles_funding_enabled(false);
        assert!(!FleetState::cycles_funding_enabled());

        FleetState::set_cycles_funding_enabled(true);
        assert!(FleetState::cycles_funding_enabled());
    }
}
