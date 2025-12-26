//! State synchronization routines shared by root and child canisters.
//!
//! Bundles snapshot portions of `AppState`, `SubnetState`, and the directory
//! views, ships them across the topology, and replays them on recipients.

use super::warn_if_large;
use crate::{
    Error,
    log::Topic,
    ops::{
        OpsError,
        directory::{AppDirectoryOps, DirectoryView, SubnetDirectoryOps},
        prelude::*,
        state::{AppStateData, AppStateOps, SubnetStateData, SubnetStateOps},
        topology::subnet::{SubnetCanisterChildrenOps, SubnetCanisterRegistryOps},
    },
};

///
/// StateBundle
/// Snapshot of mutable state and directory sections that can be propagated to peers
///

#[derive(CandidType, Clone, Debug, Default, Deserialize)]
pub struct StateBundle {
    // states
    pub app_state: Option<AppStateData>,
    pub subnet_state: Option<SubnetStateData>,

    // directories
    pub app_directory: Option<DirectoryView>,
    pub subnet_directory: Option<DirectoryView>,
}

impl StateBundle {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a bundle containing the root canister’s full state view.
    #[must_use]
    pub fn root() -> Self {
        Self {
            app_state: Some(AppStateOps::export()),
            subnet_state: Some(SubnetStateOps::export()),
            app_directory: Some(AppDirectoryOps::export()),
            subnet_directory: Some(SubnetDirectoryOps::export()),
        }
    }

    #[must_use]
    pub fn with_app_state(mut self) -> Self {
        self.app_state = Some(AppStateOps::export());
        self
    }

    #[must_use]
    pub fn with_subnet_state(mut self) -> Self {
        self.subnet_state = Some(SubnetStateOps::export());
        self
    }

    #[must_use]
    pub fn with_app_directory(mut self) -> Self {
        self.app_directory = Some(AppDirectoryOps::export());
        self
    }

    #[must_use]
    pub fn with_subnet_directory(mut self) -> Self {
        self.subnet_directory = Some(SubnetDirectoryOps::export());
        self
    }

    /// Compact debug string showing which sections are present.
    /// Example: `[as ss .. sd]`
    #[must_use]
    pub fn debug(&self) -> String {
        const fn fmt(present: bool, code: &str) -> &str {
            if present { code } else { ".." }
        }

        format!(
            "[{} {} {} {}]",
            fmt(self.app_state.is_some(), "as"),
            fmt(self.subnet_state.is_some(), "ss"),
            fmt(self.app_directory.is_some(), "ad"),
            fmt(self.subnet_directory.is_some(), "sd"),
        )
    }

    /// Whether the bundle carries any sections (true when every optional field is absent).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.app_state.is_none()
            && self.subnet_state.is_none()
            && self.app_directory.is_none()
            && self.subnet_directory.is_none()
    }
}

/// Cascade from root: distribute the state bundle to direct children.
/// No-op when the bundle is empty.
pub(crate) async fn root_cascade_state(bundle: StateBundle) -> Result<(), Error> {
    OpsError::require_root()?;

    if bundle.is_empty() {
        log!(
            Topic::Sync,
            Info,
            "💦 sync.state: root_cascade skipped (empty bundle)"
        );

        return Ok(());
    }

    let root_pid = canister_self();
    let children = SubnetCanisterRegistryOps::children(root_pid);
    let child_count = children.len();
    warn_if_large("root state cascade", child_count);

    let mut failures = 0;
    for child in children {
        if let Err(err) = send_bundle(&child.pid, &bundle).await {
            failures += 1;
            log!(
                Topic::Sync,
                Warn,
                "💦 sync.state: failed to cascade to {}: {}",
                child.pid,
                err
            );
        }
    }

    if failures > 0 {
        log!(
            Topic::Sync,
            Warn,
            "💦 sync.state: {failures} child cascade(s) failed; continuing"
        );
    }

    Ok(())
}

/// Public wrapper for root state cascades to keep the internal entrypoint crate-private.
pub async fn cascade_root_state(bundle: StateBundle) -> Result<(), Error> {
    root_cascade_state(bundle).await
}

/// Cascade from a child: forward the bundle to direct children.
/// No-op when the bundle is empty.
pub async fn nonroot_cascade_state(bundle: &StateBundle) -> Result<(), Error> {
    OpsError::deny_root()?;

    // update local state
    save_state(bundle)?;

    let children = SubnetCanisterChildrenOps::export();
    let child_count = children.len();
    warn_if_large("nonroot state cascade", child_count);

    let mut failures = 0;
    for child in children {
        if let Err(err) = send_bundle(&child.pid, bundle).await {
            failures += 1;
            log!(
                Topic::Sync,
                Warn,
                "💦 sync.state: failed to cascade to {}: {}",
                child.pid,
                err
            );
        }
    }

    if failures > 0 {
        log!(
            Topic::Sync,
            Warn,
            "💦 sync.state: {failures} child cascade(s) failed; continuing"
        );
    }

    Ok(())
}

/// Save state locally on a child canister.
fn save_state(bundle: &StateBundle) -> Result<(), Error> {
    OpsError::deny_root()?;

    // states
    if let Some(state) = bundle.app_state {
        AppStateOps::import(state);
    }
    if let Some(state) = bundle.subnet_state {
        SubnetStateOps::import(state);
    }

    // directories
    if let Some(dir) = &bundle.app_directory {
        AppDirectoryOps::import(dir.clone());
    }
    if let Some(dir) = &bundle.subnet_directory {
        SubnetDirectoryOps::import(dir.clone());
    }

    Ok(())
}

/// Low-level bundle sender.
async fn send_bundle(pid: &Principal, bundle: &StateBundle) -> Result<(), Error> {
    let debug = bundle.debug();
    log!(Topic::Sync, Info, "💦 sync.state: {debug} -> {pid}");

    call_and_decode::<Result<(), Error>>(*pid, crate::ops::rpc::methods::CANIC_SYNC_STATE, bundle)
        .await?
}
