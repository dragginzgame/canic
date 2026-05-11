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
    assert!(options.background);
    assert!(!options.debug);
}

// Ensure foreground mode is the default, matching ICP CLI.
#[test]
fn replica_start_defaults_to_foreground() {
    let options = ReplicaOptions::parse_start([]).expect("parse replica start");

    assert_eq!(options.icp, "icp");
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
    assert!(text.contains("--debug"));
    assert!(!text.contains("--icp"));
    assert!(text.contains("canic replica start --background"));
    assert!(text.contains("canic replica start --debug"));
}
