use crate::{
    Error, Log,
    ic::{api::canister_self, call::Call},
    interface::{InterfaceError, ic::IcError},
    log,
    memory::{AppState, AppStateData, CanisterState, ChildIndex, SubnetIndex, SubnetIndexData},
};
use candid::CandidType;
use serde::Deserialize;

///
/// CascadeBundle
///

#[derive(CandidType, Debug, Deserialize, Default)]
pub struct CascadeBundle {
    app_state_data: Option<AppStateData>,
    subnet_index_data: Option<SubnetIndexData>,
}

impl CascadeBundle {
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
}

// cascade
pub async fn cascade(bundle: &CascadeBundle) -> Result<(), Error> {
    // child index - bail early
    let child_index = ChildIndex::export();
    if child_index.is_empty() {
        return Ok(());
    }

    let mut debug_str = String::new();

    // import data
    if let Some(data) = &bundle.app_state_data {
        AppState::import(*data);
        debug_str.push('a');
    }
    if let Some(data) = &bundle.subnet_index_data {
        SubnetIndex::import(data.clone());
        debug_str.push('s');
    }

    let canister_self = canister_self();
    let canister_kind = CanisterState::try_get_kind()?;

    // iterate child canisters
    for (pid, kind) in child_index {
        let pid_short = &pid;

        log!(
            Log::Info,
            "cascade [{debug_str}]: {canister_self} ({canister_kind}) -> {pid_short} ({kind})"
        );

        Call::unbounded_wait(pid, "icu_cascade")
            .with_arg(bundle)
            .await
            .map_err(IcError::from)
            .map_err(InterfaceError::IcError)?;
    }

    Ok(())
}
