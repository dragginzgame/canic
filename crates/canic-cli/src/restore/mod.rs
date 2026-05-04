use canic_backup::{
    manifest::FleetBackupManifest,
    persistence::{BackupLayout, PersistenceError},
    restore::{RestoreMapping, RestorePlan, RestorePlanError, RestorePlanner},
};
use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    path::PathBuf,
};
use thiserror::Error as ThisError;

///
/// RestoreCommandError
///

#[derive(Debug, ThisError)]
pub enum RestoreCommandError {
    #[error("{0}")]
    Usage(&'static str),

    #[error("missing required option {0}")]
    MissingOption(&'static str),

    #[error("use either --manifest or --backup-dir, not both")]
    ConflictingManifestSources,

    #[error("--require-verified requires --backup-dir")]
    RequireVerifiedNeedsBackupDir,

    #[error("unknown option {0}")]
    UnknownOption(String),

    #[error("option {0} requires a value")]
    MissingValue(&'static str),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Persistence(#[from] PersistenceError),

    #[error(transparent)]
    RestorePlan(#[from] RestorePlanError),
}

///
/// RestorePlanOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestorePlanOptions {
    pub manifest: Option<PathBuf>,
    pub backup_dir: Option<PathBuf>,
    pub mapping: Option<PathBuf>,
    pub out: Option<PathBuf>,
    pub require_verified: bool,
}

impl RestorePlanOptions {
    /// Parse restore planning options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut manifest = None;
        let mut backup_dir = None;
        let mut mapping = None;
        let mut out = None;
        let mut require_verified = false;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| RestoreCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--manifest" => {
                    manifest = Some(PathBuf::from(next_value(&mut args, "--manifest")?));
                }
                "--backup-dir" => {
                    backup_dir = Some(PathBuf::from(next_value(&mut args, "--backup-dir")?));
                }
                "--mapping" => mapping = Some(PathBuf::from(next_value(&mut args, "--mapping")?)),
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--require-verified" => require_verified = true,
                "--help" | "-h" => return Err(RestoreCommandError::Usage(usage())),
                _ => return Err(RestoreCommandError::UnknownOption(arg)),
            }
        }

        if manifest.is_some() && backup_dir.is_some() {
            return Err(RestoreCommandError::ConflictingManifestSources);
        }

        if manifest.is_none() && backup_dir.is_none() {
            return Err(RestoreCommandError::MissingOption(
                "--manifest or --backup-dir",
            ));
        }

        if require_verified && backup_dir.is_none() {
            return Err(RestoreCommandError::RequireVerifiedNeedsBackupDir);
        }

        Ok(Self {
            manifest,
            backup_dir,
            mapping,
            out,
            require_verified,
        })
    }
}

/// Run a restore subcommand.
pub fn run<I>(args: I) -> Result<(), RestoreCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let Some(command) = args.next().and_then(|arg| arg.into_string().ok()) else {
        return Err(RestoreCommandError::Usage(usage()));
    };

    match command.as_str() {
        "plan" => {
            let options = RestorePlanOptions::parse(args)?;
            let plan = plan_restore(&options)?;
            write_plan(&options, &plan)?;
            Ok(())
        }
        "help" | "--help" | "-h" => Err(RestoreCommandError::Usage(usage())),
        _ => Err(RestoreCommandError::UnknownOption(command)),
    }
}

/// Build a no-mutation restore plan from a manifest and optional mapping.
pub fn plan_restore(options: &RestorePlanOptions) -> Result<RestorePlan, RestoreCommandError> {
    verify_backup_layout_if_required(options)?;

    let manifest = read_manifest_source(options)?;
    let mapping = options.mapping.as_ref().map(read_mapping).transpose()?;

    RestorePlanner::plan(&manifest, mapping.as_ref()).map_err(RestoreCommandError::from)
}

// Verify backup layout integrity before restore planning when requested.
fn verify_backup_layout_if_required(
    options: &RestorePlanOptions,
) -> Result<(), RestoreCommandError> {
    if !options.require_verified {
        return Ok(());
    }

    let Some(dir) = &options.backup_dir else {
        return Err(RestoreCommandError::RequireVerifiedNeedsBackupDir);
    };

    BackupLayout::new(dir.clone()).verify_integrity()?;
    Ok(())
}

