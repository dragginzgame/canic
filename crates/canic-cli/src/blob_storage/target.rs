//! Module: canic_cli::blob_storage::target
//!
//! Responsibility: resolve blob-storage CLI targets and local method metadata.
//! Does not own: transport execution, endpoint policy, or canister DTO parsing.
//! Boundary: maps deployment metadata plus Candid sidecars into call targets.

use crate::{
    blob_storage::{
        BlobStorageCommandError, blob_storage_installed_deployment_error, model::BlobStorageTarget,
        options::CommonOptions,
    },
    support::candid::role_candid_path,
};
use candid::Principal;
use canic_host::{
    candid_endpoints::{EndpointMode, parse_candid_service_endpoints},
    icp_config::resolve_current_canic_icp_root,
    installed_deployment::{InstalledDeploymentRequest, resolve_installed_deployment_from_root},
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
/// BlobStorageMethodMode
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BlobStorageMethodMode {
    Query,
    Update,
}

impl BlobStorageMethodMode {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Query => "query",
            Self::Update => "update",
        }
    }
}

///
/// ResolvedBlobStorageTarget
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedBlobStorageTarget {
    input: String,
    role: Option<String>,
    canister_id: String,
}

pub(super) fn resolve_blob_storage_call_target(
    options: &CommonOptions,
    deployment: &str,
    selector: &str,
    method: &str,
) -> Result<BlobStorageCallTarget, BlobStorageCommandError> {
    let icp_root = resolve_current_canic_icp_root()
        .map_err(|err| BlobStorageCommandError::InstallState(err.to_string()))?;
    let installed = resolve_installed_deployment_from_root(
        &InstalledDeploymentRequest {
            deployment: deployment.to_string(),
            network: options.network.clone(),
            icp: options.icp.clone(),
            detect_lost_local_root: true,
        },
        &icp_root,
    )
    .map_err(blob_storage_installed_deployment_error)?;
    let resolved = resolve_blob_storage_target(
        deployment,
        selector,
        &installed.state.root_canister_id,
        &installed.registry.entries,
    )?;
    let candid_path = resolved
        .role
        .as_deref()
        .and_then(|role| role_candid_path(Some(&icp_root), &options.network, role))
        .ok_or_else(|| BlobStorageCommandError::CandidUnavailable {
            deployment: deployment.to_string(),
            target: selector.to_string(),
        })?;
    let candid =
        fs::read_to_string(&candid_path).map_err(|source| BlobStorageCommandError::CandidRead {
            path: candid_path.clone(),
            source,
        })?;
    let method_mode = blob_storage_method_mode(&candid_path, &candid, method)?;

    Ok(BlobStorageCallTarget {
        target: BlobStorageTarget::from_installed_deployment(
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
    deployment: &str,
    selector: &str,
    root_canister_id: &str,
    registry: &[RegistryEntry],
) -> Result<ResolvedBlobStorageTarget, BlobStorageCommandError> {
    if selector == "root" || selector == root_canister_id {
        return Ok(ResolvedBlobStorageTarget {
            input: selector.to_string(),
            role: Some("root".to_string()),
            canister_id: root_canister_id.to_string(),
        });
    }

    if Principal::from_text(selector).is_ok() {
        if let Some(entry) = registry.iter().find(|entry| entry.pid == selector) {
            return Ok(resolved_from_entry(selector, entry));
        }
        return Ok(ResolvedBlobStorageTarget {
            input: selector.to_string(),
            role: None,
            canister_id: selector.to_string(),
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
                deployment: deployment.to_string(),
                role: selector.to_string(),
            });
        }
    }

    if let Some(entry) = registry.iter().find(|entry| entry.pid == selector) {
        return Ok(resolved_from_entry(selector, entry));
    }
    Err(BlobStorageCommandError::UnknownTarget {
        deployment: deployment.to_string(),
        target: selector.to_string(),
    })
}

fn resolved_from_entry(selector: &str, entry: &RegistryEntry) -> ResolvedBlobStorageTarget {
    ResolvedBlobStorageTarget {
        input: selector.to_string(),
        role: entry.role.clone(),
        canister_id: entry.pid.clone(),
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
    fn principal_resolution_wins_before_role_match() {
        let principal = "rrkah-fqaaa-aaaaa-aaaaq-cai";
        let entry = registry_entry("ryjl3-tyaaa-aaaaa-aaaba-cai", Some(principal));
        let target = resolve_blob_storage_target("local", principal, "aaaaa-aa", &[entry])
            .expect("principal-like target should resolve as canister id");

        assert_eq!(target.role, None);
        assert_eq!(target.canister_id, principal);
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
    fn direct_canister_id_without_registry_entry_has_no_role() {
        let principal = "rrkah-fqaaa-aaaaa-aaaaq-cai";
        let target = resolve_blob_storage_target("local", principal, "aaaaa-aa", &[])
            .expect("direct principal should resolve");

        assert_eq!(target.role, None);
        assert_eq!(target.canister_id, principal);
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
            kind: None,
            parent_pid: None,
            module_hash: None,
        }
    }
}
