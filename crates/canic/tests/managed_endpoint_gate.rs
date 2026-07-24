use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const MANAGED_START_MARKERS: &[&str] = &[
    "canic::start!()",
    "canic::start!(",
    "canic::start_local!()",
    "canic::start_local!(",
    "canic::start_wasm_store!()",
    "canic::start_wasm_store!(",
];

const RAW_ENDPOINT_MARKERS: &[&str] = &[
    "#[ic_cdk::query",
    "#[ic_cdk::update",
    "#[::ic_cdk::query",
    "#[::ic_cdk::update",
    "#[query]",
    "#[query(",
    "#[update]",
    "#[update(",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory should have a parent")
        .parent()
        .expect("workspace root should exist")
        .to_path_buf()
}

fn collect_files(root: &Path, filename: Option<&str>, extension: Option<&str>) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()))
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|error| panic!("read entry under {}: {error}", directory.display()));
        entries.sort_by_key(std::fs::DirEntry::path);
        for entry in entries {
            let path = entry.path();
            let file_type = entry
                .file_type()
                .unwrap_or_else(|error| panic!("inspect {}: {error}", path.display()));
            if file_type.is_dir() {
                pending.push(path);
            } else if file_type.is_file()
                && filename.is_none_or(|expected| entry.file_name() == expected)
                && extension
                    .is_none_or(|expected| path.extension().is_some_and(|ext| ext == expected))
            {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

#[test]
fn managed_canisters_export_endpoints_only_through_canic_macros() {
    let workspace = workspace_root();
    let mut managed_sources = BTreeSet::new();

    for source_root in ["apps", "canisters"] {
        for manifest in collect_files(&workspace.join(source_root), Some("Cargo.toml"), None) {
            let package_root = manifest.parent().expect("package manifest parent");
            let sources = collect_files(&package_root.join("src"), None, Some("rs"));
            let managed = sources.iter().any(|path| {
                let source = fs::read_to_string(path)
                    .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
                MANAGED_START_MARKERS
                    .iter()
                    .any(|marker| source.contains(marker))
            });
            if managed {
                managed_sources.extend(sources);
            }
        }
    }

    let mut violations = Vec::new();
    for path in managed_sources {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        for marker in RAW_ENDPOINT_MARKERS {
            if source.contains(marker) {
                violations.push(format!(
                    "{} contains raw managed endpoint marker {marker}",
                    path.strip_prefix(&workspace).unwrap_or(&path).display()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "managed Canister endpoints bypass the Canic activation dispatcher: {violations:#?}"
    );
}

#[test]
fn prepared_managed_init_defers_application_work_while_standalone_local_starts_it() {
    let macro_path = workspace_root().join("crates/canic/src/macros/start.rs");
    let source = fs::read_to_string(&macro_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", macro_path.display()));
    let managed = source
        .split("macro_rules! __canic_start_nonroot_lifecycle_core")
        .nth(1)
        .and_then(|rest| {
            rest.split("macro_rules! __canic_start_local_lifecycle_core")
                .next()
        })
        .expect("managed non-root lifecycle macro");
    let managed_init = managed
        .split("#[$crate::__internal::cdk::init]")
        .nth(1)
        .and_then(|rest| {
            rest.split("#[$crate::__internal::cdk::post_upgrade]")
                .next()
        })
        .expect("managed non-root init body");

    assert!(
        managed_init.contains("LifecycleApi::init_nonroot_canister_before_bootstrap"),
        "managed non-root init must enter the canonical Prepared lifecycle"
    );
    assert!(
        !managed_init.contains("schedule_init_nonroot_bootstrap")
            && !managed_init.contains("TimerApi::defer_lifecycle")
            && !managed_init.contains("canic_install("),
        "Prepared managed init must not schedule bootstrap, timers, or application hooks"
    );

    let local = source
        .split("macro_rules! __canic_start_local_lifecycle_core")
        .nth(1)
        .and_then(|rest| {
            rest.split("macro_rules! __canic_root_lifecycle_core")
                .next()
        })
        .expect("standalone-local lifecycle macro");
    let local_init = local
        .split("#[$crate::__internal::cdk::init]")
        .nth(1)
        .and_then(|rest| {
            rest.split("#[$crate::__internal::cdk::post_upgrade]")
                .next()
        })
        .expect("standalone-local init body");

    assert!(
        local_init.contains("LifecycleApi::init_local_nonroot_canister_before_bootstrap")
            && local_init.contains("schedule_init_nonroot_bootstrap")
            && local_init.contains("canic_install(args)"),
        "standalone-local init must retain its explicit local lifecycle and application startup"
    );
    assert!(
        !local.contains("CanisterInitPayload") && !local.contains("FleetBinding"),
        "standalone-local lifecycle must not fabricate managed Fleet identity"
    );
}
