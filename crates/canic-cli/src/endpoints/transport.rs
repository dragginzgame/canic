use crate::{
    cli::defaults::local_network,
    endpoints::{
        CANDID_SERVICE_METADATA, EndpointsCommandError, EndpointsOptions,
        model::{EndpointReport, EndpointTarget},
        parse::parse_candid_service_endpoints,
    },
};
use canic_host::{
    icp::IcpCli,
    icp_config::resolve_current_canic_icp_root,
    installed_fleet::{InstalledFleetRequest, resolve_installed_fleet_from_root},
    registry::RegistryEntry,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn endpoint_report(
    options: &EndpointsOptions,
) -> Result<EndpointReport, EndpointsCommandError> {
    let target = resolve_endpoint_target(options);
    if let Ok(target) = &target
        && let Ok(candid) = read_live_candid(options, target)
    {
        return Ok(EndpointReport {
            source: format!("{} metadata", options.canister),
            endpoints: parse_candid_service_endpoints(&candid)?,
        });
    }

    let role = target
        .ok()
        .and_then(|target| target.role)
        .or_else(|| (!is_principal_like(&options.canister)).then(|| options.canister.clone()));
    let Some(role) = role else {
        return Err(EndpointsCommandError::NoInterfaceArtifact {
            fleet: options.fleet.clone(),
            canister: options.canister.clone(),
        });
    };
    let path = resolve_role_did(options, &role)?;
    let candid = read_did(&path)?;
    Ok(EndpointReport {
        source: path.display().to_string(),
        endpoints: parse_candid_service_endpoints(&candid)?,
    })
}

fn read_live_candid(
    options: &EndpointsOptions,
    target: &EndpointTarget,
) -> Result<String, Box<dyn std::error::Error>> {
    let root = resolve_endpoint_icp_root()?;
    Ok(IcpCli::new(&options.icp, None, options.network.clone())
        .with_cwd(root)
        .canister_metadata_output(&target.canister, CANDID_SERVICE_METADATA)?)
}

fn resolve_endpoint_target(
    options: &EndpointsOptions,
) -> Result<EndpointTarget, Box<dyn std::error::Error>> {
    if is_principal_like(&options.canister) {
        let role = load_fleet_registry(options).ok().and_then(|registry| {
            registry
                .into_iter()
                .find(|entry| entry.pid == options.canister)
                .and_then(|entry| entry.role)
        });
        return Ok(EndpointTarget {
            canister: options.canister.clone(),
            role,
        });
    }

    let registry = load_fleet_registry(options)?;
    let entry = registry
        .iter()
        .find(|entry| entry.role.as_deref() == Some(options.canister.as_str()))
        .ok_or_else(|| -> Box<dyn std::error::Error> {
            format!(
                "role {} was not found in fleet {}",
                options.canister, options.fleet
            )
            .into()
        })?;
    Ok(EndpointTarget {
        canister: entry.pid.clone(),
        role: entry.role.clone(),
    })
}

fn load_fleet_registry(
    options: &EndpointsOptions,
) -> Result<Vec<RegistryEntry>, Box<dyn std::error::Error>> {
    let request = InstalledFleetRequest {
        fleet: options.fleet.clone(),
        network: state_network(options),
        icp: options.icp.clone(),
        detect_lost_local_root: false,
    };
    let root = resolve_endpoint_icp_root()?;
    Ok(resolve_installed_fleet_from_root(&request, &root)?
        .registry
        .entries)
}

fn resolve_role_did(
    options: &EndpointsOptions,
    role: &str,
) -> Result<PathBuf, EndpointsCommandError> {
    let root = resolve_endpoint_icp_root().unwrap_or_else(|_| PathBuf::from("."));
    for network in artifact_network_candidates(options) {
        let path = root
            .join(".icp")
            .join(&network)
            .join("canisters")
            .join(role)
            .join(format!("{role}.did"));
        if path.is_file() {
            return Ok(path);
        }
    }

    Err(EndpointsCommandError::MissingRoleArtifact {
        role: role.to_string(),
        root: root.display().to_string(),
    })
}

fn resolve_endpoint_icp_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    resolve_current_canic_icp_root(None).map_err(Into::into)
}

fn artifact_network_candidates(options: &EndpointsOptions) -> Vec<String> {
    let mut networks = Vec::new();
    if let Some(network) = &options.network {
        networks.push(network.clone());
    }
    networks.push(local_network());
    networks.sort();
    networks.dedup();
    networks
}

fn state_network(options: &EndpointsOptions) -> String {
    options.network.clone().unwrap_or_else(local_network)
}

fn read_did(path: &Path) -> Result<String, EndpointsCommandError> {
    fs::read_to_string(path).map_err(|source| EndpointsCommandError::ReadDid {
        path: path.display().to_string(),
        source,
    })
}

fn is_principal_like(value: &str) -> bool {
    value.contains('-')
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
}
