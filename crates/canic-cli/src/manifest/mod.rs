use crate::{
    args::{parse_matches, path_option, value_arg},
    output, version_text,
};
use canic_backup::manifest::{
    FleetBackupManifest, ManifestValidationError, manifest_validation_summary,
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, fs, path::PathBuf};
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

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    InvalidManifest(#[from] ManifestValidationError),
}

///
/// ManifestValidateOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestValidateOptions {
    pub manifest: PathBuf,
    pub out: Option<PathBuf>,
}

impl ManifestValidateOptions {
    /// Parse manifest validation options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, ManifestCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(manifest_validate_command(), args)
            .map_err(|_| ManifestCommandError::Usage(usage()))?;

        Ok(Self {
            manifest: path_option(&matches, "manifest")
                .ok_or(ManifestCommandError::MissingOption("--manifest"))?,
            out: path_option(&matches, "out"),
        })
    }
}

// Build the manifest validation parser.
fn manifest_validate_command() -> ClapCommand {
    ClapCommand::new("manifest-validate")
        .disable_help_flag(true)
        .arg(value_arg("manifest").long("manifest"))
        .arg(value_arg("out").long("out"))
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

    output::write_pretty_json(options.out.as_ref(), &summary)
}

// Return manifest command usage text.
const fn usage() -> &'static str {
    "usage: canic manifest validate --manifest <file> [--out <file>]"
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_backup::manifest::{
        BackupUnit, BackupUnitKind, ConsistencySection, FleetMember, FleetSection, IdentityMode,
        SourceMetadata, SourceSnapshot, ToolMetadata, VerificationCheck, VerificationPlan,
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
        ])
        .expect("parse options");

        assert_eq!(options.manifest, PathBuf::from("manifest.json"));
        assert_eq!(options.out, Some(PathBuf::from("summary.json")));
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
        };

        write_validation_summary(&options, &valid_manifest()).expect("write summary");
        let summary: serde_json::Value =
            serde_json::from_slice(&fs::read(&out).expect("read summary")).expect("parse summary");

        fs::remove_dir_all(root).expect("remove temp root");
        assert_eq!(summary["status"], "valid");
        assert_eq!(summary["backup_id"], "backup-test");
        assert_eq!(summary["members"], 1);
        assert_eq!(summary["backup_unit_count"], 1);
        assert_eq!(summary["topology_validation_status"], "validated");
        assert_eq!(summary["backup_unit_kinds"]["subtree"], 1);
        assert_eq!(summary["backup_units"][0]["unit_id"], "fleet");
        assert_eq!(summary["backup_units"][0]["kind"], "subtree");
        assert_eq!(summary["backup_units"][0]["role_count"], 1);
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
                backup_units: vec![BackupUnit {
                    unit_id: "fleet".to_string(),
                    kind: BackupUnitKind::Subtree,
                    roles: vec!["root".to_string()],
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
            verification_checks: vec![VerificationCheck {
                kind: "status".to_string(),
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
