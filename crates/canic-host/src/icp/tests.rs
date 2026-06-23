use super::*;
use std::{path::Path, process::Command};

#[test]
fn parses_icp_cli_versions_from_common_output() {
    assert_eq!(
        parse_icp_cli_version("icp 1.0.0"),
        Some(IcpCliVersion {
            major: 1,
            minor: 0,
            patch: 0
        })
    );
    assert_eq!(
        parse_icp_cli_version("icp-cli v1.2.3"),
        Some(IcpCliVersion {
            major: 1,
            minor: 2,
            patch: 3
        })
    );
    assert_eq!(parse_icp_cli_version("icp development build"), None);
}

#[test]
fn icp_cli_version_range_accepts_supported_major_only() {
    assert!(!is_supported_icp_cli_version(IcpCliVersion {
        major: 0,
        minor: 0,
        patch: 0
    }));
    assert!(is_supported_icp_cli_version(IcpCliVersion {
        major: 1,
        minor: 0,
        patch: 0
    }));
    assert!(is_supported_icp_cli_version(IcpCliVersion {
        major: 1,
        minor: 3,
        patch: 9
    }));
    assert!(!is_supported_icp_cli_version(IcpCliVersion {
        major: 2,
        minor: 0,
        patch: 0
    }));
}

#[cfg(unix)]
#[test]
fn command_runner_rejects_unparseable_icp_cli_before_running_command() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let root = unique_temp_dir("canic-unsupported-icp-cli");
    fs::create_dir_all(&root).expect("create temp dir");
    let icp_path = root.join("icp");
    fs::write(
        &icp_path,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'icp development build'; exit 0; fi\necho 'unsupported command ran' >&2\nexit 42\n",
    )
    .expect("write fake icp");
    fs::set_permissions(&icp_path, fs::Permissions::from_mode(0o755)).expect("chmod fake icp");

    let mut command = Command::new(&icp_path);
    command.args(["canister", "status", "root"]);

    let err = run_status(&mut command).expect_err("unsupported icp rejected");

    assert!(matches!(
        err,
        IcpCommandError::IncompatibleCliVersion { .. }
    ));
    assert!(err.to_string().contains("found: icp development build"));
    assert!(
        err.to_string()
            .contains("required: icp-cli >=1.0.0, <2.0.0")
    );
    assert!(
        err.to_string()
            .contains("icp network update` updates the local network launcher")
    );

    fs::remove_dir_all(root).expect("remove temp dir");
}

// Keep generated commands tied to ICP CLI environments when one is selected.
#[test]
fn renders_environment_target() {
    let icp = IcpCli::new("icp", Some("staging".to_string()), Some("ic".to_string()));

    assert_eq!(
        icp.snapshot_download_display("root", "snap-1", Path::new("backups/root")),
        "icp canister snapshot download root snap-1 --output backups/root -e staging"
    );
}

fn unique_temp_dir(label: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{label}-{}-{nanos}", std::process::id()))
}

// Keep direct network targeting available for local and ad hoc command contexts.
#[test]
fn renders_network_target() {
    let icp = IcpCli::new("icp", None, Some("ic".to_string()));

    assert_eq!(
        icp.snapshot_create_display("aaaaa-aa"),
        "icp canister snapshot create aaaaa-aa --json -n ic"
    );
}

// Keep local replica lifecycle commands explicit and project-scoped.
#[test]
fn renders_local_replica_commands() {
    let icp = IcpCli::new("icp", None, None);

    assert_eq!(
        icp.local_replica_start_display(true, false),
        "icp network start local --background"
    );
    assert_eq!(
        icp.local_replica_start_display(false, false),
        "icp network start local"
    );
    assert_eq!(
        icp.local_replica_start_display(false, true),
        "icp network start local --debug"
    );
    assert_eq!(
        icp.local_replica_status_display(false),
        "icp network status local"
    );
    assert_eq!(
        icp.local_replica_status_display(true),
        "icp network status local --debug"
    );
    assert_eq!(
        icp.local_replica_stop_display(false),
        "icp network stop local"
    );
    assert_eq!(
        icp.local_replica_stop_display(true),
        "icp network stop local --debug"
    );
}

