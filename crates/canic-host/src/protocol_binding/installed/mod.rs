//! Inspect the exact declaration retained by a converged Fleet.
//!
//! Fleet selection and immutable build validation remain with their existing owners.

use super::{ReleaseProtocolBindingError, resolve_release_registry_wire_schema};
use crate::{
    fleet_ensure::{CurrentFleetInventoryError, resolve_current_fleet},
    registry::RegistryEntry,
    release_set::BuiltWireSchema,
};
use std::path::Path;
use thiserror::Error;

#[cfg(test)]
mod tests;

/// Failure to select an installed declaration without substituting another build.
#[derive(Debug, Error)]
pub enum InstalledDeclarationError {
    #[error(transparent)]
    Fleet(#[from] CurrentFleetInventoryError),
    #[error("Fleet has no exact retained release build")]
    MissingRelease,
    #[error("Fleet has no unique target {0}; use an exact canister Principal")]
    Target(String),
    #[error(transparent)]
    Release(#[from] ReleaseProtocolBindingError),
}

/// Resolve a schema only when retained release authority matches the installed module.
pub fn load_installed_wire_schema(
    root: &Path,
    environment: &str,
    fleet: &str,
    target: &str,
) -> Result<BuiltWireSchema, InstalledDeclarationError> {
    let (entry, release) = resolve_target(root, environment, fleet, target)?;
    Ok(resolve_release_registry_wire_schema(root, release, &entry)?)
}

fn resolve_target(
    root: &Path,
    environment: &str,
    fleet: &str,
    target: &str,
) -> Result<(RegistryEntry, canic_contracts::ids::ReleaseBuildId), InstalledDeclarationError> {
    let current = resolve_current_fleet(root, environment, fleet)?;
    let entry = select_entry(&current.registry.entries, target)?;
    let release = current
        .plan
        .reviewed_desired
        .as_ref()
        .and_then(|reviewed| reviewed.desired().bootstrap.as_ref())
        .ok_or(InstalledDeclarationError::MissingRelease)?
        .release_build_id;
    Ok((entry.clone(), release))
}

fn select_entry<'a>(
    entries: &'a [RegistryEntry],
    target: &str,
) -> Result<&'a RegistryEntry, InstalledDeclarationError> {
    let principal = ic_platform::principal::ExternalPrincipal::from_text(target).ok();
    let mut matches = entries.iter().filter(|entry| match principal {
        Some(principal) => entry.pid == principal.to_text(),
        None => entry.role.as_deref() == Some(target),
    });
    let entry = matches
        .next()
        .ok_or_else(|| InstalledDeclarationError::Target(target.into()))?;
    if matches.next().is_some() {
        return Err(InstalledDeclarationError::Target(target.into()));
    }
    Ok(entry)
}
