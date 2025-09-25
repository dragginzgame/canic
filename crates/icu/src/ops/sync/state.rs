use crate::{
    Error,
    memory::{
        state::{AppState, AppStateData, SubnetState, SubnetStateData},
        subnet::{SubnetChildren, SubnetRegistry},
    },
    ops::{OpsError, prelude::*},
};

///
/// StateBundle
/// this can be made up of multiple optional parts
///

#[derive(CandidType, Copy, Clone, Debug, Default, Deserialize)]
pub struct StateBundle {
    pub app_state: Option<AppStateData>,
    pub subnet_state: Option<SubnetStateData>,
}

impl StateBundle {
    #[must_use]
    pub fn root() -> Self {
        Self {
            app_state: Some(AppState::export()),
            subnet_state: Some(SubnetState::export()),
        }
    }

    /// Compact debug string (`a..`) showing which sections are present.
    fn debug(self) -> String {
        [
            if self.app_state.is_some() { 'a' } else { '.' },
            if self.subnet_state.is_some() {
                's'
            } else {
                '.'
            },
        ]
        .iter()
        .collect()
    }

    /// Whether the bundle is "empty" (nothing to sync).
    const fn is_empty(self) -> bool {
        self.app_state.is_none()
    }
}

///
/// Cascade from root: distribute the state bundle to direct children.
/// If the bundle is empty, do nothing.
///
pub async fn root_cascade(bundle: StateBundle) -> Result<(), Error> {
    OpsError::require_root()?;

    if bundle.is_empty() {
        log!(
            Log::Info,
            "💦 sync.state: root_cascade skipped (empty bundle)"
        );
        return Ok(());
    }

    let root_pid = canister_self();
    for child in SubnetRegistry::children(root_pid) {
        send_bundle(&child.pid, &bundle).await?;
    }

    Ok(())
}

///
/// Cascade from a child: forward the bundle down to direct children.
/// If the bundle is empty, do nothing.
///
pub async fn cascade_children(bundle: &StateBundle) -> Result<(), Error> {
    OpsError::deny_root()?;

    if bundle.is_empty() {
        log!(
            Log::Info,
            "💦 sync.state: cascade_children skipped (empty bundle)"
        );
        return Ok(());
    }

    for child in SubnetChildren::export() {
        send_bundle(&child.pid, bundle).await?;
    }

    Ok(())
}

/// Save state locally on a child canister.
pub fn save_state(bundle: &StateBundle) -> Result<(), Error> {
    OpsError::deny_root()?;

    if let Some(state) = bundle.app_state {
        AppState::import(state);
    }
    if let Some(state) = bundle.subnet_state {
        SubnetState::import(state);
    }

    Ok(())
}

///
/// Low-level bundle sender.
///
async fn send_bundle(pid: &Principal, bundle: &StateBundle) -> Result<(), Error> {
    let debug = bundle.debug();
    log!(Log::Info, "💦 sync.state: [{debug}] -> {pid}");

    call_and_decode::<Result<(), Error>>(*pid, "icu_sync_state", bundle).await?
}
