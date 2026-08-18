use std::path::Path;

use canic_host::{
    protocol_binding::{
        ProtocolBindingError, ResolvedProtocolBinding, resolve_registry_protocol_binding,
    },
    registry::RegistryEntry,
};

pub fn registry_entry_candid_path(
    icp_root: Option<&Path>,
    artifact_environment: &str,
    entry: &RegistryEntry,
) -> Result<ResolvedProtocolBinding, ProtocolBindingError> {
    let root = icp_root.ok_or_else(|| ProtocolBindingError::MissingCandid {
        canister: entry.pid.clone(),
        role: entry
            .role
            .clone()
            .unwrap_or_else(|| "<missing>".to_string()),
    })?;
    resolve_registry_protocol_binding(root, artifact_environment, entry)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    #[test]
    fn registry_entry_candid_path_requires_exact_binding() {
        let root = temp_dir("canic-cli-support-candid-missing-role");
        let entry = registry_entry(None);

        assert!(registry_entry_candid_path(Some(root.as_path()), "local", &entry).is_err());
    }

    fn registry_entry(role: Option<&str>) -> RegistryEntry {
        RegistryEntry {
            pid: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
            role: role.map(str::to_string),
            parent_pid: None,
            module_hash: None,
            protocol_binding: None,
        }
    }
}
