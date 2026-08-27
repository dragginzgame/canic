//! Module: canic_cli::blob_storage::target
//!
//! Responsibility: resolve blob-storage CLI targets and local method metadata.
//! Does not own: transport execution, endpoint policy, or canister DTO parsing.
//! Boundary: maps Fleet metadata plus Candid sidecars into call targets.

use crate::blob_storage::{
    BlobStorageCommandError,
    model::{BlobStorageMethodMode, BlobStorageTarget},
    options::CommonOptions,
};
use candid::Principal;
use canic_host::{
    candid_endpoints::{EndpointMode, parse_candid_service_endpoints},
    fleet_ensure::resolve_current_fleet,
    icp_config::resolve_current_canic_icp_root,
    protocol_binding::resolve_registry_protocol_binding,
    registry::RegistryEntry,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

///
/// BlobStorageCallTarget
///

pub(super) struct BlobStorageCallTarget {
    pub(super) target: BlobStorageTarget,
    pub(super) method_mode: BlobStorageMethodMode,
    pub(super) candid_path: PathBuf,
    pub(super) icp_root: PathBuf,
}

///
/// ResolvedBlobStorageTarget
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedBlobStorageTarget {
    input: String,
    role: Option<String>,
    canister_id: String,
    registry_entry: RegistryEntry,
}

pub(super) fn resolve_blob_storage_call_target(
    options: &CommonOptions,
    fleet: &str,
    selector: &str,
    method: &str,
) -> Result<BlobStorageCallTarget, BlobStorageCommandError> {
    let icp_root = resolve_current_canic_icp_root().map_err(BlobStorageCommandError::IcpRoot)?;
    let current = resolve_current_fleet(&icp_root, &options.environment, fleet)?;
    let root_canister_id = current
        .topology
        .unique_fleet_subnet_root(fleet)?
        .to_string();
    let resolved = resolve_blob_storage_target(
        fleet,
        selector,
        &root_canister_id,
        &current.registry.entries,
    )?;
    let binding = resolve_registry_protocol_binding(
        &icp_root,
        &options.environment,
        &resolved.registry_entry,
    )
    .map_err(|_| BlobStorageCommandError::CandidUnavailable {
        fleet: fleet.to_string(),
        target: selector.to_string(),
    })?;
    let candid_path = binding.candid_path().to_path_buf();
    let candid =
        fs::read_to_string(&candid_path).map_err(|source| BlobStorageCommandError::CandidRead {
            path: candid_path.clone(),
            source,
        })?;
    let method_mode = blob_storage_method_mode(&candid_path, &candid, method)?;

    Ok(BlobStorageCallTarget {
        target: BlobStorageTarget::from_current_ensure(
            &resolved.input,
            resolved.role,
            &resolved.canister_id,
        ),
        method_mode,
        candid_path,
        icp_root,
    })
}

fn resolve_blob_storage_target(
    fleet: &str,
    selector: &str,
    root_canister_id: &str,
    registry: &[RegistryEntry],
) -> Result<ResolvedBlobStorageTarget, BlobStorageCommandError> {
    if selector == "root" || selector == root_canister_id {
        let entry = registry
            .iter()
            .find(|entry| entry.pid == root_canister_id)
            .ok_or_else(|| BlobStorageCommandError::UnknownTarget {
                fleet: fleet.to_string(),
                target: selector.to_string(),
            })?;
        return Ok(ResolvedBlobStorageTarget {
            input: selector.to_string(),
            role: Some("root".to_string()),
            canister_id: root_canister_id.to_string(),
            registry_entry: entry.clone(),
        });
    }

    if Principal::from_text(selector).is_ok() {
        if let Some(entry) = registry.iter().find(|entry| entry.pid == selector) {
            return Ok(resolved_from_entry(selector, entry));
        }
        return Err(BlobStorageCommandError::UnknownTarget {
            fleet: fleet.to_string(),
            target: selector.to_string(),
        });
    }

    let role_matches = registry
        .iter()
        .filter(|entry| entry.role.as_deref() == Some(selector))
        .collect::<Vec<_>>();
    match role_matches.as_slice() {
        [entry] => return Ok(resolved_from_entry(selector, entry)),
        [] => {}
        _ => {
            return Err(BlobStorageCommandError::AmbiguousRole {
                fleet: fleet.to_string(),
                role: selector.to_string(),
            });
        }
    }

    if let Some(entry) = registry.iter().find(|entry| entry.pid == selector) {
        return Ok(resolved_from_entry(selector, entry));
    }
    Err(BlobStorageCommandError::UnknownTarget {
        fleet: fleet.to_string(),
        target: selector.to_string(),
    })
}

fn resolved_from_entry(selector: &str, entry: &RegistryEntry) -> ResolvedBlobStorageTarget {
    ResolvedBlobStorageTarget {
        input: selector.to_string(),
        role: entry.role.clone(),
        canister_id: entry.pid.clone(),
        registry_entry: entry.clone(),
    }
}

fn blob_storage_method_mode(
    path: &Path,
    candid: &str,
    method: &str,
) -> Result<BlobStorageMethodMode, BlobStorageCommandError> {
    let endpoints = parse_candid_service_endpoints(candid).map_err(|source| {
        BlobStorageCommandError::CandidParse {
            path: path.to_path_buf(),
            source,
        }
    })?;
    let endpoint = endpoints
        .iter()
        .find(|endpoint| endpoint.name == method)
        .ok_or_else(|| BlobStorageCommandError::MethodUnavailable {
            path: path.to_path_buf(),
            method: method.to_string(),
        })?;
    if endpoint
        .modes
        .iter()
        .any(|mode| matches!(mode, EndpointMode::Query | EndpointMode::CompositeQuery))
    {
        Ok(BlobStorageMethodMode::Query)
    } else {
        Ok(BlobStorageMethodMode::Update)
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unregistered_principal_cannot_match_a_role_shaped_name() {
        let principal = "rrkah-fqaaa-aaaaa-aaaaq-cai";
        let entry = registry_entry("ryjl3-tyaaa-aaaaa-aaaba-cai", Some(principal));
        let error = resolve_blob_storage_target("local", principal, "aaaaa-aa", &[entry])
            .expect_err("an unregistered principal must fail closed");

        assert!(matches!(
            error,
            BlobStorageCommandError::UnknownTarget { target, .. } if target == principal
        ));
    }

    #[test]
    fn direct_registered_canister_id_reuses_registry_role_metadata() {
        let principal = "rrkah-fqaaa-aaaaa-aaaaq-cai";
        let entry = registry_entry(principal, Some("backend"));
        let target = resolve_blob_storage_target("local", principal, "aaaaa-aa", &[entry])
            .expect("registered principal should resolve");

        assert_eq!(target.role.as_deref(), Some("backend"));
        assert_eq!(target.canister_id, principal);
    }

    #[test]
    fn direct_canister_id_without_current_inventory_entry_rejects() {
        let principal = "rrkah-fqaaa-aaaaa-aaaaq-cai";
        let error = resolve_blob_storage_target("local", principal, "aaaaa-aa", &[])
            .expect_err("unregistered direct principal rejected");

        assert!(matches!(
            error,
            BlobStorageCommandError::UnknownTarget { target, .. } if target == principal
        ));
    }

    #[test]
    fn method_mode_comes_from_candid_metadata() {
        let candid = r#"
            service : {
                get_blob_storage_status : (record { sync_gateway_principals : bool }) -> () query;
                "_immutableObjectStorageFundFromProjectCycles" : (nat) -> ();
            }
        "#;

        assert_eq!(
            blob_storage_method_mode(
                &PathBuf::from("backend.did"),
                candid,
                "get_blob_storage_status"
            )
            .expect("status mode"),
            BlobStorageMethodMode::Query
        );
        assert_eq!(
            blob_storage_method_mode(
                &PathBuf::from("backend.did"),
                candid,
                "_immutableObjectStorageFundFromProjectCycles"
            )
            .expect("fund mode"),
            BlobStorageMethodMode::Update
        );
    }

    fn registry_entry(pid: &str, role: Option<&str>) -> RegistryEntry {
        RegistryEntry {
            pid: pid.to_string(),
            role: role.map(str::to_string),
            parent_pid: None,
            module_hash: None,
            protocol_binding: None,
        }
    }
}
