use super::{
    ListCommandError,
    options::{ListOptions, ListSource},
    render::ReadyStatus,
    state_environment,
};
use crate::{
    cli::defaults::local_environment,
    support::{parallel::collect_ordered_reports, registry_tree::visible_entries},
};
use canic_contracts::dto::observability::{
    CanisterObservabilityRequest, CanisterObservabilityResponse,
};
use canic_host::{
    canic_metadata::query_canic_metadata_version,
    canister_ready::query_canister_ready,
    fleet_ensure::{CurrentFleetResolution, resolve_current_fleet},
    format::{cycles_tc, wasm_size_label},
    icp::IcpCli,
    icp_config::resolve_current_canic_icp_root,
    observability::observe_fleet_canister,
    protocol_binding::resolve_current_registry_wire_schema,
    registry::RegistryEntry,
    release_set::artifact_root_path,
    replica_query,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

const OBSERVATION_ERROR: &str = "error";

pub(super) fn load_registry_entries(
    options: &ListOptions,
) -> Result<CurrentFleetResolution, ListCommandError> {
    match options.source {
        ListSource::FleetInventory => resolve_list_fleet(options),
        ListSource::Config => {
            unreachable!("config source does not use registry entries")
        }
    }
}

pub(super) fn list_ready_statuses(
    options: &ListOptions,
    fleet: &CurrentFleetResolution,
    canister: Option<&str>,
) -> Result<BTreeMap<String, ReadyStatus>, ListCommandError> {
    let registry = fleet.registry.entries.clone();
    let icp_root = resolve_live_icp_root()?;
    if replica_query::uses_local_replica_transport(options.environment.as_deref(), Some(&icp_root))?
    {
        return local_ready_statuses(options, fleet, canister, &icp_root);
    }

    let mut statuses = BTreeMap::new();
    for entry in visible_entries(&registry, canister)? {
        statuses.insert(
            entry.pid.clone(),
            check_ready_status(options, &icp_root, fleet, entry),
        );
    }
    Ok(statuses)
}

pub(super) fn list_cycle_balances(
    options: &ListOptions,
    fleet: &CurrentFleetResolution,
    canister: Option<&str>,
) -> Result<BTreeMap<String, String>, ListCommandError> {
    let icp = options.icp.clone();
    let environment = options.environment.clone();
    let icp_root = resolve_live_icp_root()?;
    let fleet = fleet.clone();
    let registry = fleet.registry.entries.clone();
    collect_visible_entry_values(
        &registry,
        canister,
        OBSERVATION_ERROR.to_string(),
        move |entry| {
            cycle_balance_label_endpoint(&icp, environment.clone(), &icp_root, &fleet, &entry)
        },
    )
}

pub(super) fn list_canic_versions(
    options: &ListOptions,
    fleet: &CurrentFleetResolution,
    canister: Option<&str>,
) -> Result<BTreeMap<String, String>, ListCommandError> {
    let icp = options.icp.clone();
    let environment = options.environment.clone();
    let registry = fleet.registry.entries.clone();
    let icp_root = resolve_live_icp_root()?;
    let fleet = fleet.clone();
    collect_visible_entry_values(
        &registry,
        canister,
        OBSERVATION_ERROR.to_string(),
        move |entry| {
            canic_version_label_endpoint(&icp, environment.clone(), &icp_root, &fleet, &entry)
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
    icp_root: &Path,
    fleet: &CurrentFleetResolution,
    entry: &RegistryEntry,
) -> ReadyStatus {
    if resolve_current_registry_wire_schema(icp_root, fleet, entry).is_err() {
        return ReadyStatus::Error;
    }
    let Some(binding) = entry.protocol_binding.as_ref() else {
        return ReadyStatus::Error;
    };
    let icp = live_icp(&options.icp, options.environment.clone(), Some(icp_root));
    match query_canister_ready(
        &icp,
        &entry.pid,
        &state_environment(options),
        Some(icp_root),
        binding,
    ) {
        Ok(true) => ReadyStatus::Ready,
        Ok(false) => ReadyStatus::NotReady,
        Err(_) => ReadyStatus::Error,
    }
}

fn local_ready_statuses(
    options: &ListOptions,
    fleet: &CurrentFleetResolution,
    canister: Option<&str>,
    icp_root: &Path,
) -> Result<BTreeMap<String, ReadyStatus>, ListCommandError> {
    let options = options.clone();
    let icp_root = icp_root.to_path_buf();
    let registry = fleet.registry.entries.clone();
    let fleet = fleet.clone();
    collect_visible_entry_values(&registry, canister, ReadyStatus::Error, move |entry| {
        check_ready_status(&options, &icp_root, &fleet, &entry)
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
    let entries = visible_entries(registry, canister)?;
    let values = collect_ordered_reports(
        &entries,
        |entry| query((**entry).clone()),
        |_| worker_panic_value.clone(),
    );
    Ok(entries
        .into_iter()
        .map(|entry| entry.pid.clone())
        .zip(values)
        .collect())
}

fn cycle_balance_label_endpoint(
    icp: &str,
    environment: Option<String>,
    icp_root: &Path,
    fleet: &CurrentFleetResolution,
    entry: &RegistryEntry,
) -> String {
    let environment = environment.unwrap_or_else(local_environment);
    let icp = live_icp(icp, Some(environment), Some(icp_root));
    let Ok(CanisterObservabilityResponse::CycleBalance(response)) = observe_fleet_canister(
        &icp,
        icp_root,
        fleet,
        entry,
        CanisterObservabilityRequest::CycleBalance,
    ) else {
        return OBSERVATION_ERROR.to_string();
    };
    cycles_tc(response.cycles)
}

fn canic_version_label_endpoint(
    icp: &str,
    environment: Option<String>,
    icp_root: &Path,
    fleet: &CurrentFleetResolution,
    entry: &RegistryEntry,
) -> String {
    let environment = environment.unwrap_or_else(local_environment);
    if resolve_current_registry_wire_schema(icp_root, fleet, entry).is_err() {
        return OBSERVATION_ERROR.to_string();
    }
    let Some(binding) = entry.protocol_binding.as_ref() else {
        return OBSERVATION_ERROR.to_string();
    };
    let icp = live_icp(icp, Some(environment), Some(icp_root));
    query_canic_metadata_version(&icp, &entry.pid, binding)
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

fn resolve_list_fleet(options: &ListOptions) -> Result<CurrentFleetResolution, ListCommandError> {
    let icp_root = resolve_live_icp_root()?;
    resolve_current_fleet(&icp_root, &state_environment(options), &options.target)
        .map_err(ListCommandError::from)
}

fn resolve_live_icp_root() -> Result<PathBuf, ListCommandError> {
    resolve_current_canic_icp_root().map_err(ListCommandError::from)
}

#[cfg(test)]
mod tests;
