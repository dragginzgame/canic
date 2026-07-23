use super::*;
use candid::{Encode, Principal};
use canic_core::{
    cdk::utils::hash::{decode_hex, hex_bytes},
    dto::{
        canister::CanisterInfo,
        error::Error as CanicError,
        topology::{SubnetRegistryEntry, SubnetRegistryResponse},
    },
    ids::CanisterRole,
};
use canic_host::table::{ColumnAlign, render_separator, render_table_row};
use canic_host::{
    registry::parse_registry_entries,
    release_set::{AppConfigDeclaration, AppConfigError},
};
use options::ListSource;
use render::ReadyStatus;
use serde_json::json;
use std::{collections::BTreeMap, path::PathBuf};

const ROOT: &str = "aaaaa-aa";
const APP: &str = "renrk-eyaaa-aaaaa-aaada-cai";
const APP_VARIANT: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";
const MINIMAL: &str = "rrkah-fqaaa-aaaaa-aaaaq-cai";
const WORKER: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const HASH_PREFIX: &str = "01234567";
const VARIANT_HASH: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
const VARIANT_HASH_PREFIX: &str = "abcdef01";
const TEST_REGISTRY_ALIGNMENTS: [ColumnAlign; 7] = [
    ColumnAlign::Left,
    ColumnAlign::Left,
    ColumnAlign::Left,
    ColumnAlign::Left,
    ColumnAlign::Left,
    ColumnAlign::Right,
    ColumnAlign::Right,
];

fn render_registry_table_row(row: &[impl AsRef<str>], widths: &[usize; 7]) -> String {
    render_table_row(row, widths, &TEST_REGISTRY_ALIGNMENTS)
}

fn render_registry_separator(widths: &[usize; 7]) -> String {
    render_separator(widths)
}

