use crate::{
    dto::state::{AppModeView, AppStateView, SubnetStateView},
    ops::storage::state::{app::AppStateSnapshot, subnet::SubnetStateSnapshot},
    storage::memory::state::app::AppMode,
};

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
pub fn app_state_view_from_snapshot(snapshot: AppStateSnapshot) -> AppStateView {
    let mode = snapshot.mode.unwrap_or(AppMode::Disabled);
    AppStateView {
        mode: app_mode_to_view(mode),
    }
}

#[must_use]
pub fn app_state_snapshot_from_view(view: AppStateView) -> AppStateSnapshot {
    AppStateSnapshot {
        mode: Some(app_mode_from_view(view.mode)),
    }
}

#[must_use]
pub const fn subnet_state_view_from_snapshot(_: SubnetStateSnapshot) -> SubnetStateView {
    SubnetStateView {}
}

#[must_use]
pub const fn subnet_state_snapshot_from_view(_: SubnetStateView) -> SubnetStateSnapshot {
    SubnetStateSnapshot {}
}
