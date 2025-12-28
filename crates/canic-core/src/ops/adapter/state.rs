use crate::{dto::state::AppModeView, model::memory::state::AppMode};

#[must_use]
pub fn app_mode_into_view(mode: AppMode) -> AppModeView {
    match mode {
        AppMode::Enabled => AppModeView::Enabled,
        AppMode::Readonly => AppModeView::Readonly,
        AppMode::Disabled => AppModeView::Disabled,
    }
}
