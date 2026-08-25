use super::{
    model::IcpCliVersion,
    version::{is_supported_icp_cli_version, parse_icp_cli_version},
    *,
};
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
fn icp_cli_version_range_requires_1_2_or_newer_within_major_one() {
    assert!(!is_supported_icp_cli_version(IcpCliVersion {
        major: 0,
        minor: 0,
        patch: 0
    }));
    assert!(!is_supported_icp_cli_version(IcpCliVersion {
        major: 1,
        minor: 0,
        patch: 0
    }));
    assert!(!is_supported_icp_cli_version(IcpCliVersion {
        major: 1,
        minor: 1,
        patch: 0
    }));
    assert!(is_supported_icp_cli_version(IcpCliVersion {
        major: 1,
        minor: 2,
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
            .contains("required: icp-cli >=1.2.0, <2.0.0")
    );
    assert!(
        err.to_string()
            .contains("icp network update` updates the local network launcher")
    );

    fs::remove_dir_all(root).expect("remove temp dir");
}

#[cfg(unix)]
#[test]
fn command_output_retries_a_transient_executable_busy_race() {
    use std::fs::{self, OpenOptions};
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;

    let root = unique_temp_dir("canic-icp-executable-busy");
    fs::create_dir_all(&root).expect("create temp dir");
    let executable = root.join("icp");
    fs::write(&executable, "#!/bin/sh\nprintf '%s\\n' ready\n").expect("write fake executable");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
        .expect("make fake executable runnable");
    let writer = OpenOptions::new()
        .write(true)
        .open(&executable)
        .expect("hold executable open for writing");
    let release_writer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(8));
        drop(writer);
    });
    let mut command = Command::new(&executable);

    let output = crate::output_with_executable_busy_retry(&mut command)
        .expect("retry transient executable-busy failure");
    release_writer
        .join()
        .expect("release fake executable writer");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "ready");
    fs::remove_dir_all(root).expect("remove temp dir");
}

fn unique_temp_dir(label: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{label}-{}-{nanos}", std::process::id()))
}

// Keep explicit project roots visible instead of relying only on current_dir.
#[test]
fn renders_project_root_override_for_rooted_context() {
    let icp = IcpCli::new("icp", Some("ic".to_string())).with_cwd("/workspace/app");

    assert_eq!(
        icp.canister_top_up_display("aaaaa-aa", 4_000_000_000_000),
        "icp --project-root-override /workspace/app canister top-up --amount 4000000000000 aaaaa-aa -e ic"
    );
}

#[test]
fn relays_identity_password_file_without_reading_secret_material() {
    let icp = IcpCli::new("icp", Some("ic".to_string()))
        .with_identity_password_file("/run/user/1000/canic-mainnet.password");

    assert_eq!(
        icp.canister_top_up_display("aaaaa-aa", 4_000_000_000_000),
        "icp --identity-password-file /run/user/1000/canic-mainnet.password canister top-up --amount 4000000000000 aaaaa-aa -e ic"
    );
}

// Ensure query-call previews preserve the explicit Candid argument.
#[test]
fn renders_argument_query_call_with_local_candid() {
    let icp = IcpCli::new("icp", Some("local".to_string()));

    assert_eq!(
        icp.canister_query_arg_output_display_with_candid(
            "root",
            "get_blob_storage_status",
            "(record { sync_gateway_principals = false })",
            Some("json"),
            Some(Path::new(".icp/local/canisters/root/root.did"))
        ),
        "icp canister call root get_blob_storage_status (record { sync_gateway_principals = false }) --query --candid .icp/local/canisters/root/root.did --json -e local"
    );
}

// Ensure local Candid support is available to update-call helpers.
#[test]
fn renders_argument_update_call_with_local_candid() {
    let icp = IcpCli::new("icp", Some("local".to_string()));

    assert_eq!(
        icp.canister_call_arg_output_display_with_candid(
            "root",
            "canic_command",
            "(variant { RefillCycles = record { amount_e8s = 100000000 : nat64 } })",
            Some("json"),
            Some(Path::new(".icp/local/canisters/root/root.did"))
        ),
        "icp canister call root canic_command (variant { RefillCycles = record { amount_e8s = 100000000 : nat64 } }) --candid .icp/local/canisters/root/root.did --json -e local"
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

// Ensure manual top-ups use the ICP CLI top-up command and selected environment.
#[test]
fn renders_canister_top_up() {
    let icp = IcpCli::new("icp", Some("ic".to_string()));

    assert_eq!(
        icp.canister_top_up_display("aaaaa-aa", 4_000_000_000_000),
        "icp canister top-up --amount 4000000000000 aaaaa-aa -e ic"
    );
}

#[test]
fn root_deletion_burns_only_the_bounded_reserve_without_installing_a_recovery_shim() {
    let icp = IcpCli::new("icp", Some("ic".to_string()));

    assert_eq!(
        icp.delete_canister_without_cycle_recovery_display("aaaaa-aa"),
        "icp canister delete --no-recover-cycles aaaaa-aa -e ic"
    );
}

// Ensure current ICP CLI snapshot JSON metadata parses into the typed host shape.
#[test]
fn parses_snapshot_json() {
    let snapshot = serde_json::from_str::<IcpSnapshot>(
        r#"{
  "snapshot_id": "0000000000000000ffffffffffc000020101",
  "taken_at_timestamp": 1778709681897818005,
  "total_size_bytes": 272586987
}"#,
    )
    .expect("parse snapshot metadata");

    assert_eq!(snapshot.snapshot_id, "0000000000000000ffffffffffc000020101");
    assert_eq!(snapshot.total_size_bytes, Some(272_586_987));
}

#[test]
fn parses_snapshot_inventory_json() {
    let inventory = serde_json::from_str::<super::snapshot::IcpSnapshotInventory>(
        r#"{"snapshots":[{
  "snapshot_id": "0000000000000000ffffffffffc000020101",
  "taken_at_timestamp": 1778709681897818005,
  "total_size_bytes": 272586987
}]}"#,
    )
    .expect("parse snapshot inventory");

    let snapshots = inventory.snapshots;
    assert_eq!(snapshots.len(), 1);
    assert_eq!(
        snapshots[0].snapshot_id,
        "0000000000000000ffffffffffc000020101"
    );
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
