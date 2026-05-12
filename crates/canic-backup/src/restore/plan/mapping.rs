use super::{RestoreMapping, RestorePlanError};
use crate::manifest::FleetBackupManifest;
use candid::Principal;
use std::{collections::BTreeSet, str::FromStr};

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
    manifest: &FleetBackupManifest,
    mapping: &RestoreMapping,
) -> Result<(), RestorePlanError> {
    let sources = manifest
        .fleet
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