// Keep environment-backed local replica commands aligned with ICP CLI network selection.
#[test]
fn renders_environment_local_replica_commands() {
    let icp = IcpCli::new("icp", Some("staging".to_string()), None);

    assert_eq!(
        icp.local_replica_start_display(true, false),
        "icp network start -e staging --background"
    );
    assert_eq!(
        icp.local_replica_status_display(true),
        "icp network status -e staging --debug"
    );
    assert_eq!(
        icp.local_replica_stop_display(false),
        "icp network stop -e staging"
    );
}

// Keep explicit project roots visible instead of relying only on current_dir.
#[test]
fn renders_project_root_override_for_rooted_context() {
    let icp = IcpCli::new("icp", None, Some("ic".to_string())).with_cwd("/workspace/app");

    assert_eq!(
        icp.canister_top_up_display("aaaaa-aa", 4_000_000_000_000),
        "icp --project-root-override /workspace/app canister top-up --amount 4000000000000 aaaaa-aa -n ic"
    );
}

// Ensure restore planning uses the ICP CLI upload/restore flow.
#[test]
fn renders_snapshot_restore_flow() {
    let icp = IcpCli::new("icp", Some("prod".to_string()), None);

    assert_eq!(
        icp.snapshot_upload_display("root", Path::new("artifact")),
        "icp canister snapshot upload root --input artifact --resume --json -e prod"
    );
    assert_eq!(
        icp.snapshot_restore_display("root", "uploaded-1"),
        "icp canister snapshot restore root uploaded-1 -e prod"
    );
}

// Ensure query helpers do not accidentally issue update calls for read-only endpoint probes.
#[test]
fn renders_no_argument_query_call() {
    let icp = IcpCli::new("icp", None, Some("ic".to_string()));

    assert_eq!(
        icp.canister_query_output_display("root", "canic_ready", Some("json")),
        "icp canister call root canic_ready () --query --json -n ic"
    );
}

// Ensure local Candid support is available to query helpers.
#[test]
fn renders_no_argument_query_call_with_local_candid() {
    let icp = IcpCli::new("icp", None, Some("local".to_string()));

    assert_eq!(
        icp.canister_query_output_display_with_candid(
            "root",
            "canic_ready",
            Some("json"),
            Some(Path::new(".icp/local/canisters/root/root.did"))
        ),
        "icp canister call root canic_ready () --query --candid .icp/local/canisters/root/root.did --json -n local"
    );
}

// Ensure query-call previews preserve the explicit Candid argument.
#[test]
fn renders_argument_query_call_with_local_candid() {
    let icp = IcpCli::new("icp", None, Some("local".to_string()));

    assert_eq!(
        icp.canister_query_arg_output_display_with_candid(
            "root",
            "get_blob_storage_status",
            "(record { sync_gateway_principals = false })",
            Some("json"),
            Some(Path::new(".icp/local/canisters/root/root.did"))
        ),
        "icp canister call root get_blob_storage_status (record { sync_gateway_principals = false }) --query --candid .icp/local/canisters/root/root.did --json -n local"
    );
}

// Ensure update-call previews preserve the explicit Candid argument.
#[test]
fn renders_argument_update_call() {
    let icp = IcpCli::new("icp", None, Some("ic".to_string()));

    assert_eq!(
        icp.canister_call_arg_output_display(
            "root",
            "canic_icp_refill",
            "(record { dry_run = true })",
            Some("json")
        ),
        "icp canister call root canic_icp_refill (record { dry_run = true }) --json -n ic"
    );
}

// Ensure local Candid support is available to update-call helpers.
#[test]
fn renders_argument_update_call_with_local_candid() {
    let icp = IcpCli::new("icp", None, Some("local".to_string()));

    assert_eq!(
        icp.canister_call_arg_output_display_with_candid(
            "root",
            "canic_icp_refill",
            "(record { dry_run = true })",
            Some("json"),
            Some(Path::new(".icp/local/canisters/root/root.did"))
        ),
        "icp canister call root canic_icp_refill (record { dry_run = true }) --candid .icp/local/canisters/root/root.did --json -n local"
    );
}

