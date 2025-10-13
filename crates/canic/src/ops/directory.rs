use crate::{
    Error,
    config::Config,
    interface::prelude::*,
    memory::{
        Env,
        directory::{AppDirectory, SubnetDirectory},
    },
    ops::{
        context::cfg_current_subnet,
        sync::state::{StateBundle, root_cascade_state},
    },
};
use candid::Principal;

//
// INTERNAL: directory helpers
//

///
/// Handles inserting a canister into the relevant directory or directories.
///
/// Rules:
/// - If this is the prime root subnet and `ty` is listed in the app_directory,
///   register in the global AppDirectory.
/// - If the current subnet config lists `ty` in its directory,
///   register in the local SubnetDirectory.
/// - If any directory changes, cascade state to children.
///
pub(crate) async fn add_to_directories(ty: &CanisterType, pid: Principal) -> Result<(), Error> {
    let cfg = Config::get();
    let subnet_cfg = cfg_current_subnet()?;
    let mut bundle = StateBundle::default();

    // Prime subnet → app directory
    if Env::is_prime_root() && cfg.app_directory.contains(ty) {
        AppDirectory::register(ty, pid)?;
        bundle = bundle.with_app_directory();
    }

    // Local subnet directory
    if subnet_cfg.directory.contains(ty) {
        SubnetDirectory::register(ty, pid)?;
        bundle = bundle.with_subnet_directory();
    }

    // Cascade if something changed
    if !bundle.is_empty() {
        root_cascade_state(bundle).await?;
    }

    Ok(())
}

///
/// Handles removing a canister from directories.
///
/// Rules:
/// - Removes from AppDirectory if it exists there (only prime root).
/// - Removes from SubnetDirectory if it exists there.
/// - If anything was removed, cascade state.
///
pub(crate) async fn remove_from_directories(ty: &CanisterType) -> Result<(), Error> {
    let cfg = Config::get();
    let subnet_cfg = cfg_current_subnet()?;
    let mut bundle = StateBundle::default();

    // Prime subnet → remove from app directory
    if Env::is_prime_root() && cfg.app_directory.contains(ty) && AppDirectory::remove(ty).is_some()
    {
        bundle = bundle.with_app_directory();
    }

    // Local subnet directory
    if subnet_cfg.directory.contains(ty) && SubnetDirectory::remove(ty).is_some() {
        bundle = bundle.with_subnet_directory();
    }

    // Cascade if something changed
    if !bundle.is_empty() {
        root_cascade_state(bundle).await?;
    }

    Ok(())
}
