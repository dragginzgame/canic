use super::{ListCommandError, options::ListOptions, render::ReadyStatus, state_environment};
use crate::cli::defaults::local_environment;
use crate::support::candid::registry_entry_candid_path;
use crate::support::registry_tree::visible_entries;
use canic_host::{
    canic_metadata::query_canic_metadata_version,
    canister_ready::{query_canister_ready, query_local_canister_ready},
    cycle_balance::query_cycle_balance,
    format::{cycles_tc, wasm_size_label},
    icp::{IcpCli, IcpDiagnostic, classify_icp_diagnostic},
    icp_config::resolve_current_canic_icp_root,
    installed_fleet::{
        InstalledFleetRequest, InstalledFleetResolution, resolve_installed_fleet_from_root,
    },
    registry::RegistryEntry,
    release_set::artifact_root_path,
    replica_query,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
};

use super::options::ListSource;

const OBSERVATION_ERROR: &str = "error";

pub(super) fn load_registry_entries(
    options: &ListOptions,
) -> Result<Vec<RegistryEntry>, ListCommandError> {
    let registry = match options.source {
        ListSource::RootRegistry => resolve_list_fleet(options)?.registry.entries,
        ListSource::Config => {
            unreachable!("config source does not use registry entries")
        }
    };

    Ok(registry)
}

pub(super) fn list_ready_statuses(
    options: &ListOptions,
    registry: &[RegistryEntry],
    canister: Option<&str>,
) -> Result<BTreeMap<String, ReadyStatus>, ListCommandError> {
    let icp_root = resolve_live_icp_root()?;
    if replica_query::uses_local_replica_transport(options.environment.as_deref(), Some(&icp_root))?
    {
        return local_ready_statuses(options, registry, canister, &icp_root);
    }

    let mut statuses = BTreeMap::new();
    for entry in visible_entries(registry, canister)? {
        statuses.insert(
            entry.pid.clone(),
            check_ready_status(options, Some(&icp_root), entry),
        );
    }
    Ok(statuses)
}

pub(super) fn list_cycle_balances(
    options: &ListOptions,
    registry: &[RegistryEntry],
    canister: Option<&str>,
) -> Result<BTreeMap<String, String>, ListCommandError> {
    let icp = options.icp.clone();
    let environment = options.environment.clone();
    let icp_root = resolve_live_icp_root()?;
    collect_visible_entry_values(
        registry,
        canister,
        OBSERVATION_ERROR.to_string(),
        move |entry| {
            cycle_balance_label_endpoint(&icp, environment.clone(), Some(&icp_root), &entry)
        },
    )
}

pub(super) fn list_canic_versions(
    options: &ListOptions,
    registry: &[RegistryEntry],
    canister: Option<&str>,
) -> Result<BTreeMap<String, String>, ListCommandError> {
    let icp = options.icp.clone();
    let environment = options.environment.clone();
    let icp_root = resolve_live_icp_root()?;
    collect_visible_entry_values(
        registry,
        canister,
        OBSERVATION_ERROR.to_string(),
        move |entry| {
            canic_version_label_endpoint(&icp, environment.clone(), Some(&icp_root), &entry)
        },
    )
}

pub(super) fn list_module_hashes(
    registry: &[RegistryEntry],
    canister: Option<&str>,
) -> Result<BTreeMap<String, String>, ListCommandError> {
    Ok(visible_entries(registry, canister)?
        .into_iter()
        .filter_map(|entry| {
            entry
                .module_hash
                .as_ref()
                .map(|hash| (entry.pid.clone(), hash.clone()))
        })
        .collect())
}

pub(super) fn resolve_wasm_sizes(
    options: &ListOptions,
    registry: &[RegistryEntry],
) -> Result<BTreeMap<String, String>, ListCommandError> {
    let root = resolve_icp_artifact_root(options)?;
    let artifact_environment = state_environment(options);
    let artifact_root = artifact_root_path(&root, &artifact_environment);
    Ok(registry
        .iter()
        .filter_map(|entry| entry.role.as_deref())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter_map(|role| {
            let artifact_dir = artifact_root.join(role);
            let wasm_bytes = fs::metadata(artifact_dir.join(format!("{role}.wasm")))
                .ok()
                .map(|metadata| metadata.len());
            let gzip_bytes = fs::metadata(artifact_dir.join(format!("{role}.wasm.gz")))
                .ok()
                .map(|metadata| metadata.len());
            if wasm_bytes.is_none() && gzip_bytes.is_none() {
                None
            } else {
                Some((role.to_string(), wasm_size_label(wasm_bytes, gzip_bytes)))
            }
        })
        .collect())
}

fn check_ready_status(
    options: &ListOptions,
    icp_root: Option<&Path>,
    entry: &RegistryEntry,
) -> ReadyStatus {
    let icp = live_icp(&options.icp, options.environment.clone(), icp_root);
    let Ok(binding) = registry_entry_candid_path(icp_root, &state_environment(options), entry)
    else {
        return ReadyStatus::Error;
    };
    let Ok(ready) = query_canister_ready(
        &icp,
        &entry.pid,
        &state_environment(options),
        icp_root,
        &binding,
    ) else {
        return ReadyStatus::Error;
    };
    if ready {
        ReadyStatus::Ready
    } else {
        ReadyStatus::NotReady
    }
}