// Ensure list options parse live registry queries.
#[test]
fn parses_live_list_options() {
    let options = ListOptions::parse_info_list([
        OsString::from("demo"),
        OsString::from("--subtree"),
        OsString::from(APP),
        OsString::from(crate::cli::globals::INTERNAL_ENVIRONMENT_OPTION),
        OsString::from("local"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/bin/icp"),
    ])
    .expect("parse list options");

    assert_eq!(options.source, ListSource::RootRegistry);
    assert_eq!(options.target, "demo");
    assert_eq!(options.subtree, Some(APP.to_string()));
    assert_eq!(options.environment, Some("local".to_string()));
    assert_eq!(options.icp, "/bin/icp");
    assert!(!options.verbose);
}

#[test]
fn missing_list_deployment_preserves_canonical_typed_error() {
    let error = ListCommandError::from(InstalledDeploymentError::NoInstalledDeployment {
        environment: "local".to_string(),
        deployment: "demo-local".to_string(),
    });
    let message = error.to_string();

    assert_eq!(
        message,
        "deployment target demo-local is not installed on environment local"
    );
    std::assert_matches!(
        error,
        ListCommandError::InstalledDeployment(
            InstalledDeploymentError::NoInstalledDeployment { .. }
        )
    );
}

#[test]
fn list_preserves_icp_root_resolution_causes() {
    let error = ListCommandError::from(IcpConfigError::NoIcpRoot {
        start: PathBuf::from("/project"),
    });

    std::assert_matches!(
        error,
        ListCommandError::IcpRoot(IcpConfigError::NoIcpRoot { .. })
    );
}

#[test]
fn list_preserves_app_config_causes() {
    let error = ListCommandError::from(AppConfigError::DeclarationMissing {
        declaration: AppConfigDeclaration::AppName,
    });

    std::assert_matches!(
        error,
        ListCommandError::AppConfig(AppConfigError::DeclarationMissing {
            declaration: AppConfigDeclaration::AppName
        })
    );
}

// Ensure config options parse declared App inspection.
#[test]
fn parses_config_options() {
    let options = ListOptions::parse_config([
        OsString::from("demo"),
        OsString::from(crate::cli::globals::INTERNAL_ENVIRONMENT_OPTION),
        OsString::from("local"),
        OsString::from("-v"),
    ])
    .expect("parse config options");

    assert_eq!(options.source, ListSource::Config);
    assert_eq!(options.target, "demo");
    assert_eq!(options.subtree, None);
    assert_eq!(options.environment, Some("local".to_string()));
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

    std::assert_matches!(err, ListCommandError::Usage(_));
}

// Ensure list and config help keep deployment-target and App selection separate.
#[test]
fn list_and_config_usage_explain_app_and_subtree_options() {
    let list = info_usage();
    let config = config_usage();

    assert!(list.contains("List canisters registered by an installed deployment root"));
    assert!(list.contains("Usage: canic info list [OPTIONS] <deployment>"));
    assert!(list.contains("<deployment>"));
    assert!(list.contains("Installed deployment target name to inspect"));
    assert!(list.contains("--subtree <name-or-principal>"));
    assert!(list.contains("--verbose"));
    assert!(config.contains("Usage: canic app config [OPTIONS] <app>"));
    assert!(config.contains("<app>"));
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
                    "ROLE",
                    "MODULE",
                    "CANISTER_ID",
                    "READY",
                    "CANIC",
                    "WASM",
                    "CYCLES",
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

    assert!(tree.contains("MODULE_HASH"));
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
                    "ROLE",
                    "MODULE",
                    "CANISTER_ID",
                    "READY",
                    "CANIC",
                    "WASM",
                    "CYCLES",
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

// Ensure selected subtrees can be anchored by a unique role name.
#[test]
fn renders_selected_subtree_by_role_name() {
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
    let tree = render_registry_tree(&registry, Some("app"), &columns).expect("render subtree");
    let widths = [9, 8, 27, 5, 5, 4, 6];

    assert_eq!(
        tree,
        [
            render_registry_table_row(
                &[
                    "ROLE",
                    "MODULE",
                    "CANISTER_ID",
                    "READY",
                    "CANIC",
                    "WASM",
                    "CYCLES",
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

// Ensure repeated role names require a concrete principal.
#[test]
fn selected_subtree_rejects_ambiguous_role_name() {
    let registry = parse_registry_entries(&same_role_variant_registry_json())
        .expect("parse same-role registry");
    let readiness = BTreeMap::new();
    let empty = BTreeMap::new();
    let columns = RegistryColumnData {
        readiness: &readiness,
        canic_versions: &empty,
        module_hashes: &empty,
        wasm_sizes: &empty,
        cycles: &empty,
        full_module_hashes: false,
        color_module_variants: false,
    };
    let err = render_registry_tree(&registry, Some("app"), &columns)
        .expect_err("repeated role should be ambiguous");

    std::assert_matches!(
        err,
        ListCommandError::RegistryTree(
            crate::support::registry_tree::RegistryTreeError::AmbiguousRole { role, .. }
        ) if role == "app"
    );
}

// Ensure the full list output names the selected deployment before the tree table.
#[test]
fn renders_list_output_with_deployment_title() {
    let registry = parse_registry_entries(&registry_json()).expect("parse registry");
    let title = ListTitle {
        source: ListTitleSource::Deployment,
        name: "demo".to_string(),
        environment: "local".to_string(),
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

    assert!(output.starts_with("Deployment: demo (environment local)\n\nROLE"));
    assert!(output.contains("CANISTER_ID"));
}

#[test]
fn renders_list_output_with_wasm_size_and_missing_roles() {
    let registry = parse_registry_entries(&registry_json()).expect("parse registry");
    let title = ListTitle {
        source: ListTitleSource::Deployment,
        name: "demo".to_string(),
        environment: "local".to_string(),
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

// Ensure config-only Apps render their declared roles instead of deployed inventory.
#[test]
fn renders_config_output_with_app_roles() {
    let title = ListTitle {
        source: ListTitleSource::App,
        name: "test_me".to_string(),
        environment: "local".to_string(),
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
                "services.fleet".to_string(),
                "metrics profile=hub tiers=core,placement,runtime,security (inferred)".to_string(),
                "sharding user_shards->user_shard cap=100 initial=1 max=4".to_string(),
            ],
        },
    ];
    let output = render_config_output(&title, &rows, true);

    assert_eq!(
        output,
        [
            "App: test_me (environment local)",
            "",
            "ROLE   KIND        AUTO   CAPS             METRICS   TOPUP",
            "----   ---------   ----   --------------   -------   ------------------",
            "root   root        -      -                root      -",
            "app    singleton   yes    auth, sharding   hub       4.00 TC @ 10.00 TC",
            "  - services.fleet",
            "  - metrics profile=hub tiers=core,placement,runtime,security (inferred)",
            "  - sharding user_shards->user_shard cap=100 initial=1 max=4",
        ]
        .join("\n")
    );
}

fn registry_json() -> String {
    registry_response_json(vec![
        registry_entry(ROOT, "root", None, HASH),
        registry_entry(APP, "app", Some(ROOT), HASH),
        registry_entry(MINIMAL, "minimal", Some(ROOT), HASH),
        registry_entry(WORKER, "worker", Some(APP), HASH),
    ])
}

fn same_role_variant_registry_json() -> String {
    registry_response_json(vec![
        registry_entry(ROOT, "root", None, HASH),
        registry_entry(APP, "app", Some(ROOT), HASH),
        registry_entry(APP_VARIANT, "app", Some(ROOT), VARIANT_HASH),
    ])
}

fn registry_entry(
    pid: &str,
    role: &str,
    parent_pid: Option<&str>,
    module_hash: &str,
) -> SubnetRegistryEntry {
    let pid = Principal::from_text(pid).expect("registry principal");
    let role = CanisterRole::owned(role.to_string());
    SubnetRegistryEntry {
        pid,
        role: role.clone(),
        record: CanisterInfo {
            pid,
            role,
            parent_pid: parent_pid
                .map(|parent| Principal::from_text(parent).expect("registry parent principal")),
            module_hash: Some(decode_hex(module_hash).expect("registry module hash")),
            created_at: 1,
        },
    }
}

fn registry_response_json(entries: Vec<SubnetRegistryEntry>) -> String {
    let response = Ok::<_, CanicError>(SubnetRegistryResponse(entries));
    let bytes = Encode!(&response).expect("encode registry response");
    json!({ "response_bytes": hex_bytes(bytes) }).to_string()
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
