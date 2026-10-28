// Category C - System-level artifact test (no embedded config).

mod source_scan;

use std::path::{Path, PathBuf};

#[test]
fn trap_usage_is_confined_to_approved_platform_boundaries() {
    let workspace_root = workspace_root();
    let lifecycle = workspace_root
        .join("crates")
        .join("canic-core")
        .join("src")
        .join("lifecycle");
    let ic_ops = workspace_root
        .join("crates")
        .join("canic-core")
        .join("src")
        .join("ops")
        .join("ic")
        .join("mod.rs");
    let mut violations = Vec::new();
    // Adapter-owned lifecycle rollback and the supplier's endpoint trap callback
    // are platform boundaries too; do not exempt either entire adapter crate.
    let adapter_boundaries = [
        workspace_root.join("crates/canic-icydb/src/lifecycle/mod.rs"),
        workspace_root.join("crates/canic-blob-service/src/endpoints/mod.rs"),
    ];

    scan_dir(
        &workspace_root.join("crates"),
        &lifecycle,
        &ic_ops,
        &adapter_boundaries,
        &mut violations,
    );

    assert!(
        violations.is_empty(),
        "ic_platform::cdk::api::trap usage outside approved lifecycle, adapter and IC ops boundaries: {violations:?}"
    );
}

fn scan_dir(
    root: &Path,
    lifecycle: &Path,
    ic_ops: &Path,
    adapter_boundaries: &[PathBuf],
    violations: &mut Vec<PathBuf>,
) {
    source_scan::visit(
        root,
        &|path| {
            path.file_name()
                .is_some_and(|name| name == "target" || name == ".git" || name == "tests")
                || path.starts_with(lifecycle)
                || path == ic_ops
                || adapter_boundaries.iter().any(|boundary| path == boundary)
        },
        &mut |path, contents| {
            if contents.contains("cdk::api::trap") {
                violations.push(path.to_owned());
            }
        },
    );
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}
