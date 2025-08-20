use crate::{
    Error, Log,
    ic::{api::canister_self, call::Call},
    interface::{InterfaceError, ic::IcError},
    log,
    memory::{
        AppState, AppStateData, CanisterState, ChildIndex, SubnetDirectory, SubnetDirectoryView,
    },
};
use candid::{CandidType, Principal};
use serde::Deserialize;

///
/// StateBundle
///

#[derive(CandidType, Debug, Default, Deserialize)]
pub struct StateBundle {
    app_state: Option<AppStateData>,
    subnet_directory: Option<SubnetDirectoryView>,
}

impl StateBundle {
    #[must_use]
    pub fn all() -> Self {
        Self {
            app_state: Some(AppState::export()),
            subnet_directory: Some(SubnetDirectory::export()),
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
    pub fn subnet_directory() -> Self {
        Self {
            subnet_directory: Some(SubnetDirectory::export()),
            ..Default::default()
        }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.app_state.is_none() && self.subnet_directory.is_none()
    }

    fn debug(&self) -> String {
        let mut debug_str = String::new();

        if self.app_state.is_some() {
            debug_str.push('a');
        }
        if self.subnet_directory.is_some() {
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
    if let Some(data) = &bundle.subnet_directory {
        SubnetDirectory::import(data.clone());
    }

    //   let debug_str = &bundle.debug();

    //log!(Log::Info, "state.save [{debug_str}]: saved bundle");
}

// cascade
pub async fn cascade(bundle: &StateBundle) -> Result<(), Error> {
    for (pid, _) in ChildIndex::export() {
        cascade_canister(&pid, bundle).await?;
    }

    Ok(())
}

// cascade_canister
pub async fn cascade_canister(pid: &Principal, bundle: &StateBundle) -> Result<(), Error> {
    let canister_self = canister_self();
    let canister_type = CanisterState::try_get_type()?;
    let debug_str = &bundle.debug();

    log!(
        Log::Info,
        "💦 state.cascade [{debug_str}]: {canister_self} ({canister_type}) -> {pid}"
    );

    Call::unbounded_wait(*pid, "icu_state_cascade")
        .with_arg(bundle)
        .await
        .map_err(IcError::from)
        .map_err(InterfaceError::IcError)?;

    Ok(())
}

// update_canister
pub async fn update_canister(pid: &Principal, bundle: &StateBundle) -> Result<(), Error> {
    let canister_self = canister_self();
    let canister_type = CanisterState::try_get_type()?;
    let debug_str = &bundle.debug();

    log!(
        Log::Info,
        "🔄 state.update [{debug_str}]: {canister_self} ({canister_type}) -> {pid}"
    );

    Call::unbounded_wait(*pid, "icu_state_update")
        .with_arg(bundle)
        .await
        .map_err(IcError::from)
        .map_err(InterfaceError::IcError)?;

    Ok(())
}
