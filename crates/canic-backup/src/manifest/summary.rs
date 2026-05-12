use super::{BackupUnitKind, FleetBackupManifest};
use serde_json::json;

/// Build the manifest validation summary emitted by the CLI.
#[must_use]
pub fn manifest_validation_summary(manifest: &FleetBackupManifest) -> serde_json::Value {
    json!({
        "status": "valid",
        "backup_id": manifest.backup_id,
        "members": manifest.fleet.members.len(),
        "backup_unit_count": manifest.consistency.backup_units.len(),
        "topology_hash": manifest.fleet.topology_hash,
        "topology_hash_algorithm": manifest.fleet.topology_hash_algorithm,
        "topology_hash_input": manifest.fleet.topology_hash_input,
        "topology_validation_status": "validated",
        "backup_unit_kinds": backup_unit_kind_counts(manifest),
        "backup_units": manifest
            .consistency
            .backup_units
            .iter()
            .map(|unit| json!({
                "unit_id": unit.unit_id,
                "kind": backup_unit_kind_name(&unit.kind),
                "role_count": unit.roles.len(),
            }))
            .collect::<Vec<_>>(),
    })
}

fn backup_unit_kind_counts(manifest: &FleetBackupManifest) -> serde_json::Value {
    let mut single = 0;
    let mut subtree = 0;
    for unit in &manifest.consistency.backup_units {
        match &unit.kind {
            BackupUnitKind::Single => single += 1,
            BackupUnitKind::Subtree => subtree += 1,
        }
    }

    json!({
        "single": single,
        "subtree": subtree,
    })
}

const fn backup_unit_kind_name(kind: &BackupUnitKind) -> &'static str {
    match kind {
        BackupUnitKind::Single => "single",
        BackupUnitKind::Subtree => "subtree",
    }
}
