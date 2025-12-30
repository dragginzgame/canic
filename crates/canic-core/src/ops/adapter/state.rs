use crate::{
    dto::state::{AppModeView, AppStateView, SubnetStateView},
    model::memory::state::{AppMode, AppStateData, SubnetStateData},
};

#[must_use]
pub const fn app_mode_into_view(mode: AppMode) -> AppModeView {
    match mode {
        AppMode::Enabled => AppModeView::Enabled,
        AppMode::Readonly => AppModeView::Readonly,
        AppMode::Disabled => AppModeView::Disabled,
    }
}

#[must_use]
pub const fn app_state_to_view(data: AppStateData) -> AppStateView {
    AppStateView {
        mode: app_mode_into_view(data.mode),
    }
}

#[must_use]
pub const fn subnet_state_to_view(_: SubnetStateData) -> SubnetStateView {
    SubnetStateView {}
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
pub const fn app_state_from_view(view: AppStateView) -> AppStateData {
    AppStateData {
        mode: app_mode_from_view(view.mode),
    }
}

#[must_use]
pub const fn subnet_state_from_view(_: SubnetStateView) -> SubnetStateData {
    SubnetStateData {}
}
