//! Inspect the exact declaration retained by a converged Fleet.
//!
//! Fleet selection and immutable build validation remain with their existing owners.

use super::{ProtocolBindingError, verify_protocol_binding};
use crate::{
    fleet_ensure::{CurrentFleetInventoryError, resolve_current_fleet},
    registry::RegistryEntry,
    release_set::{BuiltCandid, BuiltCandidError, load_built_candid},
};
use std::path::Path;
use thiserror::Error;

#[cfg(test)]
mod tests;

/// Failure to select an installed declaration without substituting another build.
#[derive(Debug, Error)]
pub enum InstalledCandidError {
    #[error(transparent)]
    Fleet(#[from] CurrentFleetInventoryError),
    #[error(transparent)]
    Build(#[from] BuiltCandidError),
    #[error(transparent)]
    Binding(#[from] ProtocolBindingError),
    #[error("Fleet has no exact retained release build")]
    MissingRelease,
    #[error("Fleet has no unique target {0}; use an exact canister Principal")]
    Target(String),
}

/// Load a deployed role's declaration using retained release and protocol authority.
pub fn load_installed_candid(
    root: &Path,
    environment: &str,
    fleet: &str,
    target: &str,
) -> Result<BuiltCandid, InstalledCandidError> {
    let current = resolve_current_fleet(root, environment, fleet)?;
    let entry = select_entry(&current.registry.entries, target)?;
    let release = current
        .plan
        .reviewed_desired
        .as_ref()
        .and_then(|reviewed| reviewed.desired().bootstrap.as_ref())
        .ok_or(InstalledCandidError::MissingRelease)?
        .release_build_id;
    let binding =
        entry
            .protocol_binding
            .clone()
            .ok_or_else(|| ProtocolBindingError::MissingBinding {
                canister: entry.pid.clone(),
            })?;
    if entry.role.as_deref() != Some(binding.role.as_str()) {
        return Err(ProtocolBindingError::RoleMismatch {
            canister: entry.pid.clone(),
            registry_role: entry.role.clone().unwrap_or_default(),
            protocol_role: binding.role.to_string(),
        }
        .into());
    }
    let built = load_built_candid(root, release, binding.role.as_str())?;
    verify_protocol_binding(&entry.pid, &binding, built.candid.as_bytes())?;
    Ok(built)
}

fn select_entry<'a>(
    entries: &'a [RegistryEntry],
    target: &str,
) -> Result<&'a RegistryEntry, InstalledCandidError> {
    let principal = ic_platform::principal::ExternalPrincipal::from_text(target).ok();
    let mut matches = entries.iter().filter(|entry| match principal {
        Some(principal) => entry.pid == principal.to_text(),
        None => entry.role.as_deref() == Some(target),
    });
    let entry = matches
        .next()
        .ok_or_else(|| InstalledCandidError::Target(target.into()))?;
    if matches.next().is_some() {
        return Err(InstalledCandidError::Target(target.into()));
    }
    Ok(entry)
}
