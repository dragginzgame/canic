use super::{
    SnapshotDownloadConfig, SnapshotDownloadError, SnapshotDriver, SnapshotManifestError,
    resolve_snapshot_targets, support::target_role,
};
use crate::{
    discovery::SnapshotTarget,
    topology::{TopologyHash, TopologyHasher, TopologyRecord},
};
use candid::Principal;

/// Compute the canonical topology hash for one resolved target set.
pub fn topology_hash_for_targets(
    selected_canister: &str,
    targets: &[SnapshotTarget],
) -> Result<TopologyHash, SnapshotManifestError> {
    let topology_records = targets
        .iter()
        .enumerate()
        .map(|(index, target)| topology_record(selected_canister, index, target))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TopologyHasher::hash(&topology_records))
}

/// Fail closed if topology changes after discovery but before snapshot creation.
pub fn ensure_topology_stable(
    discovery: &TopologyHash,
    pre_snapshot: &TopologyHash,
) -> Result<(), SnapshotManifestError> {
    if discovery.hash == pre_snapshot.hash {
        return Ok(());
    }

    Err(SnapshotManifestError::TopologyChanged {
        discovery: discovery.hash.clone(),
        pre_snapshot: pre_snapshot.hash.clone(),
    })
}

pub(super) fn accepted_pre_snapshot_topology_hash(
    config: &SnapshotDownloadConfig,
    driver: &mut impl SnapshotDriver,
    discovery_topology_hash: &TopologyHash,
) -> Result<TopologyHash, SnapshotDownloadError> {
    if config.dry_run {
        return Ok(discovery_topology_hash.clone());
    }

    let pre_snapshot_targets = resolve_snapshot_targets(config, driver)?;
    let pre_snapshot_topology_hash =
        topology_hash_for_targets(&config.canister, &pre_snapshot_targets)?;
    ensure_topology_stable(discovery_topology_hash, &pre_snapshot_topology_hash)?;
    Ok(pre_snapshot_topology_hash)
}

fn topology_record(
    selected_canister: &str,
    index: usize,
    target: &SnapshotTarget,
) -> Result<TopologyRecord, SnapshotManifestError> {
    Ok(TopologyRecord {
        pid: parse_principal("fleet.members[].canister_id", &target.canister_id)?,
        parent_pid: target
            .parent_canister_id
            .as_deref()
            .map(|parent| parse_principal("fleet.members[].parent_canister_id", parent))
            .transpose()?,
        role: target_role(selected_canister, index, target),
        module_hash: target.module_hash.clone(),
    })
}

fn parse_principal(field: &'static str, value: &str) -> Result<Principal, SnapshotManifestError> {
    Principal::from_text(value).map_err(|_| SnapshotManifestError::InvalidPrincipal {
        field,
        value: value.to_string(),
    })
}
