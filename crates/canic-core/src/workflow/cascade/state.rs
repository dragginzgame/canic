//!
//! State synchronization routines shared by root and child canisters.
//!
//! This module:
//! - assembles `StateBundle` DTOs from authoritative state via ops
//! - cascades bundles across the subnet topology
//! - applies received bundles locally on child canisters
//!
//! IMPORTANT:
//! - `StateBundle` is a pure DTO (data only)
//! - All assembly logic lives here (workflow)
//! - All persistence happens via ops
//!

use super::warn_if_large;
use crate::{
    Error,
    dto::{
        bundle::StateBundle,
        directory::DirectoryView,
        state::{AppStateView, SubnetStateView},
    },
    log::Topic,
    ops::{
        OpsError,
        prelude::*,
        storage::{
            children::CanisterChildrenOps,
            directory::{AppDirectoryOps, SubnetDirectoryOps},
            registry::SubnetRegistryOps,
            state::{AppStateOps, SubnetStateOps},
        },
    },
};

///
/// StateBundleBuilder
///
/// Builder for assembling `StateBundle` DTOs from authoritative state.
///
/// This type lives in workflow (not dto) because it:
/// - calls ops
/// - selects which sections to include
/// - owns no persistence
///
pub struct StateBundleBuilder {
    bundle: StateBundle,
}

impl StateBundleBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            bundle: StateBundle::default(),
        }
    }

    /// Construct a bundle containing the full root state.
    #[must_use]
    pub fn root() -> Self {
        Self {
            bundle: StateBundle {
                app_state: Some(AppStateOps::export()),
                subnet_state: Some(SubnetStateOps::export()),
                app_directory: Some(AppDirectoryOps::export()),
                subnet_directory: Some(SubnetDirectoryOps::export()),
            },
        }
    }

    #[must_use]
    pub fn with_app_state(mut self) -> Self {
        self.bundle.app_state = Some(AppStateOps::export());
        self
    }

    #[must_use]
    pub fn with_subnet_state(mut self) -> Self {
        self.bundle.subnet_state = Some(SubnetStateOps::export());
        self
    }

    #[must_use]
    pub fn with_app_directory(mut self) -> Self {
        self.bundle.app_directory = Some(AppDirectoryOps::export());
        self
    }

    #[must_use]
    pub fn with_subnet_directory(mut self) -> Self {
        self.bundle.subnet_directory = Some(SubnetDirectoryOps::export());
        self
    }

    #[must_use]
    pub fn with_app_state_view(mut self, view: AppStateView) -> Self {
        self.bundle.app_state = Some(view);
        self
    }

    #[must_use]
    pub fn with_subnet_state_view(mut self, view: SubnetStateView) -> Self {
        self.bundle.subnet_state = Some(view);
        self
    }

    #[must_use]
    pub fn with_app_directory_view(mut self, view: DirectoryView) -> Self {
        self.bundle.app_directory = Some(view);
        self
    }

    #[must_use]
    pub fn with_subnet_directory_view(mut self, view: DirectoryView) -> Self {
        self.bundle.subnet_directory = Some(view);
        self
    }

    #[must_use]
    pub fn build(self) -> StateBundle {
        self.bundle
    }
}

//
// Cascade logic
//

/// Cascade a state bundle from the root canister to its direct children.
///
/// No-op if the bundle is empty.
pub async fn root_cascade_state(bundle: &StateBundle) -> Result<(), Error> {
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
    let children = SubnetRegistryOps::children(root_pid);
    let child_count = children.len();
    warn_if_large("root state cascade", child_count);

    let mut failures = 0;

    for child in children {
        if let Err(err) = send_bundle(&child.pid, bundle).await {
            failures += 1;
            log!(
                Topic::Sync,
                Warn,
                "💦 sync.state: failed to cascade to {}: {err}",
                child.pid
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

/// Cascade a bundle from a non-root canister:
/// - apply it locally
/// - forward it to direct children
pub async fn nonroot_cascade_state(bundle: &StateBundle) -> Result<(), Error> {
    OpsError::deny_root()?;

    if bundle.is_empty() {
        log!(
            Topic::Sync,
            Info,
            "💦 sync.state: nonroot_cascade skipped (empty bundle)"
        );
        return Ok(());
    }

    // Apply locally first
    apply_state(bundle)?;

    let children = CanisterChildrenOps::export();
    let child_count = children.len();
    warn_if_large("nonroot state cascade", child_count);

    let mut failures = 0;
    for child in children {
        if let Err(err) = send_bundle(&child.pid, bundle).await {
            failures += 1;
            log!(
                Topic::Sync,
                Warn,
                "💦 sync.state: failed to cascade to {}: {err}",
                child.pid
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

//
// Local application
//

/// Apply a received state bundle locally.
///
/// Only valid on non-root canisters.
fn apply_state(bundle: &StateBundle) -> Result<(), Error> {
    OpsError::deny_root()?;

    // states
    if let Some(state) = bundle.app_state.clone() {
        AppStateOps::import(state);
    }
    if let Some(state) = bundle.subnet_state.clone() {
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

//
// Transport
//

/// Send a state bundle to another canister.
async fn send_bundle(pid: &Principal, bundle: &StateBundle) -> Result<(), Error> {
    let debug = bundle.debug();
    log!(Topic::Sync, Info, "💦 sync.state: {debug} -> {pid}");

    call_and_decode::<Result<(), Error>>(*pid, crate::ops::rpc::methods::CANIC_SYNC_STATE, bundle)
        .await?
}
