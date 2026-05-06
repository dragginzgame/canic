use crate::version_text;
use canic_backup::manifest::{
    FleetBackupManifest, ManifestValidationError, manifest_validation_summary,
};
use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    path::PathBuf,
};
use thiserror::Error as ThisError;

///
/// ManifestCommandError
///

#[derive(Debug, ThisError)]
pub enum ManifestCommandError {
    #[error("{0}")]
    Usage(&'static str),

    #[error("missing required option {0}")]
    MissingOption(&'static str),

    #[error("unknown option {0}")]
    UnknownOption(String),

    #[error("option {0} requires a value")]
    MissingValue(&'static str),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    InvalidManifest(#[from] ManifestValidationError),

    #[error("manifest {backup_id} is not design ready")]
    DesignConformanceNotReady { backup_id: String },
}

///
/// ManifestValidateOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestValidateOptions {
    pub manifest: PathBuf,
    pub out: Option<PathBuf>,
    pub require_design_v1: bool,
}

impl ManifestValidateOptions {
    /// Parse manifest validation options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, ManifestCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut manifest = None;
        let mut out = None;
        let mut require_design_v1 = false;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| ManifestCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--manifest" => {
                    manifest = Some(PathBuf::from(next_value(&mut args, "--manifest")?));
                }
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--require-design" => require_design_v1 = true,
                "--help" | "-h" => return Err(ManifestCommandError::Usage(usage())),
                _ => return Err(ManifestCommandError::UnknownOption(arg)),
            }
        }

        Ok(Self {
            manifest: manifest.ok_or(ManifestCommandError::MissingOption("--manifest"))?,
            out,
            require_design_v1,
        })
    }
}

/// Run a manifest subcommand.
pub fn run<I>(args: I) -> Result<(), ManifestCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let Some(command) = args.next().and_then(|arg| arg.into_string().ok()) else {
        return Err(ManifestCommandError::Usage(usage()));
    };

    match command.as_str() {
        "validate" => {
            let options = ManifestValidateOptions::parse(args)?;
            let manifest = validate_manifest(&options)?;
            write_validation_summary(&options, &manifest)?;
            require_design_conformance(&options, &manifest)?;
            Ok(())
        }
        "help" | "--help" | "-h" => {
            println!("{}", usage());
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("{}", version_text());
            Ok(())
        }
        _ => Err(ManifestCommandError::UnknownOption(command)),
    }
}

/// Read and validate a fleet backup manifest from disk.
pub fn validate_manifest(
    options: &ManifestValidateOptions,
) -> Result<FleetBackupManifest, ManifestCommandError> {
    let data = fs::read_to_string(&options.manifest)?;
    let manifest: FleetBackupManifest = serde_json::from_str(&data)?;
    manifest.validate()?;
    Ok(manifest)
}

// Write a concise validation summary for shell use.
fn write_validation_summary(
    options: &ManifestValidateOptions,
    manifest: &FleetBackupManifest,
) -> Result<(), ManifestCommandError> {
    let summary = manifest_validation_summary(manifest);

    if let Some(path) = &options.out {
        let data = serde_json::to_vec_pretty(&summary)?;
        fs::write(path, data)?;
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, &summary)?;
    writeln!(handle)?;
    Ok(())
}

// Fail closed when callers require the v1 backup/restore design contract.
fn require_design_conformance(
    options: &ManifestValidateOptions,
    manifest: &FleetBackupManifest,
) -> Result<(), ManifestCommandError> {
    if !options.require_design_v1 {
        return Ok(());
    }

    let report = manifest.design_conformance_report();
    if report.design_v1_ready {
        Ok(())
    } else {
        Err(ManifestCommandError::DesignConformanceNotReady {
            backup_id: manifest.backup_id.clone(),
        })
    }
}

