use crate::{
    Error,
    interface::prelude::*,
    memory::{
        AppState, AppStateData, SubnetChildrenView, SubnetDirectoryView, SubnetParentsView,
        SubnetView,
    },
};

///
/// SyncBundle
///

#[derive(CandidType, Debug, Default, Deserialize)]
pub struct SyncBundle {
    app_state: Option<AppStateData>,
    subnet_children: Option<SubnetChildrenView>,
    subnet_directory: Option<SubnetDirectoryView>,
    subnet_parents: Option<SubnetParentsView>,
}

impl SyncBundle {
    #[must_use]
    pub fn all() -> Self {
        Self {
            app_state: Some(AppState::export()),
            subnet_children: Some(SubnetView::children().export()),
            subnet_directory: Some(SubnetView::directory().export()),
            subnet_parents: Some(SubnetView::parents().export()),
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
    pub const fn with_app_state(mut self, data: AppStateData) -> Self {
        self.app_state = Some(data);
        self
    }

    #[must_use]
    pub fn with_subnet_children(mut self, view: SubnetChildrenView) -> Self {
        self.subnet_children = Some(view);
        self
    }

    #[must_use]
    pub fn with_subnet_directory(mut self, view: SubnetDirectoryView) -> Self {
        self.subnet_directory = Some(view);
        self
    }

    #[must_use]
    pub fn with_subnet_parents(mut self, view: SubnetParentsView) -> Self {
        self.subnet_parents = Some(view);
        self
    }

    fn debug(&self) -> String {
        let mut debug_str = String::new();

        if self.app_state.is_some() {
            debug_str.push('a');
        } else {
            debug_str.push('.');
        }

        if self.subnet_children.is_some() {
            debug_str.push('c');
        } else {
            debug_str.push('.');
        }

        if self.subnet_directory.is_some() {
            debug_str.push('d');
        } else {
            debug_str.push('.');
        }

        if self.subnet_parents.is_some() {
            debug_str.push('p');
        } else {
            debug_str.push('.');
        }

        debug_str
    }
}

// save_state
pub fn save_state(bundle: &SyncBundle) {
    if let Some(data) = &bundle.app_state {
        AppState::import(*data);
    }
    if let Some(data) = &bundle.subnet_children {
        SubnetView::children().import(data.clone());
    }
    if let Some(data) = &bundle.subnet_directory {
        SubnetView::directory().import(data.clone());
    }
    if let Some(data) = &bundle.subnet_parents {
        SubnetView::parents().import(data.clone());
    }
}

/// propagate state to all children
pub async fn cascade_children(bundle: &SyncBundle) -> Result<(), Error> {
    for canister in SubnetView::children().export() {
        send_bundle(&canister.pid, bundle, "icu_sync_cascade", "cascade").await?;
    }

    Ok(())
}

/// update a specific canister
pub async fn update_canister(pid: &Principal, bundle: &SyncBundle) -> Result<(), Error> {
    send_bundle(pid, bundle, "icu_sync_update", "update").await
}

/// send a bundle to one canister
async fn send_bundle(
    pid: &Principal,
    bundle: &SyncBundle,
    method: &str,
    label: &str,
) -> Result<(), Error> {
    let debug_str = bundle.debug();
    log!(Log::Info, "💦 state.{label}: [{debug_str}] -> {pid}");

    Call::unbounded_wait(*pid, method).with_arg(bundle).await?;

    Ok(())
}
