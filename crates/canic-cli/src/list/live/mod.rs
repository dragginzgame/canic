use super::{ListCommandError, options::ListOptions, render::ReadyStatus, state_network};
use crate::cli::defaults::local_network;
use crate::support::candid::registry_entry_candid_path;
use crate::support::registry_tree::visible_entries;
use canic_host::{
    canic_metadata::query_canic_metadata_version,
    canister_ready::{query_canister_ready, query_local_canister_ready},
    cycle_balance::query_cycle_balance,
    format::{cycles_tc, wasm_size_label},
    icp::{IcpCli, IcpDiagnostic, classify_icp_diagnostic},
    icp_config::resolve_current_canic_icp_root,
    installed_deployment::{
        InstalledDeploymentError, InstalledDeploymentRequest, InstalledDeploymentResolution,
        read_installed_deployment_state_from_root, resolve_installed_deployment_from_root,
    },
    registry::RegistryEntry,
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
        ListSource::RootRegistry => resolve_list_deployment(options)?.registry.entries,
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
    if replica_query::should_use_local_replica_query(options.network.as_deref()) {
        return local_ready_statuses(options, registry, canister);
    }

    let icp_root = resolve_live_icp_root()?;
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
    let network = options.network.clone();
    let icp_root = resolve_live_icp_root()?;
    collect_visible_entry_values(
        registry,
        canister,
        OBSERVATION_ERROR.to_string(),
        move |entry| cycle_balance_label_endpoint(&icp, network.clone(), Some(&icp_root), &entry),
    )
}

pub(super) fn list_canic_versions(
    options: &ListOptions,
    registry: &[RegistryEntry],
    canister: Option<&str>,
) -> Result<BTreeMap<String, String>, ListCommandError> {
    let icp = options.icp.clone();
    let network = options.network.clone();
    let icp_root = resolve_live_icp_root()?;
    collect_visible_entry_values(
        registry,
        canister,
        OBSERVATION_ERROR.to_string(),
        move |entry| canic_version_label_endpoint(&icp, network.clone(), Some(&icp_root), &entry),
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
    let network = state_network(options);
    Ok(registry
        .iter()
        .filter_map(|entry| entry.role.as_deref())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter_map(|role| {
            let artifact_dir = root
                .join(".icp")
                .join(&network)
                .join("canisters")
                .join(role);
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
    let icp = live_icp(&options.icp, options.network.clone(), icp_root);
    let candid_path = registry_entry_candid_path(icp_root, &state_network(options), entry);
    let Ok(ready) = query_canister_ready(
        &icp,
        &entry.pid,
        &state_network(options),
        icp_root,
        candid_path.as_deref(),
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
) -> Result<BTreeMap<String, ReadyStatus>, ListCommandError> {
    let network = options.network.clone();
    let icp_root = resolve_live_icp_root()?;
    collect_visible_entry_values(registry, canister, ReadyStatus::Error, move |entry| {
        match query_local_canister_ready(
            network.as_deref().unwrap_or("local"),
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
    network: Option<String>,
    icp_root: Option<&Path>,
    entry: &RegistryEntry,
) -> String {
    let network = network.unwrap_or_else(local_network);
    let candid_path = registry_entry_candid_path(icp_root, &network, entry);
    let icp = live_icp(icp, Some(network.clone()), icp_root);
    query_cycle_balance(&icp, &entry.pid, &network, icp_root, candid_path.as_deref())
        .map_or_else(|_| OBSERVATION_ERROR.to_string(), cycles_tc)
}

fn canic_version_label_endpoint(
    icp: &str,
    network: Option<String>,
    icp_root: Option<&Path>,
    entry: &RegistryEntry,
) -> String {
    let network = network.unwrap_or_else(local_network);
    let candid_path = registry_entry_candid_path(icp_root, &network, entry);
    let icp = live_icp(icp, Some(network), icp_root);
    query_canic_metadata_version(&icp, &entry.pid, candid_path.as_deref())
        .unwrap_or_else(|_| OBSERVATION_ERROR.to_string())
}

fn live_icp(icp: &str, network: Option<String>, icp_root: Option<&Path>) -> IcpCli {
    let icp = IcpCli::new(icp, None, network);
    if let Some(root) = icp_root {
        icp.with_cwd(root)
    } else {
        icp
    }
}

fn resolve_icp_artifact_root(options: &ListOptions) -> Result<PathBuf, ListCommandError> {
    let icp_root = resolve_live_icp_root()?;
    if let Ok(state) = read_installed_deployment_state_from_root(
        &state_network(options),
        &options.target,
        &icp_root,
    ) {
        return Ok(PathBuf::from(state.icp_root));
    }
    Ok(icp_root)
}

fn resolve_list_deployment(
    options: &ListOptions,
) -> Result<InstalledDeploymentResolution, ListCommandError> {
    let icp_root = resolve_live_icp_root()?;
    resolve_installed_deployment_from_root(
        &InstalledDeploymentRequest {
            deployment: options.target.clone(),
            network: state_network(options),
            icp: options.icp.clone(),
            detect_lost_local_root: true,
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
        ListCommandError::InstalledDeployment(InstalledDeploymentError::Icp(source)) => {
            let Some(hint) = source.external_output().and_then(root_registry_hint) else {
                return ListCommandError::InstalledDeployment(InstalledDeploymentError::Icp(
                    source,
                ));
            };
            ListCommandError::InstalledDeploymentHint {
                source: InstalledDeploymentError::Icp(source),
                hint,
            }
        }
        error => error,
    }
}

fn root_registry_hint(stderr: &str) -> Option<&'static str> {
    match classify_icp_diagnostic(stderr) {
        Some(IcpDiagnostic::CanisterIdMissing) => Some(
            "no root canister id exists for this deployment target. Use `canic fleet config <fleet-template>` for the selected fleet config, or run `canic install <fleet-template>` before querying the root registry.",
        ),
        Some(IcpDiagnostic::CanisterWasmMissing) => Some(
            "the root canister id exists but no Canic root code is installed. Run `canic install <name>`, then use `canic info list <name>`.",
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
