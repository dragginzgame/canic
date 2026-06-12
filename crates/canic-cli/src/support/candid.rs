use canic_host::{icp::existing_local_canister_candid_path, registry::RegistryEntry};
use std::path::{Path, PathBuf};

pub fn role_candid_path(icp_root: Option<&Path>, network: &str, role: &str) -> Option<PathBuf> {
    existing_local_canister_candid_path(icp_root?, network, role)
}

pub fn registry_entry_candid_path(
    icp_root: Option<&Path>,
    network: &str,
    entry: &RegistryEntry,
) -> Option<PathBuf> {
    role_candid_path(icp_root, network, entry.role.as_deref()?)
}
