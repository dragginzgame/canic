use crate::{
    dto::state::{AppModeView, AppStateView, SubnetStateView},
    ops::storage::state::{
        app::{AppMode, AppStateSnapshot},
        subnet::SubnetStateSnapshot,
    },
};

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
    pub fn snapshot_to_view(snapshot: AppStateSnapshot) -> AppStateView {
        let mode = snapshot.mode.unwrap_or(AppMode::Disabled);
        AppStateView {
            mode: Self::app_mode_to_view(mode),
        }
    }

    #[must_use]
    pub const fn view_to_snapshot(view: AppStateView) -> AppStateSnapshot {
        AppStateSnapshot {
            mode: Some(Self::app_mode_from_view(view.mode)),
        }
    }
}

///
/// SubnetStateMapper
///

pub struct SubnetStateMapper;

impl SubnetStateMapper {
    #[must_use]
    pub const fn snapshot_to_view(_: SubnetStateSnapshot) -> SubnetStateView {
        SubnetStateView {}
    }

    #[must_use]
    pub const fn view_to_snapshot(_: SubnetStateView) -> SubnetStateSnapshot {
        SubnetStateSnapshot {}
    }
}
