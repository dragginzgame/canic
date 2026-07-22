// Category C - System-level artifact test (no embedded config).

use std::{
    fs,
    path::{Path, PathBuf},
};

const CANIC_MANAGED_RUNTIME_CRATES: &[&str] = &[
    "canic",
    "canic-core",
    "canic-control-plane",
    "canic-macros",
    "canic-wasm-store",
];

#[test]
fn canic_managed_runtime_code_uses_managed_explicit_stable_keys() {
    let workspace_root = workspace_root();
    let mut violations = Vec::new();

    for crate_name in CANIC_MANAGED_RUNTIME_CRATES {
        scan_dir(
            &workspace_root.join("crates").join(crate_name).join("src"),
            &mut violations,
        );
    }

    assert!(
        violations.is_empty(),
        "Canic-managed runtime code must not bypass the managed explicit-key ABI: {violations:?}"
    );
}

fn scan_dir(root: &Path, violations: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir(&path, violations);
            continue;
        }

        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }

        if is_managed_memory_runtime_boundary(&path) {
            continue;
        }

        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };

        if contents.starts_with("#![cfg(test)]") {
            continue;
        }

        if has_forbidden_memory_pattern(&contents) {
            violations.push(path);
        }
    }
}

fn has_forbidden_memory_pattern(contents: &str) -> bool {
    const FORBIDDEN: &[&str] = &[
        "ic_memory!(",
        "MemoryApi::register(",
        "MemoryApi::register_with_key(",
        "MEMORY_MANAGER",
        "MemoryManager::init",
        "RestrictedMemory",
        "stable_read",
        "stable_write",
        "stable_grow",
        "stable_size",
    ];

    FORBIDDEN.iter().any(|pattern| contents.contains(pattern))
}

fn is_managed_memory_runtime_boundary(path: &Path) -> bool {
    path.to_string_lossy()
        .contains("/crates/canic-core/src/memory/")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}