// Ensure local Candid sidecar resolution matches Canic's ICP CLI artifact layout.
#[test]
fn resolves_existing_local_canister_candid_path() {
    let root = unique_temp_dir("canic-icp-candid-sidecar");
    let did_path = root.join(".icp/local/canisters/root/root.did");
    std::fs::create_dir_all(did_path.parent().expect("did parent")).expect("create did parent");
    std::fs::write(&did_path, "service : {}").expect("write did");

    assert_eq!(local_canister_candid_path(&root, "local", "root"), did_path);
    assert_eq!(
        existing_local_canister_candid_path(&root, "local", "root").as_deref(),
        Some(did_path.as_path())
    );
    assert_eq!(
        existing_local_canister_candid_path(&root, "ic", "root"),
        None
    );

    std::fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure manual top-ups use the ICP CLI top-up command and selected network.
#[test]
fn renders_canister_top_up() {
    let icp = IcpCli::new("icp", None, Some("ic".to_string()));

    assert_eq!(
        icp.canister_top_up_display("aaaaa-aa", 4_000_000_000_000),
        "icp canister top-up --amount 4000000000000 aaaaa-aa -n ic"
    );
}

// Ensure snapshot ids can be extracted from common create output.
#[test]
fn parses_snapshot_id_from_output() {
    let snapshot_id = parse_snapshot_id("Created snapshot: 0a0b0c0d\n");

    assert_eq!(snapshot_id.as_deref(), Some("0a0b0c0d"));
}

// Ensure table units are not mistaken for snapshot ids.
#[test]
fn parses_snapshot_id_from_table_output() {
    let output = "\
ID         SIZE       CREATED_AT
0a0b0c0d   1.37 MiB   2026-05-10T17:04:19Z
";

    let snapshot_id = parse_snapshot_id(output);

    assert_eq!(snapshot_id.as_deref(), Some("0a0b0c0d"));
}

// Ensure current ICP CLI snapshot JSON receipts parse into the typed host shape.
#[test]
fn parses_snapshot_create_receipt_json() {
    let receipt = serde_json::from_str::<IcpSnapshotCreateReceipt>(
        r#"{
  "snapshot_id": "0000000000000000ffffffffffc000020101",
  "taken_at_timestamp": 1778709681897818005,
  "total_size_bytes": 272586987
}"#,
    )
    .expect("parse snapshot receipt");

    assert_eq!(receipt.snapshot_id, "0000000000000000ffffffffffc000020101");
    assert_eq!(receipt.total_size_bytes, Some(272_586_987));
}

// Ensure current ICP CLI snapshot upload JSON receipts parse into the typed host shape.
#[test]
fn parses_snapshot_upload_receipt_json() {
    let receipt = serde_json::from_str::<IcpSnapshotUploadReceipt>(
        r#"{
  "snapshot_id": "0000000000000000ffffffffffc000020101"
}"#,
    )
    .expect("parse snapshot upload receipt");

    assert_eq!(receipt.snapshot_id, "0000000000000000ffffffffffc000020101");
}

// Ensure current ICP CLI status JSON parses into the typed host shape.
#[test]
fn parses_canister_status_report_json() {
    let report = serde_json::from_str::<IcpCanisterStatusReport>(
        r#"{
  "id": "t63gs-up777-77776-aaaba-cai",
  "name": "motoko-ex",
  "status": "Running",
  "settings": {
"controllers": ["zbf4m-zw3nk-6owqc-qmluz-xhwxt-2pkky-xhjy2-kqxor-qzxsn-6d2bz-nae"],
"compute_allocation": "0"
  },
  "module_hash": "0x66ce5ddcd06f1135c1a04792a2f1b7c3d9e229b977a8fc9762c71ecc5314c9eb",
  "cycles": "1_497_896_187_059"
}"#,
    )
    .expect("parse status report");

    assert_eq!(report.status, "Running");
    assert_eq!(
        report.settings.expect("settings").controllers.as_slice(),
        &["zbf4m-zw3nk-6owqc-qmluz-xhwxt-2pkky-xhjy2-kqxor-qzxsn-6d2bz-nae"]
    );
}
