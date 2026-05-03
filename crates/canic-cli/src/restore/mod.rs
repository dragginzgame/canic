use canic_backup::{
    manifest::FleetBackupManifest,
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

    #[error("unknown option {0}")]
    UnknownOption(String),

    #[error("option {0} requires a value")]
    MissingValue(&'static str),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    RestorePlan(#[from] RestorePlanError),
}

///
/// RestorePlanOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestorePlanOptions {
    pub manifest: PathBuf,
    pub mapping: Option<PathBuf>,
    pub out: Option<PathBuf>,
}

impl RestorePlanOptions {
    /// Parse restore planning options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, RestoreCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut manifest = None;
        let mut mapping = None;
        let mut out = None;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| RestoreCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--manifest" => {
                    manifest = Some(PathBuf::from(next_value(&mut args, "--manifest")?));
                }
                "--mapping" => mapping = Some(PathBuf::from(next_value(&mut args, "--mapping")?)),
                "--out" => out = Some(PathBuf::from(next_value(&mut args, "--out")?)),
                "--help" | "-h" => return Err(RestoreCommandError::Usage(usage())),
                _ => return Err(RestoreCommandError::UnknownOption(arg)),
            }
        }

        Ok(Self {
            manifest: manifest.ok_or(RestoreCommandError::MissingOption("--manifest"))?,
            mapping,
            out,
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
    let manifest = read_manifest(&options.manifest)?;
    let mapping = options.mapping.as_ref().map(read_mapping).transpose()?;

    RestorePlanner::plan(&manifest, mapping.as_ref()).map_err(RestoreCommandError::from)
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
    "usage: canic restore plan --manifest <file> [--mapping <file>] [--out <file>]"
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_backup::manifest::{
        BackupUnit, BackupUnitKind, ConsistencyMode, ConsistencySection, FleetMember, FleetSection,
        IdentityMode, SourceMetadata, SourceSnapshot, ToolMetadata, VerificationCheck,
        VerificationPlan,
    };
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

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

        assert_eq!(options.manifest, PathBuf::from("manifest.json"));
        assert_eq!(options.mapping, Some(PathBuf::from("mapping.json")));
        assert_eq!(options.out, Some(PathBuf::from("plan.json")));
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
            manifest: manifest_path,
            mapping: Some(mapping_path),
            out: None,
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
