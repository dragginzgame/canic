use super::*;
use crate::test_support::{TempDir, temp_dir};
use canic_backup::manifest::{
    BackupUnit, BackupUnitKind, ConsistencySection, DeploymentMember, DeploymentSection,
    IdentityMode, SourceMetadata, SourceSnapshot, ToolMetadata, VerificationCheck,
    VerificationPlan,
};
use std::fs;

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

// Ensure a missing manifest path points operators to the concrete flag shape.
#[test]
fn missing_manifest_validate_option_names_required_path() {
    let err = ManifestValidateOptions::parse([]).expect_err("missing manifest option");

    std::assert_matches!(err, ManifestCommandError::Usage(_));
    assert!(err.to_string().contains("--manifest <file>"));
    assert!(err.to_string().contains("canic backup manifest validate"));
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
    assert_eq!(manifest.deployment.members.len(), 1);
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
    assert_eq!(summary["backup_units"][0]["unit_id"], "deployment");
    assert_eq!(summary["backup_units"][0]["kind"], "subtree");
    assert_eq!(summary["backup_units"][0]["role_count"], 1);
}

// Build one valid manifest for validation tests.
fn valid_manifest() -> DeploymentBackupManifest {
    DeploymentBackupManifest {
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
                unit_id: "deployment".to_string(),
                kind: BackupUnitKind::Subtree,
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
            code_version: Some("v0.30.1".to_string()),
            artifact_path: "artifacts/root".to_string(),
            checksum_algorithm: "sha256".to_string(),
            checksum: Some(HASH.to_string()),
        },
    }
}

#[test]
fn manifest_validation_admits_exact_record_limit_and_refuses_larger_files() {
    use canic_backup::persistence::JSON_RECORD_MAX_BYTES;
    let root = TempDir::new("canic-cli-manifest-read-limit");
    fs::create_dir_all(&root).unwrap();
    let options = ManifestValidateOptions {
        manifest: root.join("manifest.json"),
        out: None,
    };
    let mut bytes = serde_json::to_vec(&valid_manifest()).unwrap();
    bytes.resize(JSON_RECORD_MAX_BYTES, b' ');
    fs::write(&options.manifest, &bytes).unwrap();
    drop(bytes);
    assert_eq!(
        validate_manifest(&options).unwrap().backup_id,
        "backup-test"
    );
    fs::OpenOptions::new()
        .write(true)
        .open(&options.manifest)
        .unwrap()
        .set_len(JSON_RECORD_MAX_BYTES as u64 + 1)
        .unwrap();
    std::assert_matches!(validate_manifest(&options),
        Err(ManifestCommandError::Io(error)) if error.kind() == std::io::ErrorKind::Other);
}

#[test]
fn manifest_validation_preserves_read_decode_and_schema_errors() {
    let root = TempDir::new("canic-cli-manifest-read-errors");
    fs::create_dir_all(&root).unwrap();
    let options = ManifestValidateOptions {
        manifest: root.join("manifest.json"),
        out: None,
    };
    std::assert_matches!(validate_manifest(&options),
        Err(ManifestCommandError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound);
    fs::write(&options.manifest, b"{").unwrap();
    std::assert_matches!(validate_manifest(&options),
        Err(ManifestCommandError::Json(error)) if error.is_eof());
    let mut manifest = valid_manifest();
    manifest.manifest_version = 0;
    fs::write(&options.manifest, serde_json::to_vec(&manifest).unwrap()).unwrap();
    std::assert_matches!(
        validate_manifest(&options),
        Err(ManifestCommandError::InvalidManifest(
            ManifestValidationError::UnsupportedManifestVersion(0)
        ))
    );
}

#[cfg(unix)]
#[test]
fn manifest_validation_rejects_nonregular_inputs_and_preserves_explicit_aliases() {
    let root = TempDir::new("canic-cli-manifest-descriptor");
    fs::create_dir_all(&root).unwrap();
    let mut options = ManifestValidateOptions {
        manifest: root.to_path_buf(),
        out: None,
    };
    std::assert_matches!(
        validate_manifest(&options),
        Err(ManifestCommandError::Io(_))
    );
    options.manifest = root.join("fifo");
    assert!(
        std::process::Command::new("/usr/bin/mkfifo")
            .env_clear()
            .args(["-m", "600"])
            .arg(&options.manifest)
            .status()
            .unwrap()
            .success()
    );
    std::assert_matches!(
        validate_manifest(&options),
        Err(ManifestCommandError::Io(_))
    );
    let manifest = root.join("manifest.json");
    fs::write(&manifest, serde_json::to_vec(&valid_manifest()).unwrap()).unwrap();
    options.manifest = root.join("alias.json");
    std::os::unix::fs::symlink(&manifest, &options.manifest).unwrap();
    assert_eq!(
        validate_manifest(&options).unwrap().backup_id,
        "backup-test"
    );
    fs::remove_file(&manifest).unwrap();
    std::assert_matches!(validate_manifest(&options),
        Err(ManifestCommandError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound);
}
