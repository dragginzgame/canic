use super::{
    ListCommandError,
    options::ListOptions,
    parse::{parse_canic_metadata_version_response, parse_cycle_balance_response},
    render::ReadyStatus,
    state_network,
};
use crate::support::registry_tree::visible_entries;
use canic_host::{
    format::{cycles_tc, wasm_size_label},
    icp::IcpCli,
    icp_config::resolve_current_canic_icp_root,
    installed_fleet::{
        InstalledFleetError, InstalledFleetRequest, InstalledFleetResolution,
        read_installed_fleet_state_from_root, resolve_installed_fleet_from_root,
    },
    registry::RegistryEntry,
    replica_query,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    sync::Arc,
    thread,
};

use super::options::ListSource;

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

pub(super) fn resolve_tree_anchor(options: &ListOptions) -> Option<String> {
    options.subtree.clone()
}

pub(super) fn list_ready_statuses(
    options: &ListOptions,
    registry: &[RegistryEntry],
    canister: Option<&str>,
) -> Result<BTreeMap<String, ReadyStatus>, ListCommandError> {
    if replica_query::should_use_local_replica_query(options.network.as_deref()) {
        return local_ready_statuses(options, registry, canister);
    }

    let icp_root = resolve_live_icp_root(options);
    let mut statuses = BTreeMap::new();
    for entry in visible_entries(registry, canister)? {
        statuses.insert(
            entry.pid.clone(),
            check_ready_status(options, icp_root.as_deref(), &entry.pid)?,
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
    let icp_root = resolve_live_icp_root(options);
    collect_visible_optional_values(registry, canister, move |pid| {
        query_cycle_balance_endpoint(&icp, network.clone(), icp_root.as_deref(), &pid)
            .map(cycles_tc)
    })
}

pub(super) fn list_canic_versions(
    options: &ListOptions,
    registry: &[RegistryEntry],
    canister: Option<&str>,
) -> Result<BTreeMap<String, String>, ListCommandError> {
    let icp = options.icp.clone();
    let network = options.network.clone();
    let icp_root = resolve_live_icp_root(options);
    collect_visible_optional_values(registry, canister, move |pid| {
        query_canic_metadata_version(&icp, network.clone(), icp_root.as_deref(), &pid)
    })
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
) -> BTreeMap<String, String> {
    let Some(root) = resolve_icp_artifact_root(options) else {
        return BTreeMap::new();
    };
    let network = state_network(options);
    registry
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
        .collect()
}

fn check_ready_status(
    options: &ListOptions,
    icp_root: Option<&std::path::Path>,
    canister: &str,
) -> Result<ReadyStatus, ListCommandError> {
    let mut icp = IcpCli::new(&options.icp, None, options.network.clone());
    if let Some(root) = icp_root {
        icp = icp.with_cwd(root);
    }
    let Ok(output) = icp.canister_call_output(canister, "canic_ready", Some("json")) else {
        return Ok(ReadyStatus::Error);
    };
    let data = serde_json::from_str::<serde_json::Value>(&output)?;
    Ok(if replica_query::parse_ready_json_value(&data) {
        ReadyStatus::Ready
    } else {
        ReadyStatus::NotReady
    })
}

fn local_ready_statuses(
    options: &ListOptions,
    registry: &[RegistryEntry],
    canister: Option<&str>,
) -> Result<BTreeMap<String, ReadyStatus>, ListCommandError> {
    let network = options.network.clone();
    let icp_root = resolve_live_icp_root(options);
    collect_visible_values(registry, canister, move |pid| {
        match icp_root.as_deref().map_or_else(
            || replica_query::query_ready(network.as_deref(), &pid),
            |root| replica_query::query_ready_from_root(network.as_deref(), &pid, root),
        ) {
            Ok(true) => ReadyStatus::Ready,
            Ok(false) => ReadyStatus::NotReady,
            Err(_) => ReadyStatus::Error,
        }
    })
}

fn collect_visible_optional_values<T, F>(
    registry: &[RegistryEntry],
    canister: Option<&str>,
    query: F,
) -> Result<BTreeMap<String, T>, ListCommandError>
where
    T: Send + 'static,
    F: Fn(String) -> Option<T> + Send + Sync + 'static,
{
    let values = collect_visible_values(registry, canister, query)?;
    Ok(values
        .into_iter()
        .filter_map(|(pid, value)| value.map(|value| (pid, value)))
        .collect())
}

fn collect_visible_values<T, F>(
    registry: &[RegistryEntry],
    canister: Option<&str>,
    query: F,
) -> Result<BTreeMap<String, T>, ListCommandError>
where
    T: Send + 'static,
    F: Fn(String) -> T + Send + Sync + 'static,
{
    let query = Arc::new(query);
    let mut handles = Vec::new();
    for entry in visible_entries(registry, canister)? {
        let pid = entry.pid.clone();
        let query = Arc::clone(&query);
        handles.push(thread::spawn(move || {
            let value = query(pid.clone());
            (pid, value)
        }));
    }

    Ok(handles
        .into_iter()
        .filter_map(|handle| handle.join().ok())
        .collect())
}

fn query_cycle_balance_endpoint(
    icp: &str,
    network: Option<String>,
    icp_root: Option<&std::path::Path>,
    canister: &str,
) -> Option<u128> {
    let mut icp = IcpCli::new(icp, None, network);
    if let Some(root) = icp_root {
        icp = icp.with_cwd(root);
    }
    icp.canister_call_output(
        canister,
        canic_core::protocol::CANIC_CYCLE_BALANCE,
        Some("json"),
    )
    .ok()
    .and_then(|output| parse_cycle_balance_response(&output))
}

fn query_canic_metadata_version(
    icp: &str,
    network: Option<String>,
    icp_root: Option<&std::path::Path>,
    canister: &str,
) -> Option<String> {
    let mut icp = IcpCli::new(icp, None, network);
    if let Some(root) = icp_root {
        icp = icp.with_cwd(root);
    }
    icp.canister_call_output(canister, canic_core::protocol::CANIC_METADATA, Some("json"))
        .ok()
        .and_then(|output| parse_canic_metadata_version_response(&output))
}

fn resolve_icp_artifact_root(options: &ListOptions) -> Option<PathBuf> {
    let icp_root = resolve_live_icp_root(options)?;
    if let Ok(state) =
        read_installed_fleet_state_from_root(&state_network(options), &options.fleet, &icp_root)
    {
        return Some(PathBuf::from(state.icp_root));
    }
    Some(icp_root)
}

fn resolve_list_fleet(options: &ListOptions) -> Result<InstalledFleetResolution, ListCommandError> {
    let icp_root = resolve_live_icp_root(options)
        .ok_or_else(|| ListCommandError::InstallState("could not resolve ICP root".to_string()))?;
    resolve_installed_fleet_from_root(
        &InstalledFleetRequest {
            fleet: options.fleet.clone(),
            network: state_network(options),
            icp: options.icp.clone(),
            detect_lost_local_root: true,
        },
        &icp_root,
    )
    .map_err(list_installed_fleet_error)
    .map_err(add_root_registry_hint)
}

fn resolve_live_icp_root(options: &ListOptions) -> Option<PathBuf> {
    resolve_current_canic_icp_root().ok().or_else(|| {
        read_installed_fleet_state_from_root(
            &state_network(options),
            &options.fleet,
            &std::env::current_dir().ok()?,
        )
        .ok()
        .map(|state| PathBuf::from(state.icp_root))
    })
}

fn list_installed_fleet_error(error: InstalledFleetError) -> ListCommandError {
    match error {
        InstalledFleetError::NoInstalledFleet { network, fleet } => {
            ListCommandError::NoInstalledFleet { network, fleet }
        }
        InstalledFleetError::InstallState(error) => ListCommandError::InstallState(error),
        InstalledFleetError::ReplicaQuery(error) => ListCommandError::ReplicaQuery(error),
        InstalledFleetError::IcpFailed { command, stderr } => {
            ListCommandError::IcpFailed { command, stderr }
        }
        InstalledFleetError::LostLocalFleet {
            fleet,
            network,
            root,
        } => ListCommandError::LostLocalFleet {
            fleet,
            network,
            root,
        },
        InstalledFleetError::Registry(error) => ListCommandError::Registry(error),
        InstalledFleetError::Io(error) => ListCommandError::Io(error),
    }
}

fn add_root_registry_hint(error: ListCommandError) -> ListCommandError {
    let ListCommandError::IcpFailed { command, stderr } = error else {
        return error;
    };

    let Some(hint) = root_registry_hint(&stderr) else {
        return ListCommandError::IcpFailed { command, stderr };
    };

    ListCommandError::IcpFailed {
        command,
        stderr: format!("{stderr}\nHint: {hint}\n"),
    }
}

fn root_registry_hint(stderr: &str) -> Option<&'static str> {
    if stderr.contains("Cannot find canister id") {
        return Some(
            "no root canister id exists for this fleet. Use `canic config <name>` for the selected fleet config, or run `canic install <name>` before querying the root registry.",
        );
    }

    if stderr.contains("contains no Wasm module") || stderr.contains("wasm-module-not-found") {
        return Some(
            "the root canister id exists but no Canic root code is installed. Run `canic install <name>`, then use `canic list <name>`.",
        );
    }

    None
}

#[cfg(test)]
mod tests;
