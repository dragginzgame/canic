//! Module: ops::storage::state::fleet
//!
//! Responsibility: execute deterministic Fleet-state reads and mutations.
//! Does not own: endpoint authorization, workflow orchestration, or DTO schemas.
//! Boundary: storage ops facade over the stable Fleet-state record.

use crate::{
    domain::state::FleetStatus,
    dto::state::{
        FleetCommand, FleetCommandResponse, FleetStateInput, FleetStateResponse, SetStateResponse,
    },
    ops::{
        prelude::*,
        storage::state::mapper::{FleetStateCommandMapper, FleetStateMapper},
    },
    storage::stable::state::fleet::{FleetMode, FleetState, FleetStateData, FleetStateRecord},
};

///
/// FleetStateCommand
///
/// Storage-ops command applied to the stable Fleet-state record.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetStateCommand {
    SetStatus(FleetStatus),
    SetCyclesFundingEnabled(bool),
}

///
/// FleetStateOps
///
/// Storage-ops facade for Fleet-state reads, mutations, and snapshots.
///

pub struct FleetStateOps;

impl FleetStateOps {
    // -------------------------------------------------------------------------
    // Getters
    // -------------------------------------------------------------------------

    #[must_use]
    pub(crate) fn get_mode() -> FleetMode {
        FleetState::get_mode()
    }

    #[must_use]
    pub(crate) fn is_query_allowed() -> bool {
        matches!(Self::get_mode(), FleetMode::Enabled | FleetMode::Readonly)
    }

    #[must_use]
    pub(crate) fn is_update_allowed() -> bool {
        matches!(Self::get_mode(), FleetMode::Enabled)
    }

    #[must_use]
    pub(crate) fn is_readonly() -> bool {
        matches!(Self::get_mode(), FleetMode::Readonly)
    }

    #[must_use]
    pub(crate) fn cycles_funding_enabled() -> bool {
        FleetState::cycles_funding_enabled()
    }

    // -------------------------------------------------------------------------
    // Commands
    // -------------------------------------------------------------------------

    pub fn execute_command(cmd: FleetStateCommand) -> FleetCommandResponse {
        match cmd {
            FleetStateCommand::SetStatus(status) => {
                let old_mode = FleetState::get_mode();
                let previous = mode_to_status(old_mode);
                let new_mode = status_to_mode(status);
                let changed = old_mode != new_mode;

                if changed {
                    FleetState::set_mode(new_mode);
                    log!(
                        Topic::Fleet,
                        Ok,
                        "fleet: mode changed {old_mode} -> {new_mode}"
                    );
                }

                FleetCommandResponse::Status(SetStateResponse {
                    previous,
                    current: status,
                    changed,
                })
            }
            FleetStateCommand::SetCyclesFundingEnabled(enabled) => {
                let old = FleetState::cycles_funding_enabled();
                let changed = old != enabled;

                if changed {
                    FleetState::set_cycles_funding_enabled(enabled);
                    log!(
                        Topic::Fleet,
                        Ok,
                        "fleet: cycles_funding_enabled changed {old} -> {enabled}"
                    );
                }

                FleetCommandResponse::CyclesFundingEnabled(SetStateResponse {
                    previous: old,
                    current: enabled,
                    changed,
                })
            }
        }
    }

    pub fn apply_command(cmd: FleetCommand) -> FleetCommandResponse {
        let internal = FleetStateCommandMapper::dto_to_record(cmd);
        Self::execute_command(internal)
    }

    /// Initialize Fleet state directly from App configuration.
    ///
    /// This is intended for install-time bootstraps only.
    pub fn init_mode(mode: FleetMode) {
        FleetState::import(FleetStateData {
            record: FleetStateRecord {
                mode,
                cycles_funding_enabled: true,
            },
        });
    }

    // -------------------------------------------------------------
    // Data / Import
    // -------------------------------------------------------------

    /// Export the current Fleet state as a DTO snapshot.
    #[must_use]
    pub fn snapshot_input() -> FleetStateInput {
        FleetStateMapper::record_to_input(FleetState::export().record)
    }

    /// Export the current Fleet state as a response snapshot.
    #[must_use]
    pub fn snapshot_response() -> FleetStateResponse {
        FleetStateMapper::record_to_response(FleetState::export().record)
    }

    /// Import Fleet state from an operational snapshot for unit tests.
    #[cfg(test)]
    pub fn import(data: FleetStateData) {
        FleetState::import(data);
    }

    /// Import Fleet state from a DTO snapshot.
    pub fn import_input(view: FleetStateInput) {
        let record = FleetStateMapper::input_to_record(view);
        FleetState::import(FleetStateData { record });
    }
}

const fn status_to_mode(status: FleetStatus) -> FleetMode {
    match status {
        FleetStatus::Active => FleetMode::Enabled,
        FleetStatus::Readonly => FleetMode::Readonly,
        FleetStatus::Stopped => FleetMode::Disabled,
    }
}

const fn mode_to_status(mode: FleetMode) -> FleetStatus {
    match mode {
        FleetMode::Enabled => FleetStatus::Active,
        FleetMode::Readonly => FleetStatus::Readonly,
        FleetMode::Disabled => FleetStatus::Stopped,
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_state(mode: FleetMode, cycles_funding_enabled: bool) {
        FleetStateOps::import(FleetStateData {
            record: FleetStateRecord {
                mode,
                cycles_funding_enabled,
            },
        });
    }

    #[test]
    fn set_status_changes_state_and_reports_previous_current() {
        reset_state(FleetMode::Disabled, true);

        let response = FleetStateOps::apply_command(FleetCommand::SetStatus(FleetStatus::Active));

        assert_eq!(FleetStateOps::get_mode(), FleetMode::Enabled);
        assert_eq!(
            response,
            FleetCommandResponse::Status(SetStateResponse {
                previous: FleetStatus::Stopped,
                current: FleetStatus::Active,
                changed: true,
            })
        );
    }

    #[test]
    fn set_status_replay_returns_unchanged_success() {
        reset_state(FleetMode::Enabled, true);

        let response = FleetStateOps::apply_command(FleetCommand::SetStatus(FleetStatus::Active));

        assert_eq!(FleetStateOps::get_mode(), FleetMode::Enabled);
        assert_eq!(
            response,
            FleetCommandResponse::Status(SetStateResponse {
                previous: FleetStatus::Active,
                current: FleetStatus::Active,
                changed: false,
            })
        );
    }

    #[test]
    fn set_cycles_funding_changes_state_and_reports_previous_current() {
        reset_state(FleetMode::Enabled, true);

        let response = FleetStateOps::apply_command(FleetCommand::SetCyclesFundingEnabled(false));

        assert!(!FleetStateOps::cycles_funding_enabled());
        assert_eq!(
            response,
            FleetCommandResponse::CyclesFundingEnabled(SetStateResponse {
                previous: true,
                current: false,
                changed: true,
            })
        );
    }

    #[test]
    fn set_cycles_funding_replay_returns_unchanged_success() {
        reset_state(FleetMode::Enabled, false);

        let response = FleetStateOps::apply_command(FleetCommand::SetCyclesFundingEnabled(false));

        assert!(!FleetStateOps::cycles_funding_enabled());
        assert_eq!(
            response,
            FleetCommandResponse::CyclesFundingEnabled(SetStateResponse {
                previous: false,
                current: false,
                changed: false,
            })
        );
    }
}
