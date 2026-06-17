//! Module: plan::build::selector
//!
//! Responsibility: resolve operator backup selectors against registry entries.
//! Does not own: registry discovery, plan construction, or target expansion.
//! Boundary: returns one selected canister id or a typed plan error.

use crate::{
    plan::{BackupPlanError, validation::validate_nonempty},
    registry::RegistryEntry,
};

use std::str::FromStr;

use candid::Principal;

/// Resolve an operator selector to one concrete live registry canister id.
pub fn resolve_backup_selector(
    registry: &[RegistryEntry],
    selector: &str,
) -> Result<String, BackupPlanError> {
    validate_nonempty("selector", selector)?;
    if Principal::from_str(selector).is_ok() {
        return registry
            .iter()
            .find(|entry| entry.pid == selector)
            .map(|entry| entry.pid.clone())
            .ok_or_else(|| BackupPlanError::UnknownSelector(selector.to_string()));
    }

    let matches = registry
        .iter()
        .filter(|entry| entry.role.as_deref() == Some(selector))
        .map(|entry| entry.pid.clone())
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [canister] => Ok(canister.clone()),
        [] => Err(BackupPlanError::UnknownSelector(selector.to_string())),
        _ => Err(BackupPlanError::AmbiguousSelector {
            selector: selector.to_string(),
            matches,
        }),
    }
}
