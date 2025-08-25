use crate::{
    Error,
    interface::prelude::*,
    memory::{AppState, AppStateData, CanisterChildren, CanisterDirectory, CanisterDirectoryView},
};

///
/// StateBundle
///

#[derive(CandidType, Debug, Default, Deserialize)]
pub struct StateBundle {
    app_state: Option<AppStateData>,
    canister_directory: Option<CanisterDirectoryView>,
}

impl StateBundle {
    #[must_use]
    pub fn all() -> Self {
        Self {
            app_state: Some(AppState::export()),
            canister_directory: Some(CanisterDirectory::export()),
        }
    }

    #[must_use]
    pub fn app_state() -> Self {
        Self {
            app_state: Some(AppState::export()),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn canister_directory() -> Self {
        Self {
            canister_directory: Some(CanisterDirectory::export()),
            ..Default::default()
        }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.app_state.is_none() && self.canister_directory.is_none()
    }

    fn debug(&self) -> String {
        let mut debug_str = String::new();

        if self.app_state.is_some() {
            debug_str.push('a');
        }
        if self.canister_directory.is_some() {
            debug_str.push('s');
        }

        debug_str
    }
}

// save_state
pub fn save_state(bundle: &StateBundle) {
    if let Some(data) = &bundle.app_state {
        AppState::import(*data);
    }
    if let Some(data) = &bundle.canister_directory {
        CanisterDirectory::import(data.clone());
    }
}

// cascade
pub async fn cascade(bundle: &StateBundle) -> Result<(), Error> {
    for (pid, _) in CanisterChildren::export() {
        cascade_canister(&pid, bundle).await?;
    }

    Ok(())
}

// cascade_canister
pub async fn cascade_canister(pid: &Principal, bundle: &StateBundle) -> Result<(), Error> {
    let debug_str = &bundle.debug();

    log!(Log::Info, "💦 state.cascade: [{debug_str}] -> {pid}");

    Call::unbounded_wait(*pid, "icu_state_cascade")
        .with_arg(bundle)
        .await
        .map_err(InterfaceError::from)?;

    Ok(())
}

// update_canister
pub async fn update_canister(pid: &Principal, bundle: &StateBundle) -> Result<(), Error> {
    let debug_str = &bundle.debug();

    log!(Log::Info, "🔄 state.update: [{debug_str}] -> {pid}");

    Call::unbounded_wait(*pid, "icu_state_update")
        .with_arg(bundle)
        .await
        .map_err(InterfaceError::from)?;

    Ok(())
}
