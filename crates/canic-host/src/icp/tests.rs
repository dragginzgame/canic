use super::*;

// Keep generated commands tied to ICP CLI environments when one is selected.
#[test]
fn renders_environment_target() {
    let icp = IcpCli::new("icp", Some("staging".to_string()), Some("ic".to_string()));

    assert_eq!(
        icp.snapshot_download_display("root", "snap-1", Path::new("backups/root")),
        "icp canister snapshot download root snap-1 --output backups/root -e staging"
    );
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

// Keep environment-backed local replica commands aligned with ICP CLI 0.3 network selection.
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

// Keep explicit project roots visible to ICP CLI 0.3 instead of relying only on current_dir.
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
