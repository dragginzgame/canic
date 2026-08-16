//! Module: ops::fleet_registry_mirror
//!
//! Responsibility: read and commit root-local Fleet Registry mirror state.
//! Does not own: snapshot validation, Coordinator calls, or lifecycle orchestration.
//! Boundary: converts stable records into read-only views before workflow use.

use crate::{
    storage::stable::fleet_registry_mirror::{
        RootFleetRegistryActiveRecord, RootFleetRegistryCandidateRecord,
        RootFleetRegistryMirrorStore,
    },
    view::fleet_registry_mirror::{
        RootFleetRegistryActiveView, RootFleetRegistryCandidateView, RootFleetRegistryMirrorView,
        ValidatedRootFleetRegistryMirrorView,
    },
};
use canic_core::{
    control_plane_support::{
        error::InternalError,
        ops::{config::ConfigOps, fleet_registry::FleetRegistryOps},
    },
    dto::{
        fleet_registry::{
            FleetDirectorySnapshot, FleetRegistryManifest, FleetRegistrySnapshotResponse,
            FleetRegistryVersion, FleetSubnetRootEntry, FleetSubnetRootSnapshotAcknowledgement,
            FleetSubnetRootStatus,
        },
        fleet_subnet_root::FleetSubnetRootAuthority,
    },
};

#[derive(Eq, PartialEq)]
struct CanonicalMirrorEvidence<'a> {
    manifest: &'a FleetRegistryManifest,
    version: &'a FleetRegistryVersion,
    directory: &'a FleetDirectorySnapshot,
}

///
/// FleetRegistryMirrorOps
///
/// Single-step root-local Fleet Registry mirror storage operations.
///

pub struct FleetRegistryMirrorOps;

impl FleetRegistryMirrorOps {
    pub(crate) fn current() -> RootFleetRegistryMirrorView {
        let data = RootFleetRegistryMirrorStore::export();
        RootFleetRegistryMirrorView {
            candidate: data.candidate.map(candidate_record_to_view),
            active: data.active.map(active_record_to_view),
        }
    }

    pub(crate) fn validated_current(
        authority: &FleetSubnetRootAuthority,
        root: candid::Principal,
    ) -> Result<ValidatedRootFleetRegistryMirrorView, InternalError> {
        if authority.binding.fleet_subnet_root != root {
            return Err(InternalError::invalid_input());
        }
        let active = Self::current()
            .active
            .ok_or_else(InternalError::unavailable)?;
        let topology = ConfigOps::component_topology()?;
        FleetRegistryOps::validate(
            &authority.binding.authority,
            &topology,
            &active.snapshot.registry,
        )?;
        let manifest = FleetRegistryOps::manifest(
            &authority.binding.authority,
            &topology,
            &active.snapshot.registry,
        )?;
        let version = FleetRegistryOps::version(
            &authority.binding.authority,
            &topology,
            &active.snapshot.registry,
        )?;
        let root_entry = validated_root_entry(
            authority,
            root,
            &active.snapshot.registry.fleet_subnet_roots,
        )?;
        let directory = FleetRegistryOps::directory_for_root(
            &authority.binding.authority,
            &topology,
            &active.snapshot.registry,
            root,
        )?;
        let stored = CanonicalMirrorEvidence {
            manifest: &active.snapshot.manifest,
            version: &active.snapshot.version,
            directory: &active.directory,
        };
        let canonical = CanonicalMirrorEvidence {
            manifest: &manifest,
            version: &version,
            directory: &directory,
        };
        if stored != canonical || !version_precedes(&active.previous_registry, &version) {
            return Err(InternalError::invariant());
        }
        Ok(ValidatedRootFleetRegistryMirrorView { active, root_entry })
    }

    pub(crate) fn commit_candidate(
        snapshot: FleetRegistrySnapshotResponse,
        acknowledgement: Option<FleetSubnetRootSnapshotAcknowledgement>,
    ) {
        RootFleetRegistryMirrorStore::commit_candidate(RootFleetRegistryCandidateRecord {
            snapshot,
            acknowledgement,
        });
    }

    pub(crate) fn commit_active(
        previous_registry: FleetRegistryVersion,
        snapshot: FleetRegistrySnapshotResponse,
        directory: FleetDirectorySnapshot,
    ) {
        RootFleetRegistryMirrorStore::commit_active(RootFleetRegistryActiveRecord {
            previous_registry,
            snapshot,
            directory,
        });
    }
}

fn validated_root_entry(
    authority: &FleetSubnetRootAuthority,
    root: candid::Principal,
    entries: &[FleetSubnetRootEntry],
) -> Result<FleetSubnetRootEntry, InternalError> {
    let root_entry = entries
        .iter()
        .find(|entry| entry.fleet_subnet_root == root)
        .cloned()
        .ok_or_else(InternalError::invariant)?;
    let expected = FleetSubnetRootEntry {
        placement_subnet: authority.binding.placement_subnet,
        fleet_subnet_root: root,
        component_admissions: authority.binding.component_admissions.clone(),
        component_topology_digest: authority.binding.component_topology_digest,
        active_release_set: authority.initial_release_set,
        limits: authority.binding.limits.clone(),
        status: root_entry.status,
    };
    let status_is_current = matches!(
        root_entry.status,
        FleetSubnetRootStatus::Active | FleetSubnetRootStatus::Draining
    );
    if root_entry != expected || !status_is_current {
        return Err(InternalError::invariant());
    }
    Ok(root_entry)
}

fn version_precedes(previous: &FleetRegistryVersion, current: &FleetRegistryVersion) -> bool {
    let same_authority = previous.authority == current.authority;
    let earlier_revision = previous.revision < current.revision;
    let hashes_are_present = previous.content_hash != [0; 32] && current.content_hash != [0; 32];
    [same_authority, earlier_revision, hashes_are_present]
        .into_iter()
        .all(|valid| valid)
}

fn candidate_record_to_view(
    record: RootFleetRegistryCandidateRecord,
) -> RootFleetRegistryCandidateView {
    RootFleetRegistryCandidateView {
        snapshot: record.snapshot,
        acknowledgement: record.acknowledgement,
    }
}

fn active_record_to_view(record: RootFleetRegistryActiveRecord) -> RootFleetRegistryActiveView {
    RootFleetRegistryActiveView {
        previous_registry: record.previous_registry,
        snapshot: record.snapshot,
        directory: record.directory,
    }
}
