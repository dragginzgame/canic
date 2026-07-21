// Category C - System-level artifact test (no embedded config).

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[test]
fn timer_and_timed_wait_inventory_is_explicit() {
    let root = workspace_root();
    let mut scheduling = BTreeMap::new();
    let mut waits = BTreeMap::new();

    for source_root in ["crates", "canisters", "fleets"] {
        collect_rust_sources(&root.join(source_root), &root, &mut |path, source| {
            if excluded_test_source(path) {
                return;
            }

            let scheduling_count = [
                "TimerWorkflow::",
                "TimerApi::",
                "TimerOps::",
                "cdk_set_timer(",
                "cdk_set_timer_interval(",
                "cdk_clear_timer(",
            ]
            .into_iter()
            .map(|fragment| source.matches(fragment).count())
            .sum();
            if scheduling_count > 0 {
                scheduling.insert(path.to_string(), scheduling_count);
            }

            let wait_count = [
                "thread::sleep(",
                "recv_timeout(",
                "park_timeout(",
                "sleep_until(",
                "tokio::time::sleep(",
            ]
            .into_iter()
            .map(|fragment| source.matches(fragment).count())
            .sum();
            if wait_count > 0 {
                waits.insert(path.to_string(), wait_count);
            }
        });
    }

    assert_eq!(scheduling, expected_scheduling_inventory());
    assert_eq!(waits, expected_wait_inventory());
}

#[test]
fn direct_ic_timer_access_has_one_production_owner() {
    let root = workspace_root();
    let mut raw_crate_users = BTreeMap::new();
    let mut reexport_users = BTreeMap::new();

    for source_root in ["crates", "canisters", "fleets"] {
        collect_rust_sources(&root.join(source_root), &root, &mut |path, source| {
            if excluded_test_source(path) {
                return;
            }

            let raw_count = source.matches("ic_cdk_timers").count();
            if raw_count > 0 {
                raw_crate_users.insert(path.to_string(), raw_count);
            }

            let reexport_count = source.matches("cdk::timers").count();
            if reexport_count > 0 {
                reexport_users.insert(path.to_string(), reexport_count);
            }
        });
    }

    assert_eq!(
        raw_crate_users,
        BTreeMap::from([("crates/canic-core/src/ops/runtime/timer.rs".to_string(), 1)])
    );
    assert!(reexport_users.is_empty());
}

fn expected_scheduling_inventory() -> BTreeMap<String, usize> {
    BTreeMap::from([
        ("crates/canic/src/macros/start.rs".to_string(), 7),
        ("crates/canic/src/api/mod.rs".to_string(), 1),
        ("crates/canic/src/macros/timer.rs".to_string(), 2),
        (
            "crates/canic-control-plane/src/api/lifecycle.rs".to_string(),
            2,
        ),
        ("crates/canic-core/src/api/runtime/mod.rs".to_string(), 1),
        ("crates/canic-core/src/api/timer.rs".to_string(), 3),
        (
            "crates/canic-core/src/lifecycle/init/nonroot.rs".to_string(),
            1,
        ),
        (
            "crates/canic-core/src/lifecycle/upgrade/nonroot.rs".to_string(),
            1,
        ),
        ("crates/canic-core/src/ops/runtime/timer.rs".to_string(), 2),
        (
            "crates/canic-core/src/workflow/placement/acknowledgement.rs".to_string(),
            2,
        ),
        (
            "crates/canic-core/src/workflow/pool/scheduler.rs".to_string(),
            2,
        ),
        (
            "crates/canic-core/src/workflow/runtime/auth/renewal.rs".to_string(),
            2,
        ),
        (
            "crates/canic-core/src/workflow/runtime/cycles/mod.rs".to_string(),
            2,
        ),
        (
            "crates/canic-core/src/workflow/runtime/intent.rs".to_string(),
            2,
        ),
        (
            "crates/canic-core/src/workflow/runtime/log.rs".to_string(),
            1,
        ),
        (
            "crates/canic-core/src/workflow/runtime/timer/mod.rs".to_string(),
            3,
        ),
    ])
}

fn expected_wait_inventory() -> BTreeMap<String, usize> {
    BTreeMap::from([
        (
            "crates/canic-backup/src/persistence/command_lifetime_lock/mod.rs".to_string(),
            4,
        ),
        (
            "crates/canic-backup/src/persistence/journal_lock/mod.rs".to_string(),
            2,
        ),
        ("crates/canic-host/src/icp/command.rs".to_string(), 1),
        (
            "crates/canic-host/src/install_root/readiness/mod.rs".to_string(),
            1,
        ),
    ])
}

fn collect_rust_sources(directory: &Path, root: &Path, visit: &mut impl FnMut(&str, &str)) {
    let mut entries = fs::read_dir(directory)
        .unwrap_or_else(|err| panic!("read {}: {err}", directory.display()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|err| panic!("read entry below {}: {err}", directory.display()));
    entries.sort_by_key(std::fs::DirEntry::path);

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources(&path, root, visit);
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .unwrap_or_else(|err| panic!("relativize {}: {err}", path.display()))
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        visit(&relative, &source);
    }
}

fn excluded_test_source(path: &str) -> bool {
    path.contains("/tests/")
        || path.ends_with("/tests.rs")
        || path.ends_with("/test_support.rs")
        || path.starts_with("canisters/test/")
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(PathBuf::from)
        .expect("workspace root")
}
