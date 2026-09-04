use super::*;
use crate::test_support::temp_dir;
use candid::Principal;
use canic_core::{
    dto::fleet_registry::FleetRegistry,
    ids::{
        AppId, CanonicalNetworkId, FleetAdmissionPolicy, FleetBinding, FleetCoordinatorBinding,
        FleetId, FleetKey, FleetRegistryAuthority, SubnetId,
    },
};
use canic_host::fleet_ensure::{
    CurrentFleetInventoryError,
    model::{
        CycleConservation, FLEET_ENSURE_SCHEMA_VERSION, FleetEnsureCompletion,
        FleetEnsureJournalRecord, FleetEnsurePlan, FleetEnsurePlanScope, FleetEnsureStateRecord,
        FleetEnsureTopologyRecord,
    },
    ops::{EnsurePaths, read_journal, write_journal, write_plan, write_state},
    resolve_current_fleet,
};
use std::{collections::BTreeMap, fs};

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

// Ensure App configs and current Fleets render as independent inventories.
#[test]
fn renders_status_report() {
    let report = StatusReport {
        environment: "local".to_string(),
        replica: ReplicaStatus::Running,
        replica_port: "8000".to_string(),
        icp_cli: "icp 0.2.5".to_string(),
        icp_config: "ok (icp.yaml)".to_string(),
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
            coordinator: "aaaaa-aa".to_string(),
        }],
    };

    let rendered = render_status_report(&report);

    assert!(rendered.contains("Apps: 2 configured"));
    assert!(rendered.contains("Fleets: 1/1 deployed (environment local, network network-1)"));
    assert!(rendered.contains("APP    CONFIG                 CANISTERS"));
    assert!(rendered.contains("shop   apps/demo/canic.toml   4"));
    assert!(rendered.contains("FLEET     APP    NETWORK     DEPLOYED   COORDINATOR"));
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
        icp_config: "not checked (no Canic App configs)".to_string(),
        canonical_network_id: "network-1".to_string(),
        apps: Vec::new(),
        fleets: Vec::new(),
    };

    assert_eq!(
        render_status_report(&report),
        "Replica: stopped (local, port 8001)\nICP CLI: icp 0.2.5\nICP config: not checked (no Canic App configs)\nApps: 0 configured\nFleets: 0/0 deployed (environment local, network network-1)"
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
        icp_config: "ok (icp.yaml)".to_string(),
        canonical_network_id: "network-1".to_string(),
        apps: Vec::new(),
        fleets: Vec::new(),
    };

    assert_eq!(
        render_status_report(&report),
        "Replica: running (local, port 8000, HTTP reachable; ICP CLI status stopped)\nICP CLI: icp 0.2.6\nICP config: ok (icp.yaml)\nApps: 0 configured\nFleets: 0/0 deployed (environment local, network network-1)"
    );
}

// Ensure status renders config paths relative to the resolved Canic workspace root.
#[test]
fn status_app_inventory_uses_workspace_root_for_config_paths() {
    let root = temp_dir("canic-status-workspace-root");
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
    let row = inventory.first().expect("status App row");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(row.app, "toko");
    assert_eq!(row.config, "apps/toko/canic.toml");
    assert_eq!(row.canisters, "2");
}

#[test]
fn fleet_status_uses_terminal_registry_app_binding() {
    let fleet = current_fleet_summary();
    let row = status_fleet_row(&fleet);

    assert_eq!(row.fleet, "demo");
    assert_eq!(row.app, "shop");
    assert_ne!(row.fleet, row.app);
    assert_eq!(row.network, fleet.canonical_network_id.to_string());
    assert_eq!(row.deployed, "yes");
    assert_eq!(row.coordinator, fleet.coordinator);
}

