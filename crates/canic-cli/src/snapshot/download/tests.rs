use super::*;
use crate::support::path_stamp::backup_directory_stamp_from_unix;

const ROOT: &str = "aaaaa-aa";

// Ensure option parsing covers the intended dry-run command.
#[test]
fn parses_download_options() {
    let options = SnapshotDownloadOptions::parse([
        OsString::from("demo"),
        OsString::from("--canister"),
        OsString::from(ROOT),
        OsString::from("--out"),
        OsString::from("backups/test"),
        OsString::from("--root"),
        OsString::from(ROOT),
        OsString::from("--recursive"),
        OsString::from("--dry-run"),
        OsString::from("--resume-after-snapshot"),
    ])
    .expect("parse options");

    assert_eq!(options.canister.as_deref(), Some(ROOT));
    assert_eq!(options.fleet, "demo");
    assert_eq!(options.out.as_deref(), Some(Path::new("backups/test")));
    assert!(options.include_children);
    assert!(options.recursive);
    assert!(options.dry_run);
    assert_eq!(options.root.as_deref(), Some(ROOT));
    assert_eq!(options.lifecycle, SnapshotLifecycleMode::StopAndResume);
}

// Ensure --out can be omitted for the common named-fleet backup flow.
#[test]
fn download_options_default_output_directory() {
    let options = SnapshotDownloadOptions::parse([
        OsString::from("demo"),
        OsString::from("--canister"),
        OsString::from(ROOT),
        OsString::from("--recursive"),
    ])
    .expect("parse options");
    let out = default_snapshot_output_path(&options.fleet);
    let out = out.to_string_lossy();

    assert!(out.starts_with("backups/fleet-"));
    assert!(out.chars().last().is_some_and(|last| last.is_ascii_digit()));
}

// Ensure a named fleet can be selected without spelling out its root canister.
#[test]
fn parses_download_fleet_options_without_canister() {
    let options =
        SnapshotDownloadOptions::parse([OsString::from("demo"), OsString::from("--dry-run")])
            .expect("parse options");

    assert_eq!(options.fleet, "demo");
    assert_eq!(options.canister, None);
    assert!(options.dry_run);
}

// Ensure explicit fleet/canister selections fail when the registry omits the canister.
#[test]
fn fleet_membership_rejects_unknown_canister() {
    let registry = serde_json::json!({
        "Ok": [
            {
                "pid": ROOT,
                "role": "root",
                "record": { "parent_pid": null }
            }
        ]
    })
    .to_string();
    let err = validate_fleet_membership_json("demo", "missing-cai", &registry)
        .expect_err("missing canister should reject");

    assert!(matches!(
        err,
        SnapshotCommandError::CanisterNotInFleet { fleet, canister }
            if fleet == "demo" && canister == "missing-cai"
    ));
}

// Ensure cached installed-fleet registry entries can validate membership without reparsing.
#[test]
fn fleet_membership_entries_accept_known_canister() {
    let entries = vec![HostRegistryEntry {
        pid: ROOT.to_string(),
        role: Some("root".to_string()),
        kind: None,
        parent_pid: None,
        module_hash: None,
    }];

    validate_fleet_membership_entries("demo", ROOT, &entries).expect("root is a member");
}

// Ensure generated default path labels are filesystem-friendly.
#[test]
fn snapshot_default_path_sanitizes_labels() {
    assert_eq!(file_safe_component("demo fleet/root"), "demo-fleet-root");
}

// Ensure default backup directory timestamps are compact calendar labels.
#[test]
fn backup_directory_stamp_uses_calendar_time() {
    assert_eq!(backup_directory_stamp_from_unix(0), "19700101-000000");
    assert_eq!(
        backup_directory_stamp_from_unix(1_715_090_400),
        "20240507-140000"
    );
}
