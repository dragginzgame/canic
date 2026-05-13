use super::*;

// Ensure replica start defaults to foreground mode while allowing background use.
#[test]
fn parses_replica_start_options() {
    let options = ReplicaOptions::parse_start([
        OsString::from("--background"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/tmp/icp"),
    ])
    .expect("parse replica start");

    assert_eq!(options.icp, "/tmp/icp");
    assert_eq!(options.port, None);
    assert!(options.background);
    assert!(!options.debug);
}

// Ensure replica start can set the project-local gateway port before launch.
#[test]
fn parses_replica_start_port() {
    let options = ReplicaOptions::parse_start([OsString::from("--port"), OsString::from("8001")])
        .expect("parse replica start");

    assert_eq!(options.port, Some(8001));
}

// Ensure invalid ports fail at CLI parsing before touching icp.yaml.
#[test]
fn rejects_invalid_replica_start_port() {
    let error = ReplicaOptions::parse_start([OsString::from("--port"), OsString::from("0")])
        .expect_err("port 0 should be invalid");

    assert!(matches!(error, ReplicaCommandError::InvalidPort { .. }));
}

// Ensure foreground mode is the default, matching ICP CLI.
#[test]
fn replica_start_defaults_to_foreground() {
    let options = ReplicaOptions::parse_start([]).expect("parse replica start");

    assert_eq!(options.icp, "icp");
    assert_eq!(options.port, None);
    assert!(!options.background);
    assert!(!options.debug);
}

// Ensure replica lifecycle commands can enable ICP CLI debug logging.
#[test]
fn parses_replica_debug_options() {
    let start =
        ReplicaOptions::parse_start([OsString::from("--debug")]).expect("parse replica start");
    let status =
        ReplicaOptions::parse_status([OsString::from("--debug")]).expect("parse replica status");
    let stop = ReplicaOptions::parse_stop([OsString::from("--debug")]).expect("parse replica stop");

    assert!(start.debug);
    assert!(status.debug);
    assert!(stop.debug);
}

// Ensure status uses the default ICP executable when no override is provided.
#[test]
fn parses_replica_status_options() {
    let options = ReplicaOptions::parse_status([]).expect("parse replica status");

    assert_eq!(options.icp, "icp");
    assert_eq!(options.port, None);
    assert!(!options.background);
    assert!(!options.debug);
}

// Ensure replica help exposes the native lifecycle commands.
#[test]
fn replica_usage_lists_commands() {
    let text = usage();

    assert!(text.contains("Manage the local ICP replica"));
    assert!(text.contains("start"));
    assert!(text.contains("status"));
    assert!(text.contains("stop"));
    assert!(text.contains("canic replica status"));
}

// Ensure leaf help documents command-specific options and examples.
#[test]
fn replica_leaf_usage_lists_options() {
    let text = start_usage();

    assert!(text.contains("--background"));
    assert!(text.contains("--port"));
    assert!(text.contains("--debug"));
    assert!(!text.contains("--icp"));
    assert!(text.contains("canic replica start --background"));
    assert!(text.contains("canic replica start --port 8001 --background"));
    assert!(text.contains("canic replica start --debug"));
}

// Ensure ICP's foreign-owner diagnostic is surfaced as an ownership problem.
#[test]
fn maps_foreign_local_replica_owner_error() {
    let error = replica_icp_error(IcpCommandError::Failed {
        command: "icp network start local --background".to_string(),
        stderr: "Error: port 8000 is in use by the local network of the project at '/home/adam/projects/icydb'\n".to_string(),
    });

    assert!(matches!(
        error,
        ReplicaCommandError::ForeignLocalReplicaOwner { ref network, ref project }
            if network == "local" && project == "/home/adam/projects/icydb"
    ));
    assert!(
        error
            .to_string()
            .contains("owned by ICP network `local` for project: /home/adam/projects/icydb")
    );
}

// Ensure ICP's raw missing-project error is replaced with a Canic setup hint.
#[test]
fn maps_missing_project_manifest_error() {
    let error = replica_icp_error(IcpCommandError::Failed {
        command: "icp network start local".to_string(),
        stderr: "Error: failed to locate project directory\n\nCaused by:\n    project manifest not found in /home/adam/projects/toko/backend\n".to_string(),
    });

    assert!(matches!(error, ReplicaCommandError::ProjectManifestMissing));
    assert!(error.to_string().contains("fleets/<fleet>/canic.toml"));
    assert!(
        error
            .to_string()
            .contains("canic fleet sync --fleet <fleet>")
    );
}

// Ensure owner parsing keeps the ICP network/environment separate from the project path.
#[test]
fn parses_foreign_local_replica_owner() {
    let owner = extract_foreign_local_owner(
        "Error: port 8000 is in use by the demo network of the project at '/home/adam/projects/toko'\n",
    )
    .expect("parse foreign owner");

    assert_eq!(
        owner,
        LocalReplicaOwner {
            network: "demo".to_string(),
            project: "/home/adam/projects/toko".to_string(),
        }
    );
}

// Ensure stop can distinguish ICP's project-scoped not-running diagnostic.
#[test]
fn detects_project_local_network_not_running() {
    let error = IcpCommandError::Failed {
        command: "icp network stop local".to_string(),
        stderr: "Error: network 'local' is not running\n".to_string(),
    };

    assert!(local_network_not_running(&error));

    let status_error = IcpCommandError::Failed {
        command: "icp network status local".to_string(),
        stderr: "Error: unable to access network 'local', is it running?\n\nCaused by:\n    the local network for this project is not running\n".to_string(),
    };

    assert!(local_network_not_running(&status_error));
}
