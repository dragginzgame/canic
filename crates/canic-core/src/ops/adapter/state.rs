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
