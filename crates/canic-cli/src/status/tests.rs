use super::*;

// Ensure status defaults to the local network and ordinary `icp` binary.
#[test]
fn parses_status_options() {
    let default_options = StatusOptions::parse([]).expect("parse default options");
    assert_eq!(default_options.network, "local");
    assert_eq!(default_options.icp, "icp");

    let options = StatusOptions::parse([
        OsString::from(crate::cli::globals::INTERNAL_NETWORK_OPTION),
        OsString::from("ic"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/tmp/icp"),
    ])
    .expect("parse explicit options");
    assert_eq!(options.network, "ic");
    assert_eq!(options.icp, "/tmp/icp");
}

// Ensure the compact summary includes replica, deployment count, and fleet rows.
#[test]
fn renders_status_report() {
    let report = StatusReport {
        network: "local".to_string(),
        replica: ReplicaStatus::Running,
        replica_port: "8000".to_string(),
        icp_cli: "icp 0.2.5".to_string(),
        fleets: vec![
            StatusFleetRow {
                fleet: "demo".to_string(),
                deployed: "no".to_string(),
                config: "fleets/demo/canic.toml".to_string(),
                canisters: "2".to_string(),
                root: "-".to_string(),
            },
            StatusFleetRow {
                fleet: "test".to_string(),
                deployed: "yes".to_string(),
                config: "fleets/test/canic.toml".to_string(),
                canisters: "7".to_string(),
                root: "aaaaa-aa".to_string(),
            },
        ],
    };

    assert_eq!(
        render_status_report(&report),
        [
            "Replica: running (local, port 8000)",
            "ICP CLI: icp 0.2.5",
            "Fleets:  1/2 deployed (network local)",
            "",
            "FLEET   DEPLOYED   CONFIG                   CANISTERS   ROOT",
            "-----   --------   ----------------------   ---------   --------",
            "demo    no         fleets/demo/canic.toml   2           -",
            "test    yes        fleets/test/canic.toml   7           aaaaa-aa",
        ]
        .join("\n")
    );
}

// Ensure empty fleet workspaces still render a useful quick status.
#[test]
fn renders_empty_status_report() {
    let report = StatusReport {
        network: "local".to_string(),
        replica: ReplicaStatus::Stopped,
        replica_port: "8001".to_string(),
        icp_cli: "icp 0.2.5".to_string(),
        fleets: Vec::new(),
    };

    assert_eq!(
        render_status_report(&report),
        "Replica: stopped (local, port 8001)\nICP CLI: icp 0.2.5\nFleets:  0/0 deployed (network local)"
    );
}

// Ensure foreground/untracked replicas are visible instead of being silently
// collapsed into ordinary ICP CLI-managed status.
#[test]
fn renders_http_fallback_replica_status() {
    let report = StatusReport {
        network: "local".to_string(),
        replica: ReplicaStatus::RunningHttpFallback,
        replica_port: "8000".to_string(),
        icp_cli: "icp 0.2.6".to_string(),
        fleets: Vec::new(),
    };

    assert_eq!(
        render_status_report(&report),
        "Replica: running (local, port 8000, HTTP reachable; ICP CLI status stopped)\nICP CLI: icp 0.2.6\nFleets:  0/0 deployed (network local)"
    );
}

// Ensure local missing-root rows explain the non-persistent local ICP CLI replica.
#[test]
fn renders_lost_local_fleet_note() {
    let report = StatusReport {
        network: "local".to_string(),
        replica: ReplicaStatus::Running,
        replica_port: "8000".to_string(),
        icp_cli: "icp 0.2.6".to_string(),
        fleets: vec![StatusFleetRow {
            fleet: "test".to_string(),
            deployed: LOCAL_LOST_DEPLOYMENT.to_string(),
            config: "fleets/test/canic.toml".to_string(),
            canisters: "6".to_string(),
            root: "t63gs-up777-77776-aaaba-cai".to_string(),
        }],
    };

    let rendered = render_status_report(&report);

    assert!(rendered.contains("test"));
    assert!(rendered.contains("lost"));
    assert!(rendered.contains("local ICP CLI replica state is not persistent"));
    assert!(rendered.contains("canic install <fleet>"));
}

// Ensure local installed-state rows are not reported as deployed when live roots are unchecked.
#[test]
fn local_deployed_label_is_unknown_without_replica_verification() {
    assert_eq!(
        deployed_label(
            "demo",
            "local",
            "icp",
            std::path::Path::new("."),
            "aaaaa-aa",
            false,
            &["root".to_string()]
        ),
        "unknown"
    );
    assert_eq!(
        deployed_label(
            "demo",
            "ic",
            "icp",
            std::path::Path::new("."),
            "aaaaa-aa",
            false,
            &["root".to_string()]
        ),
        "yes"
    );
}

#[test]
fn local_deployment_is_partial_when_registry_is_missing_configured_roles() {
    let configured_roles = vec!["root".to_string(), "app".to_string()];
    let registry = vec![registry_entry("aaaaa-aa", "root")];

    assert_eq!(
        classify_local_deployment(&configured_roles, &registry),
        "partial"
    );
}

#[test]
fn local_deployment_is_yes_when_registry_contains_configured_roles() {
    let configured_roles = vec!["root".to_string(), "app".to_string()];
    let registry = vec![
        registry_entry("aaaaa-aa", "root"),
        registry_entry("uxrrr-q7777-77774-qaaaq-cai", "app"),
    ];

    assert_eq!(
        classify_local_deployment(&configured_roles, &registry),
        "yes"
    );
}

// Ensure status help points to the compact project summary command.
#[test]
fn status_usage_lists_options_and_examples() {
    let text = usage();

    assert!(text.contains("Show quick Canic project status"));
    assert!(text.contains("Usage: canic status"));
    assert!(!text.contains("--network"));
    assert!(!text.contains("--icp"));
    assert!(text.contains("Examples:"));
    assert!(text.contains("does not persist canister state"));
}

fn registry_entry(pid: &str, role: &str) -> RegistryEntry {
    RegistryEntry {
        pid: pid.to_string(),
        role: Some(role.to_string()),
        kind: None,
        parent_pid: None,
        module_hash: None,
    }
}