#[test]
fn terminal_ensure_is_the_status_fleet_source() {
    let root = temp_dir("canic-status-terminal-ensure-reproduction");
    fs::create_dir_all(&root).expect("create workspace root");
    fs::write(
        root.join("icp.yaml"),
        "environments:\n  - name: staging\n    network: ic\n  - name: production\n    network: ic\n",
    )
    .expect("write ICP config");
    retain_terminal_fleet(&root, "staging", "demo");
    retain_terminal_fleet(&root, "staging", "pending");
    let pending_paths = EnsurePaths::under(&root, "staging", "pending");
    let mut pending = read_journal(&pending_paths)
        .expect("read pending journal")
        .expect("pending journal");
    pending.completion = FleetEnsureCompletion::InProgress;
    write_journal(&pending_paths, &pending).expect("retain pending journal");

    resolve_current_fleet(&root, "staging", "demo").expect("terminal Fleet authority");
    let discovery = discover_current_fleets(&root, "staging").expect("status Fleet source");
    let production = discover_current_fleets(&root, "production").expect("production status");
    let rows = discovery
        .fleets
        .iter()
        .map(status_fleet_row)
        .collect::<Vec<_>>();

    assert_eq!(discovery.environment, "staging");
    assert!(production.fleets.is_empty());
    assert_eq!(
        discovery.canonical_network_id,
        CanonicalNetworkId::ic_mainnet()
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].fleet, "demo");
    assert_eq!(rows[0].app, "shop");
    assert_eq!(
        rows[0].network,
        CanonicalNetworkId::ic_mainnet().to_string()
    );
    assert_eq!(rows[0].deployed, "yes");
    assert_eq!(rows[0].coordinator, "rrkah-fqaaa-aaaaa-aaaaq-cai");

    retain_terminal_fleet(&root, "staging", "duplicate");
    assert!(matches!(
        discover_current_fleets(&root, "staging"),
        Err(CurrentFleetInventoryError::DuplicateDiscoveryAuthority {
            field: "Fleet ID",
            ..
        })
    ));

    fs::remove_dir_all(root).expect("remove fixture");
}

// Ensure Fleet discovery never follows an environment-level directory symlink.
#[cfg(unix)]
#[test]
fn terminal_ensure_discovery_rejects_symlinked_environment_directory() {
    let root = temp_dir("canic-status-terminal-ensure-symlink");
    let target = root.join("terminal-ensure-target");
    let discovery_parent = root.join(".canic/fleet-ensure");
    fs::create_dir_all(&target).expect("create symlink target");
    fs::create_dir_all(&discovery_parent).expect("create discovery parent");
    fs::write(
        root.join("icp.yaml"),
        "environments:\n  - name: staging\n    network: ic\n",
    )
    .expect("write ICP config");
    std::os::unix::fs::symlink(&target, discovery_parent.join("staging"))
        .expect("symlink discovery directory");

    assert!(matches!(
        discover_current_fleets(&root, "staging"),
        Err(CurrentFleetInventoryError::UnsafeDiscoveryPath { .. })
    ));

    fs::remove_dir_all(root).expect("remove fixture");
}

// Ensure status help points to the compact workspace summary command.
#[test]
fn status_usage_lists_options_and_examples() {
    let text = usage();

    assert!(text.contains("Show quick local workspace status"));
    assert!(text.contains("Usage: canic status"));
    assert!(!text.contains("--environment"));
    assert!(!text.contains("--icp"));
    assert!(text.contains("Examples:"));
    assert!(text.contains("validated terminal Fleet Ensure state"));
    assert!(text.contains("does not query live Coordinator"));
}

fn current_fleet_summary() -> CurrentFleetSummary {
    CurrentFleetSummary {
        app: AppId::from("shop"),
        canonical_network_id: canic_core::ids::CanonicalNetworkId::ic_mainnet(),
        coordinator: "aaaaa-aa".to_string(),
        fleet: "demo".parse().expect("Fleet name"),
        fleet_id: FleetId::from_generated_bytes([7; 32]),
    }
}

