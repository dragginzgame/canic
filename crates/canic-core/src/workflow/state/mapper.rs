use crate::{
    dto::state::{AppCommand, AppModeView, AppStateView, SubnetStateView},
    ops::storage::state::app::AppStateCommand,
    storage::stable::state::{
        app::{AppMode, AppStateData},
        subnet::SubnetStateData,
    },
};

///
/// AppCommandMapper
///

pub struct AppCommandMapper;

impl AppCommandMapper {
    #[must_use]
    pub const fn from_dto(cmd: AppCommand) -> AppStateCommand {
        match cmd {
            AppCommand::Start => AppStateCommand::Start,
            AppCommand::Readonly => AppStateCommand::Readonly,
            AppCommand::Stop => AppStateCommand::Stop,
        }
    }
}

///
/// AppStateMapper
///

pub struct AppStateMapper;

impl AppStateMapper {
    #[must_use]
    pub const fn app_mode_to_view(mode: AppMode) -> AppModeView {
        match mode {
            AppMode::Enabled => AppModeView::Enabled,
            AppMode::Readonly => AppModeView::Readonly,
            AppMode::Disabled => AppModeView::Disabled,
        }
    }

    #[must_use]
    pub const fn app_mode_from_view(mode: AppModeView) -> AppMode {
        match mode {
            AppModeView::Enabled => AppMode::Enabled,
            AppModeView::Readonly => AppMode::Readonly,
            AppModeView::Disabled => AppMode::Disabled,
        }
    }

    #[must_use]
    pub const fn data_to_view(data: AppStateData) -> AppStateView {
        AppStateView {
            mode: Self::app_mode_to_view(data.mode),
        }
    }

    #[must_use]
    pub const fn view_to_data(view: AppStateView) -> AppStateData {
        AppStateData {
            mode: Self::app_mode_from_view(view.mode),
        }
    }
}

///
/// SubnetStateMapper
///

pub struct SubnetStateMapper;

impl SubnetStateMapper {
    #[must_use]
    pub const fn data_to_view(_: SubnetStateData) -> SubnetStateView {
        SubnetStateView {}
    }

    #[must_use]
    pub const fn view_to_data(_: SubnetStateView) -> SubnetStateData {
        SubnetStateData {}
    }
}
