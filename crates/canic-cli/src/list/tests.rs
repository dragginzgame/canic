use super::*;
use canic_host::registry::parse_registry_entries;
use canic_host::replica_query;
use options::ListSource;
use render::ReadyStatus;
use serde_json::json;
use std::collections::BTreeMap;

const ROOT: &str = "aaaaa-aa";
const APP: &str = "renrk-eyaaa-aaaaa-aaada-cai";
const APP_VARIANT: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";
const MINIMAL: &str = "rrkah-fqaaa-aaaaa-aaaaq-cai";
const WORKER: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const HASH_PREFIX: &str = "01234567";
const VARIANT_HASH: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
const VARIANT_HASH_PREFIX: &str = "abcdef01";
// Ensure list options parse live registry queries.
#[test]
fn parses_live_list_options() {
    let options = ListOptions::parse_list([
        OsString::from("demo"),
        OsString::from("--subtree"),
        OsString::from(APP),
        OsString::from(crate::cli::globals::INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/bin/icp"),
    ])
    .expect("parse list options");

    assert_eq!(options.source, ListSource::RootRegistry);
    assert_eq!(options.fleet, "demo");
    assert_eq!(options.subtree, Some(APP.to_string()));
    assert_eq!(options.network, Some("local".to_string()));
    assert_eq!(options.icp, "/bin/icp");
    assert!(!options.verbose);
}

// Ensure config options parse declared fleet inspection.
#[test]
fn parses_config_options() {
    let options = ListOptions::parse_config([
        OsString::from("demo"),
        OsString::from(crate::cli::globals::INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
        OsString::from("-v"),
    ])
    .expect("parse config options");

    assert_eq!(options.source, ListSource::Config);
    assert_eq!(options.fleet, "demo");
    assert_eq!(options.subtree, None);
    assert_eq!(options.network, Some("local".to_string()));
    assert_eq!(options.icp, "icp");
    assert!(options.verbose);
}

// Ensure config rejects deployed-list-only subtree anchors.
#[test]
fn config_rejects_subtree_option() {
    let err = ListOptions::parse_config([
        OsString::from("demo"),
        OsString::from("--subtree"),
        OsString::from("user_hub"),
    ])
    .expect_err("config --subtree should fail");

    assert!(matches!(err, ListCommandError::Usage(_)));
}

// Ensure removed list selectors are hard rejected.
#[test]
fn list_rejects_removed_root_and_from_options() {
    let root_err = ListOptions::parse_list([
        OsString::from("demo"),
        OsString::from("--root"),
        OsString::from("aaaaa-aa"),
    ])
    .expect_err("list --root should fail");
    let from_err = ListOptions::parse_list([
        OsString::from("demo"),
        OsString::from("--from"),
        OsString::from("user_hub"),
    ])
    .expect_err("list --from should fail");

    assert!(matches!(root_err, ListCommandError::Usage(_)));
    assert!(matches!(from_err, ListCommandError::Usage(_)));
}

// Ensure list and config help explain fleet selection and subtree rendering.
#[test]
fn list_and_config_usage_explain_fleet_and_subtree_options() {
    let list = usage();
    let config = config_usage();

    assert!(list.contains("List canisters registered by the deployed root"));
    assert!(list.contains("Usage: canic list [OPTIONS] <fleet>"));
    assert!(list.contains("<fleet>"));
    assert!(!list.contains("--fleet <name>"));
    assert!(list.contains("--subtree <name-or-principal>"));
    assert!(list.contains("--verbose"));
    assert!(!list.contains("--from"));
    assert!(!list.contains("--root"));
    assert!(config.contains("Usage: canic config [OPTIONS] <fleet>"));
    assert!(config.contains("<fleet>"));
    assert!(!config.contains("--fleet <name>"));
    assert!(!config.contains("--subtree"));
    assert!(!config.contains("--from"));
    assert!(config.contains("--verbose"));
    assert!(config.contains("-v"));
    assert!(config.contains("Examples:"));
}

// Ensure registry entries render as a stable whitespace table.
#[test]
fn renders_registry_table() {
    let registry = parse_registry_entries(&registry_json()).expect("parse registry");
    let readiness = readiness_map();
    let module_hashes = module_hash_map();
    let empty = BTreeMap::new();
    let columns = RegistryColumnData {
        readiness: &readiness,
        canic_versions: &empty,
        module_hashes: &module_hashes,
        wasm_sizes: &empty,
        cycles: &empty,
        full_module_hashes: false,
        color_module_variants: false,
    };
    let tree = render_registry_tree(&registry, None, &columns).expect("render tree");
    let widths = [12, 8, 27, 5, 5, 4, 6];

    assert_eq!(
        tree,
        [
            render_registry_table_row(
                &[
                    ROLE_HEADER,
                    MODULE_HEADER,
                    CANISTER_HEADER,
                    READY_HEADER,
                    CANIC_HEADER,
                    WASM_HEADER,
                    CYCLES_HEADER,
                ],
                &widths
            ),
            render_registry_separator(&widths),
            render_registry_table_row(&["root", HASH_PREFIX, ROOT, "yes", "-", "-", "-"], &widths),
            render_registry_table_row(&["├─ app", HASH_PREFIX, APP, "no", "-", "-", "-"], &widths),
            render_registry_table_row(
                &["│  └─ worker", HASH_PREFIX, WORKER, "error", "-", "-", "-"],
                &widths
            ),
            render_registry_table_row(
                &["└─ minimal", HASH_PREFIX, MINIMAL, "yes", "-", "-", "-"],
                &widths
            )
        ]
        .join("\n")
    );
}

// Ensure verbose registry output shows full module hashes.
#[test]
fn renders_verbose_registry_table_with_full_module_hashes() {
    let registry = parse_registry_entries(&registry_json()).expect("parse registry");
    let readiness = readiness_map();
    let module_hashes = module_hash_map();
    let empty = BTreeMap::new();
    let columns = RegistryColumnData {
        readiness: &readiness,
        canic_versions: &empty,
        module_hashes: &module_hashes,
        wasm_sizes: &empty,
        cycles: &empty,
        full_module_hashes: true,
        color_module_variants: false,
    };
    let tree = render_registry_tree(&registry, Some(APP), &columns).expect("render tree");

    assert!(tree.contains(MODULE_HASH_HEADER));
    assert!(tree.contains(HASH));
}

// Ensure different roles are not colored just because they use different modules.
#[test]
fn module_hash_color_ignores_cross_role_differences() {
    let registry = parse_registry_entries(&registry_json()).expect("parse registry");
    let readiness = readiness_map();
    let module_hashes = BTreeMap::from([
        (ROOT.to_string(), VARIANT_HASH.to_string()),
        (APP.to_string(), HASH.to_string()),
    ]);
    let empty = BTreeMap::new();
    let columns = RegistryColumnData {
        readiness: &readiness,
        canic_versions: &empty,
        module_hashes: &module_hashes,
        wasm_sizes: &empty,
        cycles: &empty,
        full_module_hashes: false,
        color_module_variants: true,
    };
    let tree = render_registry_tree(&registry, None, &columns).expect("render tree");

    assert!(!tree.contains("\x1b[38;5;179m"));
}

// Ensure module coloring flags drift only within one repeated role.
#[test]
fn module_hash_color_flags_same_role_differences() {
    let registry = parse_registry_entries(&same_role_variant_registry_json())
        .expect("parse same-role registry");
    let readiness = readiness_map();
    let module_hashes = BTreeMap::from([
        (APP.to_string(), HASH.to_string()),
        (APP_VARIANT.to_string(), VARIANT_HASH.to_string()),
    ]);
    let empty = BTreeMap::new();
    let columns = RegistryColumnData {
        readiness: &readiness,
        canic_versions: &empty,
        module_hashes: &module_hashes,
        wasm_sizes: &empty,
        cycles: &empty,
        full_module_hashes: false,
        color_module_variants: true,
    };
    let tree = render_registry_tree(&registry, None, &columns).expect("render tree");

    assert!(tree.contains(&format!("\x1b[38;5;179m{VARIANT_HASH_PREFIX}")));
    assert!(!tree.contains(&format!("\x1b[38;5;179m{HASH_PREFIX}")));
}

// Ensure one selected subtree can be rendered without siblings.
#[test]
fn renders_selected_subtree() {
    let registry = parse_registry_entries(&registry_json()).expect("parse registry");
    let readiness = readiness_map();
    let module_hashes = module_hash_map();
    let empty = BTreeMap::new();
    let columns = RegistryColumnData {
        readiness: &readiness,
        canic_versions: &empty,
        module_hashes: &module_hashes,
        wasm_sizes: &empty,
        cycles: &empty,
        full_module_hashes: false,
        color_module_variants: false,
    };
    let tree = render_registry_tree(&registry, Some(APP), &columns).expect("render subtree");
    let widths = [9, 8, 27, 5, 5, 4, 6];

    assert_eq!(
        tree,
        [
            render_registry_table_row(
                &[
                    ROLE_HEADER,
                    MODULE_HEADER,
                    CANISTER_HEADER,
                    READY_HEADER,
                    CANIC_HEADER,
                    WASM_HEADER,
                    CYCLES_HEADER,
                ],
                &widths
            ),
            render_registry_separator(&widths),
            render_registry_table_row(&["app", HASH_PREFIX, APP, "no", "-", "-", "-"], &widths),
            render_registry_table_row(
                &["└─ worker", HASH_PREFIX, WORKER, "error", "-", "-", "-"],
                &widths
            )
        ]
        .join("\n")
    );
}

// Ensure the full list output names the selected fleet before the tree table.
#[test]
fn renders_list_output_with_fleet_title() {
    let registry = parse_registry_entries(&registry_json()).expect("parse registry");
    let title = ListTitle {
        fleet: "demo".to_string(),
        network: "local".to_string(),
    };
    let readiness = readiness_map();
    let module_hashes = module_hash_map();
    let empty = BTreeMap::new();
    let columns = RegistryColumnData {
        readiness: &readiness,
        canic_versions: &empty,
        module_hashes: &module_hashes,
        wasm_sizes: &empty,
        cycles: &empty,
        full_module_hashes: false,
        color_module_variants: false,
    };
    let output = render_list_output(&title, &registry, Some(APP), &columns, &[])
        .expect("render list output");

    assert!(output.starts_with("Fleet: demo (network local)\n\nROLE"));
    assert!(output.contains("CANISTER_ID"));
}

#[test]
fn renders_list_output_with_wasm_size_and_missing_roles() {
    let registry = parse_registry_entries(&registry_json()).expect("parse registry");
    let title = ListTitle {
        fleet: "demo".to_string(),
        network: "local".to_string(),
    };
    let canic_versions = BTreeMap::from([(APP.to_string(), "0.33.6".to_string())]);
    let wasm_sizes = BTreeMap::from([("app".to_string(), "1.52 MiB (gz 811.20 KiB)".to_string())]);
    let cycles = BTreeMap::from([(APP.to_string(), "12.35 TC".to_string())]);
    let readiness = readiness_map();
    let module_hashes = module_hash_map();
    let columns = RegistryColumnData {
        readiness: &readiness,
        canic_versions: &canic_versions,
        module_hashes: &module_hashes,
        wasm_sizes: &wasm_sizes,
        cycles: &cycles,
        full_module_hashes: false,
        color_module_variants: false,
    };
    let output = render_list_output(&title, &registry, None, &columns, &["audit".to_string()])
        .expect("render list output");

    assert!(output.contains("WASM"));
    assert!(output.contains("CYCLES"));
    assert!(output.contains("0.33.6"));
    assert!(output.contains("1.52 MiB"));
    assert!(output.contains("811.20 KiB"));
    assert!(output.contains("12.35 TC"));
    assert!(output.contains("Missing roles: audit"));
}

// Ensure config-only fleets render their declared roles instead of deployed inventory.
#[test]
fn renders_config_output_with_fleet_roles() {
    let title = ListTitle {
        fleet: "test_me".to_string(),
        network: "local".to_string(),
    };
    let rows = vec![
        ConfigRoleRow {
            role: "root".to_string(),
            kind: "root".to_string(),
            capabilities: "-".to_string(),
            auto_create: "-".to_string(),
            topup: "-".to_string(),
            metrics: "root".to_string(),
            details: Vec::new(),
        },
        ConfigRoleRow {
            role: "app".to_string(),
            kind: "singleton".to_string(),
            capabilities: "auth, sharding".to_string(),
            auto_create: "yes".to_string(),
            topup: "4.00 TC @ 10.00 TC".to_string(),
            metrics: "hub".to_string(),
            details: vec![
                "app_index".to_string(),
                "metrics profile=hub tiers=core,placement,runtime,security (inferred)".to_string(),
                "sharding user_shards->user_shard cap=100 initial=1 max=4".to_string(),
            ],
        },
    ];
    let output = render_config_output(&title, &rows, true);

    assert_eq!(
        output,
        [
            "Fleet: test_me (network local)",
            "",
            "ROLE   KIND        AUTO   CAPS             METRICS   TOPUP",
            "----   ---------   ----   --------------   -------   ------------------",
            "root   root        -      -                root      -",
            "app    singleton   yes    auth, sharding   hub       4.00 TC @ 10.00 TC",
            "  - app_index",
            "  - metrics profile=hub tiers=core,placement,runtime,security (inferred)",
            "  - sharding user_shards->user_shard cap=100 initial=1 max=4",
        ]
        .join("\n")
    );
}

// Ensure readiness parsing accepts common command-line JSON shapes.
#[test]
fn parses_ready_json_shapes() {
    assert!(replica_query::parse_ready_json_value(&json!(true)));
    assert!(replica_query::parse_ready_json_value(
        &json!({ "Ok": true })
    ));
    assert!(!replica_query::parse_ready_json_value(&json!(false)));
    assert!(!replica_query::parse_ready_json_value(
        &json!({ "Ok": false })
    ));
}

// Build representative subnet registry JSON.
fn registry_json() -> String {
    json!({
        "Ok": [
            {
                "pid": ROOT,
                "role": "root",
                "record": {
                    "pid": ROOT,
                    "role": "root",
                    "kind": "root",
                    "parent_pid": null,
                    "module_hash": HASH
                }
            },
            {
                "pid": APP,
                "role": "app",
                "record": {
                    "pid": APP,
                    "role": "app",
                    "kind": "singleton",
                    "parent_pid": ROOT,
                    "module_hash": HASH
                }
            },
            {
                "pid": MINIMAL,
                "role": "minimal",
                "record": {
                    "pid": MINIMAL,
                    "role": "minimal",
                    "kind": "singleton",
                    "parent_pid": ROOT,
                    "module_hash": HASH
                }
            },
            {
                "pid": WORKER,
                "role": "worker",
                "record": {
                    "pid": WORKER,
                    "role": "worker",
                    "kind": "replica",
                    "parent_pid": [APP],
                    "module_hash": HASH
                }
            }
        ]
    })
    .to_string()
}

fn same_role_variant_registry_json() -> String {
    json!({
        "Ok": [
            {
                "pid": ROOT,
                "role": "root",
                "record": {
                    "pid": ROOT,
                    "role": "root",
                    "kind": "root",
                    "parent_pid": null,
                    "module_hash": HASH
                }
            },
            {
                "pid": APP,
                "role": "app",
                "record": {
                    "pid": APP,
                    "role": "app",
                    "kind": "singleton",
                    "parent_pid": ROOT,
                    "module_hash": HASH
                }
            },
            {
                "pid": APP_VARIANT,
                "role": "app",
                "record": {
                    "pid": APP_VARIANT,
                    "role": "app",
                    "kind": "singleton",
                    "parent_pid": ROOT,
                    "module_hash": VARIANT_HASH
                }
            }
        ]
    })
    .to_string()
}

fn readiness_map() -> BTreeMap<String, ReadyStatus> {
    BTreeMap::from([
        (ROOT.to_string(), ReadyStatus::Ready),
        (APP.to_string(), ReadyStatus::NotReady),
        (MINIMAL.to_string(), ReadyStatus::Ready),
        (WORKER.to_string(), ReadyStatus::Error),
    ])
}

fn module_hash_map() -> BTreeMap<String, String> {
    BTreeMap::from([
        (ROOT.to_string(), HASH.to_string()),
        (APP.to_string(), HASH.to_string()),
        (MINIMAL.to_string(), HASH.to_string()),
        (WORKER.to_string(), HASH.to_string()),
    ])
}
