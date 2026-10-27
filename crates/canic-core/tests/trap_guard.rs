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

    scan_dir(
        &workspace_root.join("crates"),
        &lifecycle,
        &ic_ops,
        &mut violations,
    );

    assert!(
        violations.is_empty(),
        "ic_platform::cdk::api::trap usage outside lifecycle and the approved IC ops facade: {violations:?}"
    );
}

fn scan_dir(root: &Path, lifecycle: &Path, ic_ops: &Path, violations: &mut Vec<PathBuf>) {
    source_scan::visit(
        root,
        &|path| {
            path.file_name()
                .is_some_and(|name| name == "target" || name == ".git" || name == "tests")
                || path.starts_with(lifecycle)
                || path == ic_ops
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