fn retain_terminal_fleet(root: &Path, environment: &str, fleet_name: &str) {
    let network = CanonicalNetworkId::ic_mainnet();
    let coordinator =
        Principal::from_text("rrkah-fqaaa-aaaaa-aaaaq-cai").expect("Coordinator Principal");
    let registry = terminal_registry(network, coordinator);
    let operation_id = "11".repeat(32);
    let plan_sha256 = "22".repeat(32);
    let paths = EnsurePaths::under(root, environment, fleet_name);
    write_state(
        &paths,
        &FleetEnsureStateRecord {
            active_registry: Some(registry),
            completed_reinstall_action_sha256: BTreeMap::new(),
            completed_reinstall_operation_id: None,
            completed_reinstalls: BTreeMap::new(),
            fleet: fleet_name.to_string(),
            pending_principals: BTreeMap::new(),
            principals: BTreeMap::from([("coordinator".to_string(), coordinator.to_text())]),
            retained_cycles_by_principal: BTreeMap::new(),
            schema_version: FLEET_ENSURE_SCHEMA_VERSION,
            topology: BTreeMap::from([(
                "coordinator".to_string(),
                FleetEnsureTopologyRecord {
                    kind: canic_host::fleet_ensure::model::DesiredCanisterKind::Coordinator,
                    module_hash: None,
                    parent: None,
                    protocol_binding: None,
                    role: None,
                },
            )]),
        },
    )
    .expect("write terminal state");
    write_plan(
        &paths,
        &FleetEnsurePlan {
            canisters: Vec::new(),
            conservation: CycleConservation {
                estate_funding_domains: Vec::new(),
                expected_post_operation_cycles: 0,
                maximum_execution_burn_cycles: 0,
                maximum_new_funding_cycles: 0,
                maximum_operator_debit_cycles: 0,
                maximum_unavoidable_fee_cycles: 0,
                observed_controlled_cycles: 0,
                retained_in_reused_canisters_cycles: 0,
                scheduled_transfer_cycles: 0,
            },
            desired_sha256: "33".repeat(32),
            environment: environment.to_string(),
            fleet: fleet_name.to_string(),
            operation_id: operation_id.clone(),
            plan_sha256: plan_sha256.clone(),
            planned_at_time: 1,
            protocol_actions: Vec::new(),
            root_start_authority: None,
            reviewed_desired: None,
            schema_version: FLEET_ENSURE_SCHEMA_VERSION,
            scope: FleetEnsurePlanScope::Full,
            terminal_inventory_operation_id: Some(operation_id.clone()),
        },
    )
    .expect("write terminal plan");
    write_journal(
        &paths,
        &FleetEnsureJournalRecord {
            completion: FleetEnsureCompletion::Converged,
            estate_funding_required: None,
            effects: Vec::new(),
            fleet: fleet_name.to_string(),
            initial_controlled_cycles: 0,
            initial_estate_funding_cycles_by_root: BTreeMap::new(),
            initial_operator_cycles: 0,
            operation_id,
            plan_sha256,
            schema_version: FLEET_ENSURE_SCHEMA_VERSION,
            stalled_observations: 0,
        },
    )
    .expect("write terminal journal");
}

fn terminal_registry(network: CanonicalNetworkId, coordinator: Principal) -> FleetRegistry {
    let fleet = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: network,
            fleet_id: FleetId::from_generated_bytes([7; 32]),
        },
        app: AppId::from("shop"),
    };
    FleetRegistry {
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: fleet.clone(),
                coordinator_subnet: SubnetId::from_principal(
                    Principal::from_text("rwlgt-iiaaa-aaaaa-aaaaa-cai")
                        .expect("Coordinator Subnet Principal"),
                ),
                coordinator,
            },
            epoch: 1,
        },
        revision: 1,
        admission: FleetAdmissionPolicy {
            schema_version: 1,
            fleet,
            generation: 1,
            fleet_principals: Vec::new(),
            rules: Vec::new(),
            policy_digest: [0; 32],
        },
        component_specs: Vec::new(),
        fleet_subnet_roots: Vec::new(),
        services: Vec::new(),
    }
}
