//! Module: canic_cli::support::fleet_recipient
//!
//! Responsibility: resolve compact Fleet recipients for token and cycles commands.
//! Does not own: inventory persistence, ledger behavior, or transfer execution.
//! Boundary: delegates current Fleet authority lookup to Host and returns a CLI recipient.

#[cfg(test)]
mod tests;

use crate::support::icp_target::IcpTargetOptions;
use canic_host::{
    fleet_ensure::{CurrentFleetInventoryError, resolve_current_fleet},
    registry::RegistryEntry,
};
use std::path::Path;

///
/// FleetRecipientError
///
/// Typed CLI lookup failures mapped by each consuming command to its own diagnostics.
///

#[derive(Debug)]
pub enum FleetRecipientError {
    AmbiguousRole { fleet: String, role: String },
    CurrentFleet(CurrentFleetInventoryError),
    InvalidRecipient,
    UnknownTarget { fleet: String, target: String },
}

/// Resolve a compact Fleet target, preserving raw ICP recipients unchanged.
pub fn resolve(
    target: &IcpTargetOptions,
    root: &Path,
    receiver: &str,
) -> Result<String, FleetRecipientError> {
    let Some((fleet, canister_or_role)) = split_fleet_target(receiver)? else {
        return Ok(receiver.to_string());
    };
    let current = resolve_current_fleet(root, &target.environment, fleet)
        .map_err(FleetRecipientError::CurrentFleet)?;
    let root_canister_id = current
        .topology
        .unique_fleet_subnet_root(fleet)
        .map_err(FleetRecipientError::CurrentFleet)?;
    resolve_canister_or_role(
        fleet,
        canister_or_role,
        root_canister_id,
        &current.registry.entries,
    )
}

fn split_fleet_target(receiver: &str) -> Result<Option<(&str, &str)>, FleetRecipientError> {
    let Some((fleet, canister_or_role)) = receiver.split_once('/') else {
        return Ok(None);
    };
    if fleet.is_empty() || canister_or_role.is_empty() || canister_or_role.contains('/') {
        return Err(FleetRecipientError::InvalidRecipient);
    }
    Ok(Some((fleet, canister_or_role)))
}

fn resolve_canister_or_role(
    fleet: &str,
    target: &str,
    root_canister_id: &str,
    registry: &[RegistryEntry],
) -> Result<String, FleetRecipientError> {
    if target == "root" || target == root_canister_id {
        return Ok(root_canister_id.to_string());
    }
    if registry.iter().any(|entry| entry.pid == target) {
        return Ok(target.to_string());
    }
    resolve_role_principal(fleet, target, registry)
}

fn resolve_role_principal(
    fleet: &str,
    role: &str,
    registry: &[RegistryEntry],
) -> Result<String, FleetRecipientError> {
    let matches = registry
        .iter()
        .filter(|entry| entry.role.as_deref() == Some(role))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [entry] => Ok(entry.pid.clone()),
        [] => Err(FleetRecipientError::UnknownTarget {
            fleet: fleet.to_string(),
            target: role.to_string(),
        }),
        _ => Err(FleetRecipientError::AmbiguousRole {
            fleet: fleet.to_string(),
            role: role.to_string(),
        }),
    }
}
