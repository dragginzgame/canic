use super::*;
use crate::test_support::temp_dir;
use std::fs;

// Ensure status defaults to the local environment and ordinary `icp` binary.
#[test]
fn parses_status_options() {
    let default_options = StatusOptions::parse([]).expect("parse default options");
    assert_eq!(default_options.environment, "local");
    assert_eq!(default_options.icp, "icp");

    let options = StatusOptions::parse([
        OsString::from(crate::cli::globals::INTERNAL_ENVIRONMENT_OPTION),
        OsString::from("ic"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/tmp/icp"),
    ])
    .expect("parse explicit options");
    assert_eq!(options.environment, "ic");
    assert_eq!(options.icp, "/tmp/icp");
}

// Ensure App configs and installed Fleets render as independent inventories.
#[test]
fn renders_status_report() {
    let report = StatusReport {
        environment: "local".to_string(),
        replica: ReplicaStatus::Running,
        replica_port: "8000".to_string(),
        icp_cli: "icp 0.2.5".to_string(),
        icp_project: "ok (icp.yaml)".to_string(),
        canonical_network_id: "network-1".to_string(),
        apps: vec![
            StatusAppRow {
                app: "shop".to_string(),
                config: "apps/demo/canic.toml".to_string(),
                canisters: "4".to_string(),
            },
            StatusAppRow {
                app: "test".to_string(),
                config: "apps/test/canic.toml".to_string(),
                canisters: "7".to_string(),
            },
        ],
        fleets: vec![StatusFleetRow {
            fleet: "staging".to_string(),
            app: "shop".to_string(),
            network: "network-1".to_string(),
            deployed: "yes".to_string(),
            root: "aaaaa-aa".to_string(),
        }],
    };

    let rendered = render_status_report(&report);

    assert!(rendered.contains("Apps: 2 configured"));
    assert!(rendered.contains("Fleets: 1/1 deployed (environment local, network network-1)"));
    assert!(rendered.contains("APP    CONFIG                 CANISTERS"));
    assert!(rendered.contains("shop   apps/demo/canic.toml   4"));
    assert!(rendered.contains("FLEET     APP    NETWORK     DEPLOYED   ROOT"));
    assert!(rendered.contains("staging   shop   network-1   yes        aaaaa-aa"));
    assert!(!rendered.contains("DEPLOYMENT"));
}

// Ensure workspaces without App configs still render a useful quick status.
#[test]
fn renders_empty_status_report() {
    let report = StatusReport {
        environment: "local".to_string(),
        replica: ReplicaStatus::Stopped,
        replica_port: "8001".to_string(),
        icp_cli: "icp 0.2.5".to_string(),
        icp_project: "not checked (no Canic App configs)".to_string(),
        canonical_network_id: "network-1".to_string(),
        apps: Vec::new(),
        fleets: Vec::new(),
    };

    assert_eq!(
        render_status_report(&report),
        "Replica: stopped (local, port 8001)\nICP CLI: icp 0.2.5\nICP project: not checked (no Canic App configs)\nApps: 0 configured\nFleets: 0/0 deployed (environment local, network network-1)"
    );
}

// Ensure foreground/untracked replicas are visible instead of being silently
// collapsed into ordinary ICP CLI-managed status.
#[test]
fn renders_http_fallback_replica_status() {
    let report = StatusReport {
        environment: "local".to_string(),
        replica: ReplicaStatus::RunningHttpFallback,
        replica_port: "8000".to_string(),
        icp_cli: "icp 0.2.6".to_string(),
        icp_project: "ok (icp.yaml)".to_string(),
        canonical_network_id: "network-1".to_string(),
        apps: Vec::new(),
        fleets: Vec::new(),
    };

    assert_eq!(
        render_status_report(&report),
        "Replica: running (local, port 8000, HTTP reachable; ICP CLI status stopped)\nICP CLI: icp 0.2.6\nICP project: ok (icp.yaml)\nApps: 0 configured\nFleets: 0/0 deployed (environment local, network network-1)"
    );
}

// Ensure local missing-root rows explain the non-persistent local ICP CLI replica.
#[test]
fn renders_lost_local_fleet_note() {
    let report = StatusReport {
        environment: "local".to_string(),
        replica: ReplicaStatus::Running,
        replica_port: "8000".to_string(),
        icp_cli: "icp 0.2.6".to_string(),
        icp_project: "incomplete (missing canisters: app)".to_string(),
        canonical_network_id: "network-1".to_string(),
        apps: Vec::new(),
        fleets: vec![StatusFleetRow {
            fleet: "test".to_string(),
            app: "shop".to_string(),
            network: "network-1".to_string(),
            deployed: LOCAL_LOST_FLEET.to_string(),
            root: "t63gs-up777-77776-aaaba-cai".to_string(),
        }],
    };

    let rendered = render_status_report(&report);

    assert!(rendered.contains("test"));
    assert!(rendered.contains("lost"));
    assert!(rendered.contains("local ICP CLI replica state is not persistent"));
    assert!(rendered.contains("lost local Fleet"));
    assert!(rendered.contains("canic install <app> <fleet>"));
}

// Ensure status renders config paths relative to the resolved Canic project root.
#[test]
fn status_app_inventory_uses_project_root_for_config_paths() {
    let root = temp_dir("canic-status-project-root");
    let config = root.join("apps/toko/canic.toml");
    fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
    fs::write(
        &config,
        r#"
[app]
name = "toko"

[roles.root]
kind = "root"
package = "root"

[roles.app]
kind = "canister"
package = "app"

[auth.delegated_tokens]
enabled = false



[component_specs.app]
component_role = "app"
maximum_instances = 1
"#,
    )
    .expect("write config");

    let inventory = load_status_apps(&root, std::slice::from_ref(&config));
    let row = inventory.rows.first().expect("status App row");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(row.app, "toko");
    assert_eq!(row.config, "apps/toko/canic.toml");
    assert_eq!(row.canisters, "2");
    assert_eq!(
        inventory.bootstrap_roles_by_app.get("toko"),
        Some(&vec!["root".to_string(), "app".to_string()])
    );
}

// Ensure local installed-state rows are not reported as deployed when live roots are unchecked.
#[test]
fn local_deployed_label_is_unknown_without_replica_verification() {
    let fleet = fleet_catalog_entry();
    assert_eq!(
        deployed_label(
            &fleet,
            &StatusOptions {
                environment: "local".to_string(),
                icp: "icp".to_string(),
            },
            std::path::Path::new("."),
            false,
            Some(&["root".to_string()])
        ),
        "unknown"
    );
    assert_eq!(
        deployed_label(
            &fleet,
            &StatusOptions {
                environment: "ic".to_string(),
                icp: "icp".to_string(),
            },
            std::path::Path::new("."),
            false,
            Some(&["root".to_string()])
        ),
        "yes"
    );
}

#[test]
fn fleet_status_uses_explicit_catalog_app_binding() {
    let fleet = fleet_catalog_entry();
    let row = status_fleet_row(
        std::path::Path::new("."),
        &fleet,
        &StatusOptions {
            environment: "local".to_string(),
            icp: "icp".to_string(),
        },
        false,
        None,
    );

    assert_eq!(row.fleet, "demo");
    assert_eq!(row.app, "shop");
    assert_ne!(row.fleet, row.app);
    assert_eq!(row.network, fleet.canonical_network_id.to_string());
}

#[test]
fn local_fleet_is_partial_when_registry_is_missing_configured_roles() {
    let configured_roles = vec!["root".to_string(), "app".to_string()];
    let registry = vec![registry_entry("aaaaa-aa", "root")];

    assert_eq!(
        classify_local_fleet(&configured_roles, &registry),
        "partial"
    );
}

#[test]
fn local_fleet_is_yes_when_registry_contains_configured_roles() {
    let configured_roles = vec!["root".to_string(), "app".to_string()];
    let registry = vec![
        registry_entry("aaaaa-aa", "root"),
        registry_entry("uxrrr-q7777-77774-qaaaq-cai", "app"),
    ];

    assert_eq!(classify_local_fleet(&configured_roles, &registry), "yes");
}

// Ensure status help points to the compact project summary command.
#[test]
fn status_usage_lists_options_and_examples() {
    let text = usage();

    assert!(text.contains("Show quick Canic project status"));
    assert!(text.contains("Usage: canic status"));
    assert!(!text.contains("--environment"));
    assert!(!text.contains("--icp"));
    assert!(text.contains("Examples:"));
    assert!(text.contains("does not persist canister state"));
}

fn registry_entry(pid: &str, role: &str) -> RegistryEntry {
    RegistryEntry {
        pid: pid.to_string(),
        role: Some(role.to_string()),
        parent_pid: None,
        module_hash: None,
    }
}

fn fleet_catalog_entry() -> FleetCatalogEntryV1 {
    FleetCatalogEntryV1 {
        canonical_network_id: canic_core::ids::CanonicalNetworkId::public_ic(),
        fleet_id: canic_core::ids::FleetId::from_generated_bytes([1; 32]),
        fleet_name: "demo".parse().expect("Fleet name"),
        app: canic_core::ids::AppId::from("shop"),
        environment: "local".to_string(),
        deployed_at_unix_secs: 1,
        root_principal: "aaaaa-aa".to_string(),
    }
}
