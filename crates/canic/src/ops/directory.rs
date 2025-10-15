use crate::{
    Error,
    config::Config,
    memory::{
        Env,
        directory::{AppDirectory, PrincipalList, SubnetDirectory},
        topology::SubnetCanisterRegistry,
    },
    ops::{
        context::cfg_current_subnet,
        sync::state::{StateBundle, root_cascade_state},
    },
};
use std::collections::BTreeMap;

///
/// Rebuilds and reimports the AppDirectory and/or SubnetDirectory
/// based on the current contents of the SubnetCanisterRegistry.
///
/// Detects changes by comparing against the existing stable directories
/// before importing and cascading.
///
pub(crate) async fn sync_directories_from_registry() -> Result<(), Error> {
    let cfg = Config::get();
    let subnet_cfg = cfg_current_subnet()?;
    let mut bundle = StateBundle::default();

    // Get all current canisters from the registry
    let entries = SubnetCanisterRegistry::export();

    //
    // Rebuild the app and subnet directory views from registry data
    //
    let mut app_map: BTreeMap<_, PrincipalList> = BTreeMap::new();
    let mut subnet_map: BTreeMap<_, PrincipalList> = BTreeMap::new();

    for entry in entries {
        let ty = entry.ty.clone();

        // Prime root → add to AppDirectory if configured
        if Env::is_prime_root() && cfg.app_directory.contains(&ty) {
            app_map.entry(ty.clone()).or_default().push(entry.pid);
        }

        // Always check subnet directory configuration
        if subnet_cfg.subnet_directory.contains(&ty) {
            subnet_map.entry(ty).or_default().push(entry.pid);
        }
    }

    //
    // Detect and import AppDirectory changes (only for prime root)
    //
    if Env::is_prime_root() {
        let current_app = AppDirectory::export();
        let new_app: Vec<_> = app_map
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        if current_app != new_app {
            AppDirectory::import(app_map.into_iter().collect());
            bundle = bundle.with_app_directory();
        }
    }

    //
    // Detect and import SubnetDirectory changes
    //
    let current_subnet = SubnetDirectory::export();
    let new_subnet: Vec<_> = subnet_map
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    if current_subnet != new_subnet {
        SubnetDirectory::import(subnet_map.into_iter().collect());
        bundle = bundle.with_subnet_directory();
    }

    //
    // Cascade updates if anything changed
    //
    if !bundle.is_empty() {
        root_cascade_state(bundle).await?;
    }

    Ok(())
}