// Read the next required option value.
fn next_value<I>(args: &mut I, option: &'static str) -> Result<String, ManifestCommandError>
where
    I: Iterator<Item = OsString>,
{
    args.next()
        .and_then(|value| value.into_string().ok())
        .ok_or(ManifestCommandError::MissingValue(option))
}

// Return manifest command usage text.
const fn usage() -> &'static str {
    "usage: canic manifest validate --manifest <file> [--out <file>] [--require-design]"
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_backup::manifest::{
        BackupUnit, BackupUnitKind, ConsistencyMode, ConsistencySection, FleetMember, FleetSection,
        IdentityMode, SourceMetadata, SourceSnapshot, ToolMetadata, VerificationCheck,
        VerificationPlan,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    const ROOT: &str = "aaaaa-aa";
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    // Ensure manifest validation options parse the intended command shape.
    #[test]
    fn parses_manifest_validate_options() {
        let options = ManifestValidateOptions::parse([
            OsString::from("--manifest"),
            OsString::from("manifest.json"),
            OsString::from("--out"),
            OsString::from("summary.json"),
            OsString::from("--require-design"),
        ])
        .expect("parse options");

        assert_eq!(options.manifest, PathBuf::from("manifest.json"));
        assert_eq!(options.out, Some(PathBuf::from("summary.json")));
        assert!(options.require_design_v1);
    }

    // Ensure manifest validation loads JSON and runs the manifest contract.
    #[test]
    fn validate_manifest_reads_and_validates_manifest() {
        let root = temp_dir("canic-cli-manifest-validate");
        fs::create_dir_all(&root).expect("create temp root");
        let manifest_path = root.join("manifest.json");

        fs::write(
            &manifest_path,
            serde_json::to_vec(&valid_manifest()).expect("serialize manifest"),
        )
        .expect("write manifest");

        let options = ManifestValidateOptions {
            manifest: manifest_path,
            out: None,
            require_design_v1: false,
        };

        let manifest = validate_manifest(&options).expect("validate manifest");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(manifest.backup_id, "backup-test");
        assert_eq!(manifest.fleet.members.len(), 1);
    }

    // Ensure manifest validation summaries can be written for automation.
    #[test]
    fn write_validation_summary_writes_out_file() {
        let root = temp_dir("canic-cli-manifest-summary");
        fs::create_dir_all(&root).expect("create temp root");
        let out = root.join("summary.json");
        let options = ManifestValidateOptions {
            manifest: root.join("manifest.json"),
            out: Some(out.clone()),
            require_design_v1: false,
        };

        write_validation_summary(&options, &valid_manifest()).expect("write summary");
        let summary: serde_json::Value =
            serde_json::from_slice(&fs::read(&out).expect("read summary")).expect("parse summary");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(summary["status"], "valid");
        assert_eq!(summary["backup_id"], "backup-test");
        assert_eq!(summary["members"], 1);
        assert_eq!(summary["backup_unit_count"], 1);
        assert_eq!(summary["consistency_mode"], "crash-consistent");
        assert_eq!(summary["topology_validation_status"], "validated");
        assert_eq!(summary["backup_unit_kinds"]["subtree_rooted"], 1);
        assert_eq!(summary["backup_units"][0]["unit_id"], "fleet");
        assert_eq!(summary["backup_units"][0]["kind"], "subtree-rooted");
        assert_eq!(summary["backup_units"][0]["role_count"], 1);
        assert_eq!(summary["design_conformance"]["design_v1_ready"], true);
        assert_eq!(
            summary["design_conformance"]["topology"]["canonical_input"],
            true
        );
        assert_eq!(
            summary["design_conformance"]["snapshot_provenance"]["all_members_have_checksum"],
            true
        );
    }

    // Ensure manifest validation can fail closed after writing conformance output.
    #[test]
    fn require_design_v1_fails_after_writing_summary() {
        let root = temp_dir("canic-cli-manifest-design");
        fs::create_dir_all(&root).expect("create temp root");
        let out = root.join("summary.json");
        let mut manifest = valid_manifest();
        manifest.fleet.topology_hash_input = "legacy-input".to_string();
        let options = ManifestValidateOptions {
            manifest: root.join("manifest.json"),
            out: Some(out.clone()),
            require_design_v1: true,
        };

        write_validation_summary(&options, &manifest).expect("write summary");
        let err =
            require_design_conformance(&options, &manifest).expect_err("design gate should fail");
        let summary: serde_json::Value =
            serde_json::from_slice(&fs::read(&out).expect("read summary")).expect("parse summary");

        fs::remove_dir_all(root).expect("remove temp root");
        assert!(matches!(
            err,
            ManifestCommandError::DesignConformanceNotReady { .. }
        ));
        assert_eq!(summary["design_conformance"]["design_v1_ready"], false);
        assert_eq!(
            summary["design_conformance"]["topology"]["canonical_input"],
            false
        );
    }

    // Build one valid manifest for validation tests.
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
                    roles: vec!["root".to_string()],
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
                members: vec![fleet_member()],
            },
            verification: VerificationPlan::default(),
        }
    }

    // Build one valid manifest member.
    fn fleet_member() -> FleetMember {
        FleetMember {
            role: "root".to_string(),
            canister_id: ROOT.to_string(),
            parent_canister_id: None,
            subnet_canister_id: Some(ROOT.to_string()),
            controller_hint: None,
            identity_mode: IdentityMode::Fixed,
            restore_group: 1,
            verification_class: "basic".to_string(),
            verification_checks: vec![VerificationCheck {
                kind: "status".to_string(),
                method: None,
                roles: vec!["root".to_string()],
            }],
            source_snapshot: SourceSnapshot {
                snapshot_id: "root-snapshot".to_string(),
                module_hash: None,
                wasm_hash: None,
                code_version: Some("v0.30.1".to_string()),
                artifact_path: "artifacts/root".to_string(),
                checksum_algorithm: "sha256".to_string(),
                checksum: Some(HASH.to_string()),
            },
        }
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
