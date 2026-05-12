mod error;
mod mapping;
mod members;
mod ordering;
mod summary;
mod types;

pub use error::RestorePlanError;
pub use types::*;

use crate::manifest::FleetBackupManifest;
use mapping::{validate_mapping, validate_mapping_sources};
use members::resolve_members;
use ordering::{order_members, restore_ordering_summary};
use summary::{
    restore_identity_summary, restore_operation_summary, restore_readiness_summary,
    restore_snapshot_summary, restore_verification_summary,
};

///
/// RestorePlanner
///

pub struct RestorePlanner;

impl RestorePlanner {
    /// Build a no-mutation restore plan from the manifest and optional target mapping.
    pub fn plan(
        manifest: &FleetBackupManifest,
        mapping: Option<&RestoreMapping>,
    ) -> Result<RestorePlan, RestorePlanError> {
        manifest.validate()?;
        if let Some(mapping) = mapping {
            validate_mapping(mapping)?;
            validate_mapping_sources(manifest, mapping)?;
        }

        let members = resolve_members(manifest, mapping)?;
        let identity_summary = restore_identity_summary(&members, mapping.is_some());
        let snapshot_summary = restore_snapshot_summary(&members);
        let verification_summary = restore_verification_summary(manifest, &members);
        let readiness_summary = restore_readiness_summary(&snapshot_summary, &verification_summary);
        let members = order_members(members)?;
        let ordering_summary = restore_ordering_summary(&members);
        let operation_summary =
            restore_operation_summary(manifest.fleet.members.len(), &verification_summary);

        Ok(RestorePlan {
            backup_id: manifest.backup_id.clone(),
            source_environment: manifest.source.environment.clone(),
            source_root_canister: manifest.source.root_canister.clone(),
            topology_hash: manifest.fleet.topology_hash.clone(),
            member_count: manifest.fleet.members.len(),
            identity_summary,
            snapshot_summary,
            verification_summary,
            readiness_summary,
            operation_summary,
            ordering_summary,
            fleet_verification_checks: manifest.verification.fleet_checks.clone(),
            members,
        })
    }
}
