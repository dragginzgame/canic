//! Module: ops::storage::state::app
//!
//! Responsibility: execute deterministic app-state reads and mutations.
//! Does not own: endpoint authorization, workflow orchestration, or DTO schemas.
//! Boundary: storage ops facade over the stable app-state record.

use crate::{
    domain::state::AppStatus,
    dto::state::{
        AppCommand, AppCommandResponse, AppStateInput, AppStateResponse, SetStateResponse,
    },
    ops::{
        prelude::*,
        storage::state::mapper::{AppStateCommandMapper, AppStateMapper},
    },
    storage::stable::state::app::{AppMode, AppState, AppStateData, AppStateRecord},
};

///
/// AppStateCommand
///
/// Storage-ops command applied to the stable app-state record.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppStateCommand {
    SetStatus(AppStatus),
    SetCyclesFundingEnabled(bool),
}

///
/// AppStateOps
///
/// Storage-ops facade for app-state reads, mutations, and snapshots.
///

pub struct AppStateOps;

impl AppStateOps {
    // -------------------------------------------------------------------------
    // Getters
    // -------------------------------------------------------------------------

    #[must_use]
    pub(crate) fn get_mode() -> AppMode {
        AppState::get_mode()
    }

    #[must_use]
    pub(crate) fn is_query_allowed() -> bool {
        matches!(Self::get_mode(), AppMode::Enabled | AppMode::Readonly)
    }

    #[must_use]
    pub(crate) fn is_update_allowed() -> bool {
        matches!(Self::get_mode(), AppMode::Enabled)
    }

    #[must_use]
    pub(crate) fn is_readonly() -> bool {
        matches!(Self::get_mode(), AppMode::Readonly)
    }

    #[must_use]
    pub(crate) fn cycles_funding_enabled() -> bool {
        AppState::cycles_funding_enabled()
    }

    // -------------------------------------------------------------------------
    // Commands
    // -------------------------------------------------------------------------

    pub fn execute_command(cmd: AppStateCommand) -> AppCommandResponse {
        match cmd {
            AppStateCommand::SetStatus(status) => {
                let old_mode = AppState::get_mode();
                let previous = mode_to_status(old_mode);
                let new_mode = status_to_mode(status);
                let changed = old_mode != new_mode;

                if changed {
                    AppState::set_mode(new_mode);
                    log!(Topic::App, Ok, "app: mode changed {old_mode} -> {new_mode}");
                }

                AppCommandResponse::Status(SetStateResponse {
                    previous,
                    current: status,
                    changed,
                })
            }
            AppStateCommand::SetCyclesFundingEnabled(enabled) => {
                let old = AppState::cycles_funding_enabled();
                let changed = old != enabled;

                if changed {
                    AppState::set_cycles_funding_enabled(enabled);
                    log!(
                        Topic::App,
                        Ok,
                        "app: cycles_funding_enabled changed {old} -> {enabled}"
                    );
                }

                AppCommandResponse::CyclesFundingEnabled(SetStateResponse {
                    previous: old,
                    current: enabled,
                    changed,
                })
            }
        }
    }

    pub fn apply_command(cmd: AppCommand) -> AppCommandResponse {
        let internal = AppStateCommandMapper::dto_to_record(cmd);
        Self::execute_command(internal)
    }

    /// Initialize app state directly from configuration.
    ///
    /// This is intended for install-time bootstraps only.
    pub fn init_mode(mode: AppMode) {
        AppState::import(AppStateData {
            record: AppStateRecord {
                mode,
                cycles_funding_enabled: true,
            },
        });
    }

    // -------------------------------------------------------------
    // Data / Import
    // -------------------------------------------------------------

    /// Export the current application state as a DTO snapshot.
    #[must_use]
    pub fn snapshot_input() -> AppStateInput {
        AppStateMapper::record_to_input(AppState::export().record)
    }

    /// Export the current application state as a response snapshot.
    #[must_use]
    pub fn snapshot_response() -> AppStateResponse {
        AppStateMapper::record_to_response(AppState::export().record)
    }

    /// Import application state from an operational snapshot.
    ///
    /// Validation occurs during snapshot → data conversion.
    #[cfg_attr(not(test), expect(dead_code))]
    pub fn import(data: AppStateData) {
        AppState::import(data);
    }

    /// Import application state from a DTO snapshot.
    pub fn import_input(view: AppStateInput) {
        let record = AppStateMapper::input_to_record(view);
        AppState::import(AppStateData { record });
    }
}

const fn status_to_mode(status: AppStatus) -> AppMode {
    match status {
        AppStatus::Active => AppMode::Enabled,
        AppStatus::Readonly => AppMode::Readonly,
        AppStatus::Stopped => AppMode::Disabled,
    }
}

const fn mode_to_status(mode: AppMode) -> AppStatus {
    match mode {
        AppMode::Enabled => AppStatus::Active,
        AppMode::Readonly => AppStatus::Readonly,
        AppMode::Disabled => AppStatus::Stopped,
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_state(mode: AppMode, cycles_funding_enabled: bool) {
        AppStateOps::import(AppStateData {
            record: AppStateRecord {
                mode,
                cycles_funding_enabled,
            },
        });
    }

    #[test]
    fn set_status_changes_state_and_reports_previous_current() {
        reset_state(AppMode::Disabled, true);

        let response = AppStateOps::apply_command(AppCommand::SetStatus(AppStatus::Active));

        assert_eq!(AppStateOps::get_mode(), AppMode::Enabled);
        assert_eq!(
            response,
            AppCommandResponse::Status(SetStateResponse {
                previous: AppStatus::Stopped,
                current: AppStatus::Active,
                changed: true,
            })
        );
    }

    #[test]
    fn set_status_replay_returns_unchanged_success() {
        reset_state(AppMode::Enabled, true);

        let response = AppStateOps::apply_command(AppCommand::SetStatus(AppStatus::Active));

        assert_eq!(AppStateOps::get_mode(), AppMode::Enabled);
        assert_eq!(
            response,
            AppCommandResponse::Status(SetStateResponse {
                previous: AppStatus::Active,
                current: AppStatus::Active,
                changed: false,
            })
        );
    }

    #[test]
    fn set_cycles_funding_changes_state_and_reports_previous_current() {
        reset_state(AppMode::Enabled, true);

        let response = AppStateOps::apply_command(AppCommand::SetCyclesFundingEnabled(false));

        assert!(!AppStateOps::cycles_funding_enabled());
        assert_eq!(
            response,
            AppCommandResponse::CyclesFundingEnabled(SetStateResponse {
                previous: true,
                current: false,
                changed: true,
            })
        );
    }

    #[test]
    fn set_cycles_funding_replay_returns_unchanged_success() {
        reset_state(AppMode::Enabled, false);

        let response = AppStateOps::apply_command(AppCommand::SetCyclesFundingEnabled(false));

        assert!(!AppStateOps::cycles_funding_enabled());
        assert_eq!(
            response,
            AppCommandResponse::CyclesFundingEnabled(SetStateResponse {
                previous: false,
                current: false,
                changed: false,
            })
        );
    }
}
