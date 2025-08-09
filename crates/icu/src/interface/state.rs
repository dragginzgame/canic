use crate::{
    Error, Log,
    ic::{api::canister_self, call::Call},
    interface::{InterfaceError, ic::IcError},
    log,
    memory::{AppState, AppStateData, CanisterState, ChildIndex, SubnetIndex, SubnetIndexData},
};
use candid::{CandidType, Principal};
use serde::Deserialize;

///
/// StateBundle
///

#[derive(CandidType, Debug, Default, Deserialize)]
pub struct StateBundle {
    app_state_data: Option<AppStateData>,
    subnet_index_data: Option<SubnetIndexData>,
}

impl StateBundle {
    #[must_use]
    pub fn all() -> Self {
        Self {
            app_state_data: Some(AppState::export()),
            subnet_index_data: Some(SubnetIndex::export()),
        }
    }

    #[must_use]
    pub fn app_state() -> Self {
        Self {
            app_state_data: Some(AppState::export()),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn subnet_index() -> Self {
        Self {
            subnet_index_data: Some(SubnetIndex::export()),
            ..Default::default()
        }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.app_state_data.is_none() && self.subnet_index_data.is_none()
    }

    fn debug(&self) -> String {
        let mut debug_str = String::new();

        if self.app_state_data.is_some() {
            debug_str.push('a');
        }
        if self.subnet_index_data.is_some() {
            debug_str.push('s');
        }

        debug_str
    }
}

// save_state
pub fn save_state(bundle: &StateBundle) {
    if let Some(data) = &bundle.app_state_data {
        AppState::import(*data);
    }
    if let Some(data) = &bundle.subnet_index_data {
        SubnetIndex::import(data.clone());
    }

    let debug_str = &bundle.debug();

    log!(Log::Info, "state.save [{debug_str}]: saved bundle");
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
    let canister_kind = CanisterState::try_get_kind()?;
    let debug_str = &bundle.debug();

    log!(
        Log::Info,
        "state.cascade [{debug_str}]: {canister_self} ({canister_kind}) -> {pid}"
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
    let canister_kind = CanisterState::try_get_kind()?;
    let debug_str = &bundle.debug();

    log!(
        Log::Info,
        "state.update [{debug_str}]: {canister_self} ({canister_kind}) -> {pid}"
    );

    Call::unbounded_wait(*pid, "icu_state_update")
        .with_arg(bundle)
        .await
        .map_err(IcError::from)
        .map_err(InterfaceError::IcError)?;

    Ok(())
}
