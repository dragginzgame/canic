//! Module: ops::storage::fleet_activation::fixture
//!
//! Responsibility: convert and validate immutable target source assignments.
//! Does not own: publication, grant issuance, import progress or data readiness.
//! Boundary: every assignment must match the sole installed activation identity.

#[cfg(test)]
mod tests;

use crate::{
    dto::fixture_provisioning::{
        FixtureAssignment, FixtureChunkDescriptor, FixtureDescriptor, FixtureGrant,
        FixtureTargetBinding,
    },
    ops::{fixture_content, storage::fleet_activation::FleetActivationOpsError},
    storage::stable::fleet_activation::{
        FleetActivationRecord, FleetActivationStateRecord,
        fixture::{FixtureAssignmentRecord, FixtureChunkRecord},
    },
};
use candid::Principal;

/// Reject substituted descriptors and assignments from another installation or release.
pub fn validate(record: &FleetActivationRecord) -> Result<(), FleetActivationOpsError> {
    let Some(runtime) = &record.component_runtime else {
        return Ok(());
    };
    let Some(fixture) = &runtime.fixture else {
        return Ok(());
    };
    let identity = match &record.state {
        FleetActivationStateRecord::Prepared { identity, .. }
        | FleetActivationStateRecord::Active { identity, .. } => identity,
    };
    let assignment = to_dto(fixture);
    let expected = FixtureTargetBinding {
        target: runtime.binding.clone(),
        installation: identity.operation_id,
        release_build_id: identity.release_build_id,
        content_id: fixture_content::content_id(&assignment.descriptor).map_err(|_| invalid())?,
    };
    if assignment.grant.binding != expected {
        return Err(invalid());
    }
    let (component, target) = match &runtime.binding {
        crate::ids::ManagedCanisterBinding::Component(binding) => (binding, binding.canister_id),
        crate::ids::ManagedCanisterBinding::ComponentChild(binding) => {
            (&binding.component, binding.canister_id)
        }
    };
    let invalid_store = [
        Principal::anonymous(),
        Principal::management_canister(),
        target,
        component.fleet_subnet_root,
    ]
    .contains(&assignment.store);
    if invalid_store || !assignment.grant.enabled || assignment.grant.revision == 0 {
        return Err(invalid());
    }
    if component.authority.binding.fleet != identity.fleet {
        return Err(invalid());
    }
    Ok(())
}

/// Convert the Root-selected payload into the protected current storage schema.
pub fn to_record(assignment: FixtureAssignment) -> FixtureAssignmentRecord {
    let FixtureAssignment {
        store,
        grant,
        descriptor,
    } = assignment;
    let FixtureTargetBinding {
        target,
        installation,
        release_build_id,
        content_id,
    } = grant.binding;
    FixtureAssignmentRecord {
        store,
        target,
        installation,
        release_build_id,
        content_id,
        grant_revision: grant.revision,
        grant_enabled: grant.enabled,
        schema_version: descriptor.schema_version,
        format_hash: descriptor.format_hash,
        encoded_length: descriptor.encoded_length,
        chunks: descriptor
            .chunks
            .into_iter()
            .map(|chunk| FixtureChunkRecord {
                digest: chunk.digest,
                length: chunk.length,
            })
            .collect(),
        completion_summary: descriptor.completion_summary,
    }
}

/// Project source authority without inventing an application progress or readiness flag.
pub fn to_dto(record: &FixtureAssignmentRecord) -> FixtureAssignment {
    FixtureAssignment {
        store: record.store,
        grant: FixtureGrant {
            revision: record.grant_revision,
            enabled: record.grant_enabled,
            binding: FixtureTargetBinding {
                target: record.target.clone(),
                installation: record.installation,
                release_build_id: record.release_build_id,
                content_id: record.content_id,
            },
        },
        descriptor: FixtureDescriptor {
            schema_version: record.schema_version,
            format_hash: record.format_hash,
            encoded_length: record.encoded_length,
            chunks: record
                .chunks
                .iter()
                .map(|chunk| FixtureChunkDescriptor {
                    digest: chunk.digest,
                    length: chunk.length,
                })
                .collect(),
            completion_summary: record.completion_summary,
        },
    }
}

fn invalid() -> FleetActivationOpsError {
    FleetActivationOpsError::InvalidRecord {
        reason: "fixture assignment does not match the protected installation".to_string(),
    }
}
