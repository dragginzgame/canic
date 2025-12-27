use crate::{
    Error, ThisError,
    dto::state::{AppCommand, AppStateView},
    log,
    log::Topic,
    model::memory::state::{AppMode, AppState, AppStateData},
    ops::storage::state::StateOpsError,
};

///
/// AppStateOpsError
///

#[derive(Debug, ThisError)]
pub enum AppStateOpsError {
    #[error("app is already in {0} mode")]
    AlreadyInMode(AppMode),
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
    #[must_use]
    pub fn get_mode() -> AppMode {
        AppState::get_mode()
    }

    pub fn set_mode(mode: AppMode) {
        AppState::set_mode(mode);
    }

    pub fn command(cmd: AppCommand) -> Result<(), Error> {
        let old_mode = AppState::get_mode();

        let new_mode = match cmd {
            AppCommand::Start => AppMode::Enabled,
            AppCommand::Readonly => AppMode::Readonly,
            AppCommand::Stop => AppMode::Disabled,
        };

        if old_mode == new_mode {
            return Err(AppStateOpsError::AlreadyInMode(old_mode))?;
        }

        AppState::set_mode(new_mode);

        log!(Topic::App, Ok, "app: mode changed {old_mode} -> {new_mode}");

        Ok(())
    }

    /// Import app state from a public view.
    pub fn import(view: AppStateView) {
        let data: AppStateData = view.into();
        AppState::import(data);
    }

    /// Export app state as a public view.
    #[must_use]
    pub fn export() -> AppStateView {
        let data: AppStateData = AppState::export();
        data.into()
    }
}

///
/// Adapter
///

impl From<AppStateData> for AppStateView {
    fn from(d: AppStateData) -> Self {
        Self { mode: d.mode }
    }
}

impl From<AppStateView> for AppStateData {
    fn from(v: AppStateView) -> Self {
        Self { mode: v.mode }
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn reset_state(mode: AppMode) {
        Config::reset_for_tests();
        let _ = Config::init_for_tests();
        AppStateOps::import(AppStateView { mode });
    }

    #[test]
    fn command_changes_modes() {
        reset_state(AppMode::Disabled);

        assert!(AppStateOps::command(AppCommand::Start).is_ok());
        assert_eq!(AppStateOps::get_mode(), AppMode::Enabled);

        assert!(AppStateOps::command(AppCommand::Readonly).is_ok());
        assert_eq!(AppStateOps::get_mode(), AppMode::Readonly);

        assert!(AppStateOps::command(AppCommand::Stop).is_ok());
        assert_eq!(AppStateOps::get_mode(), AppMode::Disabled);
    }

    #[test]
    fn duplicate_command_fails() {
        reset_state(AppMode::Enabled);

        let err = AppStateOps::command(AppCommand::Start)
            .unwrap_err()
            .to_string();

        assert!(
            err.contains("app is already in Enabled mode"),
            "unexpected error: {err}"
        );
    }
}
