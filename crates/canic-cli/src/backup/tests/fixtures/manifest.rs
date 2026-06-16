//! Module: backup::tests::fixtures::manifest
//!
//! Responsibility: build backup manifest test fixtures.
//! Does not own: backup layout persistence or execution journal fixtures.
//! Boundary: deterministic manifest data for CLI backup tests.

use super::{HASH, ROOT};
use canic_backup::manifest::{
    BackupUnit, BackupUnitKind, ConsistencySection, DeploymentBackupManifest, DeploymentMember,
    DeploymentSection, IdentityMode, SourceMetadata, SourceSnapshot, ToolMetadata,
    VerificationCheck, VerificationPlan,
};

// Build one valid manifest for CLI verification tests.
pub(in crate::backup::tests) fn valid_manifest() -> DeploymentBackupManifest {
    valid_manifest_with("backup-test", "2026-05-03T00:00:00Z")
}

// Build one valid manifest with caller-provided summary fields.
pub(in crate::backup::tests) fn valid_manifest_with(
    backup_id: &str,
    created_at: &str,
) -> DeploymentBackupManifest {
    DeploymentBackupManifest {
        manifest_version: 1,
        backup_id: backup_id.to_string(),
        created_at: created_at.to_string(),
        tool: ToolMetadata {
            name: "canic".to_string(),
            version: "0.30.3".to_string(),
        },
        source: SourceMetadata {
            environment: "local".to_string(),
            root_canister: ROOT.to_string(),
        },
        consistency: ConsistencySection {
            backup_units: vec![BackupUnit {
                unit_id: "deployment".to_string(),
                kind: BackupUnitKind::Single,
                roles: vec!["root".to_string()],
            }],
        },
        deployment: DeploymentSection {
            topology_hash_algorithm: "sha256".to_string(),
            topology_hash_input: "sorted(pid,parent_pid,role,module_hash)".to_string(),
            discovery_topology_hash: HASH.to_string(),
            pre_snapshot_topology_hash: HASH.to_string(),
            topology_hash: HASH.to_string(),
            members: vec![deployment_member()],
        },
        verification: VerificationPlan::default(),
    }
}

// Build one valid manifest member.
fn deployment_member() -> DeploymentMember {
    DeploymentMember {
        role: "root".to_string(),
        canister_id: ROOT.to_string(),
        parent_canister_id: None,
        subnet_canister_id: Some(ROOT.to_string()),
        controller_hint: None,
        identity_mode: IdentityMode::Fixed,
        verification_checks: vec![VerificationCheck {
            kind: "status".to_string(),
            roles: vec!["root".to_string()],
        }],
        source_snapshot: SourceSnapshot {
            snapshot_id: "root-snapshot".to_string(),
            module_hash: None,
            code_version: Some("v0.30.3".to_string()),
            artifact_path: "artifacts/root".to_string(),
            checksum_algorithm: "sha256".to_string(),
            checksum: None,
        },
    }
}
