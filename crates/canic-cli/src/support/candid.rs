use std::path::{Path, PathBuf};

use canic_host::{icp::existing_local_canister_candid_path, registry::RegistryEntry};

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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    use std::fs;

    #[test]
    fn role_candid_path_returns_existing_sidecar() {
        let root = temp_dir("canic-cli-support-candid-existing");
        let did_path = root.join(".icp/local/canisters/root/root.did");
        fs::create_dir_all(did_path.parent().expect("did parent")).expect("create did parent");
        fs::write(&did_path, "service : {}").expect("write did");

        assert_eq!(
            role_candid_path(Some(root.as_path()), "local", "root").as_deref(),
            Some(did_path.as_path())
        );
        assert_eq!(role_candid_path(Some(root.as_path()), "ic", "root"), None);

        fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn role_candid_path_without_root_returns_none() {
        assert_eq!(role_candid_path(None, "local", "root"), None);
    }

    #[test]
    fn registry_entry_candid_path_requires_entry_role() {
        let root = temp_dir("canic-cli-support-candid-missing-role");
        let entry = registry_entry(None);

        assert_eq!(
            registry_entry_candid_path(Some(root.as_path()), "local", &entry),
            None
        );
    }

    fn registry_entry(role: Option<&str>) -> RegistryEntry {
        RegistryEntry {
            pid: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
            role: role.map(str::to_string),
            kind: None,
            parent_pid: None,
            module_hash: None,
        }
    }
}