// Read the manifest from a direct path or canonical backup layout.
fn read_manifest_source(
    options: &RestorePlanOptions,
) -> Result<FleetBackupManifest, RestoreCommandError> {
    if let Some(path) = &options.manifest {
        return read_manifest(path);
    }

    let Some(dir) = &options.backup_dir else {
        return Err(RestoreCommandError::MissingOption(
            "--manifest or --backup-dir",
        ));
    };

    BackupLayout::new(dir.clone())
        .read_manifest()
        .map_err(RestoreCommandError::from)
}

// Read and decode a fleet backup manifest from disk.
fn read_manifest(path: &PathBuf) -> Result<FleetBackupManifest, RestoreCommandError> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(RestoreCommandError::from)
}

// Read and decode an optional source-to-target restore mapping from disk.
fn read_mapping(path: &PathBuf) -> Result<RestoreMapping, RestoreCommandError> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(RestoreCommandError::from)
}

// Write the computed plan to stdout or a requested output file.
fn write_plan(options: &RestorePlanOptions, plan: &RestorePlan) -> Result<(), RestoreCommandError> {
    if let Some(path) = &options.out {
        let data = serde_json::to_vec_pretty(plan)?;
        fs::write(path, data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, plan)?;
    writeln!(handle)?;
    Ok(())
}

// Read the next required option value.
fn next_value<I>(args: &mut I, option: &'static str) -> Result<String, RestoreCommandError>
where
    I: Iterator<Item = OsString>,
{
    args.next()
        .and_then(|value| value.into_string().ok())
        .ok_or(RestoreCommandError::MissingValue(option))
}

// Return restore command usage text.
const fn usage() -> &'static str {
    "usage: canic restore plan (--manifest <file> | --backup-dir <dir>) [--mapping <file>] [--out <file>] [--require-verified]"
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_backup::{
        artifacts::ArtifactChecksum,
        journal::{ArtifactJournalEntry, ArtifactState, DownloadJournal},
        manifest::{
            BackupUnit, BackupUnitKind, ConsistencyMode, ConsistencySection, FleetMember,
            FleetSection, IdentityMode, SourceMetadata, SourceSnapshot, ToolMetadata,
            VerificationCheck, VerificationPlan,
        },
    };
    use serde_json::json;
    use std::{
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };

    const ROOT: &str = "aaaaa-aa";
    const CHILD: &str = "renrk-eyaaa-aaaaa-aaada-cai";
    const MAPPED_CHILD: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    // Ensure restore plan options parse the intended no-mutation command.
    #[test]
    fn parses_restore_plan_options() {
        let options = RestorePlanOptions::parse([
            OsString::from("--manifest"),
            OsString::from("manifest.json"),
            OsString::from("--mapping"),
            OsString::from("mapping.json"),
            OsString::from("--out"),
            OsString::from("plan.json"),
        ])
        .expect("parse options");

        assert_eq!(options.manifest, Some(PathBuf::from("manifest.json")));
        assert_eq!(options.backup_dir, None);
        assert_eq!(options.mapping, Some(PathBuf::from("mapping.json")));
        assert_eq!(options.out, Some(PathBuf::from("plan.json")));
        assert!(!options.require_verified);
    }

    // Ensure verified restore plan options parse with the canonical backup source.
    #[test]
    fn parses_verified_restore_plan_options() {
        let options = RestorePlanOptions::parse([
            OsString::from("--backup-dir"),
            OsString::from("backups/run"),
            OsString::from("--require-verified"),
        ])
        .expect("parse verified options");

        assert_eq!(options.manifest, None);
        assert_eq!(options.backup_dir, Some(PathBuf::from("backups/run")));
        assert_eq!(options.mapping, None);
        assert_eq!(options.out, None);
        assert!(options.require_verified);
    }

    // Ensure backup-dir restore planning reads the canonical layout manifest.
    #[test]
    fn plan_restore_reads_manifest_from_backup_dir() {
        let root = temp_dir("canic-cli-restore-plan-layout");
        let layout = BackupLayout::new(root.clone());
        layout
            .write_manifest(&valid_manifest())
            .expect("write manifest");

        let options = RestorePlanOptions {
            manifest: None,
            backup_dir: Some(root.clone()),
            mapping: None,
            out: None,
            require_verified: false,
        };

        let plan = plan_restore(&options).expect("plan restore");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(plan.backup_id, "backup-test");
        assert_eq!(plan.member_count, 2);
    }

    // Ensure restore planning has exactly one manifest source.
    #[test]
    fn parse_rejects_conflicting_manifest_sources() {
        let err = RestorePlanOptions::parse([
            OsString::from("--manifest"),
            OsString::from("manifest.json"),
            OsString::from("--backup-dir"),
            OsString::from("backups/run"),
        ])
        .expect_err("conflicting sources should fail");

        assert!(matches!(
            err,
            RestoreCommandError::ConflictingManifestSources
        ));
    }

    // Ensure verified planning requires the canonical backup layout source.
    #[test]
    fn parse_rejects_require_verified_with_manifest_source() {
        let err = RestorePlanOptions::parse([
            OsString::from("--manifest"),
            OsString::from("manifest.json"),
            OsString::from("--require-verified"),
        ])
        .expect_err("verification should require a backup layout");

        assert!(matches!(
            err,
            RestoreCommandError::RequireVerifiedNeedsBackupDir
        ));
    }

    // Ensure restore planning can require manifest, journal, and artifact integrity.
    #[test]
    fn plan_restore_requires_verified_backup_layout() {
        let root = temp_dir("canic-cli-restore-plan-verified");
        let layout = BackupLayout::new(root.clone());
        let manifest = valid_manifest();
        write_verified_layout(&root, &layout, &manifest);

        let options = RestorePlanOptions {
            manifest: None,
            backup_dir: Some(root.clone()),
            mapping: None,
            out: None,
            require_verified: true,
        };

        let plan = plan_restore(&options).expect("plan verified restore");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(plan.backup_id, "backup-test");
        assert_eq!(plan.member_count, 2);
    }

    // Ensure required verification fails before planning when the layout is incomplete.
    #[test]
    fn plan_restore_rejects_unverified_backup_layout() {
        let root = temp_dir("canic-cli-restore-plan-unverified");
        let layout = BackupLayout::new(root.clone());
        layout
            .write_manifest(&valid_manifest())
            .expect("write manifest");

        let options = RestorePlanOptions {
            manifest: None,
            backup_dir: Some(root.clone()),
            mapping: None,
            out: None,
            require_verified: true,
        };

        let err = plan_restore(&options).expect_err("missing journal should fail");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(matches!(err, RestoreCommandError::Persistence(_)));
    }

    // Ensure the CLI planning path validates manifests and applies mappings.
    #[test]
    fn plan_restore_reads_manifest_and_mapping() {
        let root = temp_dir("canic-cli-restore-plan");
        fs::create_dir_all(&root).expect("create temp root");
        let manifest_path = root.join("manifest.json");
        let mapping_path = root.join("mapping.json");

        fs::write(
            &manifest_path,
            serde_json::to_vec(&valid_manifest()).expect("serialize manifest"),
        )
        .expect("write manifest");
        fs::write(
            &mapping_path,
            json!({
                "members": [
                    {"source_canister": ROOT, "target_canister": ROOT},
                    {"source_canister": CHILD, "target_canister": MAPPED_CHILD}
                ]
            })
            .to_string(),
        )
        .expect("write mapping");

        let options = RestorePlanOptions {
            manifest: Some(manifest_path),
            backup_dir: None,
            mapping: Some(mapping_path),
            out: None,
            require_verified: false,
        };

        let plan = plan_restore(&options).expect("plan restore");

        fs::remove_dir_all(root).expect("remove temp root");
        let members = plan.ordered_members();
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].source_canister, ROOT);
        assert_eq!(members[1].target_canister, MAPPED_CHILD);
    }

    // Build one valid manifest for restore planning tests.
    fn valid_manifest() -> FleetBackupManifest {
        FleetBackupManifest {
            manifest_version: 1,
            backup_id: "backup-test".to_string(),
            created_at: "2026-05-03T00:00:00Z".to_string(),
            tool: ToolMetadata {
                name: "canic".to_string(),
                version: "0.30.1".to_string(),
            },
            source: SourceMetadata {
                environment: "local".to_string(),
                root_canister: ROOT.to_string(),
            },
            consistency: ConsistencySection {
                mode: ConsistencyMode::CrashConsistent,
                backup_units: vec![BackupUnit {
                    unit_id: "fleet".to_string(),
                    kind: BackupUnitKind::SubtreeRooted,
                    roles: vec!["root".to_string(), "app".to_string()],
                    consistency_reason: None,
                    dependency_closure: Vec::new(),
                    topology_validation: "subtree-closed".to_string(),
                    quiescence_strategy: None,
                }],
            },
            fleet: FleetSection {
                topology_hash_algorithm: "sha256".to_string(),
                topology_hash_input: "sorted(pid,parent_pid,role,module_hash)".to_string(),
                discovery_topology_hash: HASH.to_string(),
                pre_snapshot_topology_hash: HASH.to_string(),
                topology_hash: HASH.to_string(),
                members: vec![
                    fleet_member("root", ROOT, None, IdentityMode::Fixed),
                    fleet_member("app", CHILD, Some(ROOT), IdentityMode::Relocatable),
                ],
            },
            verification: VerificationPlan::default(),
        }
    }

    // Build one valid manifest member.
    fn fleet_member(
        role: &str,
        canister_id: &str,
        parent_canister_id: Option<&str>,
        identity_mode: IdentityMode,
    ) -> FleetMember {
        FleetMember {
            role: role.to_string(),
            canister_id: canister_id.to_string(),
            parent_canister_id: parent_canister_id.map(str::to_string),
            subnet_canister_id: Some(ROOT.to_string()),
            controller_hint: None,
            identity_mode,
            restore_group: 1,
            verification_class: "basic".to_string(),
            verification_checks: vec![VerificationCheck {
                kind: "status".to_string(),
                method: None,
                roles: vec![role.to_string()],
            }],
            source_snapshot: SourceSnapshot {
                snapshot_id: format!("{role}-snapshot"),
                module_hash: None,
                wasm_hash: None,
                code_version: Some("v0.30.1".to_string()),
                artifact_path: format!("artifacts/{role}"),
                checksum_algorithm: "sha256".to_string(),
                checksum: None,
            },
        }
    }

    // Write a canonical backup layout whose journal checksums match the artifacts.
    fn write_verified_layout(root: &Path, layout: &BackupLayout, manifest: &FleetBackupManifest) {
        layout.write_manifest(manifest).expect("write manifest");

        let artifacts = manifest
            .fleet
            .members
            .iter()
            .map(|member| {
                let bytes = format!("{} artifact", member.role);
                let artifact_path = root.join(&member.source_snapshot.artifact_path);
                if let Some(parent) = artifact_path.parent() {
                    fs::create_dir_all(parent).expect("create artifact parent");
                }
                fs::write(&artifact_path, bytes.as_bytes()).expect("write artifact");
                let checksum = ArtifactChecksum::from_bytes(bytes.as_bytes());

                ArtifactJournalEntry {
                    canister_id: member.canister_id.clone(),
                    snapshot_id: member.source_snapshot.snapshot_id.clone(),
                    state: ArtifactState::Durable,
                    temp_path: None,
                    artifact_path: member.source_snapshot.artifact_path.clone(),
                    checksum_algorithm: checksum.algorithm,
                    checksum: Some(checksum.hash),
                    updated_at: "2026-05-03T00:00:00Z".to_string(),
                }
            })
            .collect();

        layout
            .write_journal(&DownloadJournal {
                journal_version: 1,
                backup_id: manifest.backup_id.clone(),
                discovery_topology_hash: Some(manifest.fleet.discovery_topology_hash.clone()),
                pre_snapshot_topology_hash: Some(manifest.fleet.pre_snapshot_topology_hash.clone()),
                artifacts,
            })
            .expect("write journal");
    }

    // Build a unique temporary directory.
    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }
}
