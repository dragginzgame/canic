//! Module: restore::plan::mapping
//!
//! Responsibility: validate explicit restore source-to-target mappings.
//! Does not own: restore member resolution, operation ordering, or execution.
//! Boundary: rejects malformed mappings before restore plans are built.

use crate::{
    manifest::DeploymentBackupManifest,
    restore::{RestoreMapping, RestorePlanError},
};

use std::{collections::BTreeSet, str::FromStr};

use candid::Principal;

pub(super) fn validate_mapping(mapping: &RestoreMapping) -> Result<(), RestorePlanError> {
    let mut sources = BTreeSet::new();
    let mut targets = BTreeSet::new();

    for entry in &mapping.members {
        validate_principal("mapping.members[].source_canister", &entry.source_canister)?;
        validate_principal("mapping.members[].target_canister", &entry.target_canister)?;

        if !sources.insert(entry.source_canister.clone()) {
            return Err(RestorePlanError::DuplicateMappingSource(
                entry.source_canister.clone(),
            ));
        }

        if !targets.insert(entry.target_canister.clone()) {
            return Err(RestorePlanError::DuplicateMappingTarget(
                entry.target_canister.clone(),
            ));
        }
    }

    Ok(())
}

pub(super) fn validate_mapping_sources(
    manifest: &DeploymentBackupManifest,
    mapping: &RestoreMapping,
) -> Result<(), RestorePlanError> {
    let sources = manifest
        .deployment
        .members
        .iter()
        .map(|member| member.canister_id.as_str())
        .collect::<BTreeSet<_>>();

    for entry in &mapping.members {
        if !sources.contains(entry.source_canister.as_str()) {
            return Err(RestorePlanError::UnknownMappingSource(
                entry.source_canister.clone(),
            ));
        }
    }

    Ok(())
}

fn validate_principal(field: &'static str, value: &str) -> Result<(), RestorePlanError> {
    Principal::from_str(value)
        .map(|_| ())
        .map_err(|_| RestorePlanError::InvalidPrincipal {
            field,
            value: value.to_string(),
        })
}