fn local_ready_statuses(
    options: &ListOptions,
    registry: &[RegistryEntry],
    canister: Option<&str>,
    icp_root: &Path,
) -> Result<BTreeMap<String, ReadyStatus>, ListCommandError> {
    let environment = options.environment.clone();
    let icp_root = icp_root.to_path_buf();
    collect_visible_entry_values(registry, canister, ReadyStatus::Error, move |entry| {
        match query_local_canister_ready(
            environment.as_deref().unwrap_or("local"),
            &entry.pid,
            Some(&icp_root),
        ) {
            Ok(true) => ReadyStatus::Ready,
            Ok(false) => ReadyStatus::NotReady,
            Err(_) => ReadyStatus::Error,
        }
    })
}

fn collect_visible_entry_values<T, F>(
    registry: &[RegistryEntry],
    canister: Option<&str>,
    worker_panic_value: T,
    query: F,
) -> Result<BTreeMap<String, T>, ListCommandError>
where
    T: Clone + Send + 'static,
    F: Fn(RegistryEntry) -> T + Send + Sync + 'static,
{
    let query = Arc::new(query);
    let mut handles = Vec::new();
    for entry in visible_entries(registry, canister)? {
        let entry = entry.clone();
        let pid = entry.pid.clone();
        let query = Arc::clone(&query);
        handles.push((pid, thread::spawn(move || query(entry))));
    }

    let mut values = BTreeMap::new();
    for (pid, handle) in handles {
        let value = handle.join().unwrap_or_else(|_| worker_panic_value.clone());
        values.insert(pid, value);
    }
    Ok(values)
}

fn cycle_balance_label_endpoint(
    icp: &str,
    environment: Option<String>,
    icp_root: Option<&Path>,
    entry: &RegistryEntry,
) -> String {
    let environment = environment.unwrap_or_else(local_environment);
    let Ok(binding) = registry_entry_candid_path(icp_root, &environment, entry) else {
        return OBSERVATION_ERROR.to_string();
    };
    let icp = live_icp(icp, Some(environment.clone()), icp_root);
    query_cycle_balance(&icp, &entry.pid, &environment, icp_root, &binding)
        .map_or_else(|_| OBSERVATION_ERROR.to_string(), cycles_tc)
}

fn canic_version_label_endpoint(
    icp: &str,
    environment: Option<String>,
    icp_root: Option<&Path>,
    entry: &RegistryEntry,
) -> String {
    let environment = environment.unwrap_or_else(local_environment);
    let Ok(binding) = registry_entry_candid_path(icp_root, &environment, entry) else {
        return OBSERVATION_ERROR.to_string();
    };
    let icp = live_icp(icp, Some(environment), icp_root);
    query_canic_metadata_version(&icp, &entry.pid, &binding)
        .unwrap_or_else(|_| OBSERVATION_ERROR.to_string())
}

fn live_icp(icp: &str, environment: Option<String>, icp_root: Option<&Path>) -> IcpCli {
    let icp = IcpCli::new(icp, environment);
    if let Some(root) = icp_root {
        icp.with_cwd(root)
    } else {
        icp
    }
}

fn resolve_icp_artifact_root(_options: &ListOptions) -> Result<PathBuf, ListCommandError> {
    resolve_live_icp_root()
}

fn resolve_list_fleet(options: &ListOptions) -> Result<InstalledFleetResolution, ListCommandError> {
    let icp_root = resolve_live_icp_root()?;
    resolve_installed_fleet_from_root(
        &InstalledFleetRequest {
            fleet: options.target.clone(),
            environment: state_environment(options),
        },
        &icp_root,
    )
    .map_err(ListCommandError::from)
    .map_err(add_root_registry_hint)
}

fn resolve_live_icp_root() -> Result<PathBuf, ListCommandError> {
    resolve_current_canic_icp_root().map_err(ListCommandError::from)
}

fn add_root_registry_hint(error: ListCommandError) -> ListCommandError {
    match error {
        ListCommandError::Icp(source) => {
            let Some(hint) = source.external_output().and_then(root_registry_hint) else {
                return ListCommandError::Icp(source);
            };
            ListCommandError::IcpHint { source, hint }
        }
        error => error,
    }
}

fn root_registry_hint(stderr: &str) -> Option<&'static str> {
    match classify_icp_diagnostic(stderr) {
        Some(IcpDiagnostic::CanisterIdMissing) => Some(
            "no root canister id exists for this Fleet. Use `canic app config <app>` to inspect source config, or run `canic install <app> <fleet> --fleet-input <path>` before querying the root registry.",
        ),
        Some(IcpDiagnostic::CanisterWasmMissing) => Some(
            "the root canister id exists but no Canic root code is installed. Run `canic install <app> <fleet> --fleet-input <path>`, then use `canic info list <fleet>`.",
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
