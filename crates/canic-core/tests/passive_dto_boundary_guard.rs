// Category C - System-level artifact test (no embedded config).

use std::{
    fs,
    path::{Path, PathBuf},
};

#[test]
fn non_error_dtos_do_not_import_internal_behavior_layers() {
    let mut violations = Vec::new();

    for dto_root in dto_roots() {
        scan_rust_files(&dto_root, &mut |path, contents| {
            if is_public_error_boundary_adapter(path) {
                return;
            }

            for (line_number, line) in code_lines(contents) {
                for forbidden in FORBIDDEN_LAYER_FRAGMENTS {
                    if line.contains(forbidden) {
                        violations.push(format!(
                            "{}:{line_number} imports forbidden DTO boundary fragment `{forbidden}`",
                            display(path)
                        ));
                    }
                }
            }
        });
    }

    assert!(
        violations.is_empty(),
        "passive DTO layer boundary changed: {violations:?}"
    );
}

#[test]
fn non_error_dtos_do_not_perform_side_effects() {
    let mut violations = Vec::new();

    for dto_root in dto_roots() {
        scan_rust_files(&dto_root, &mut |path, contents| {
            if is_public_error_boundary_adapter(path) {
                return;
            }

            for (line_number, line) in code_lines(contents) {
                for forbidden in FORBIDDEN_EFFECT_FRAGMENTS {
                    if line.contains(forbidden) {
                        violations.push(format!(
                            "{}:{line_number} uses forbidden DTO side-effect fragment `{forbidden}`",
                            display(path)
                        ));
                    }
                }
            }
        });
    }

    assert!(
        violations.is_empty(),
        "passive DTO side-effect boundary changed: {violations:?}"
    );
}

const FORBIDDEN_LAYER_FRAGMENTS: &[&str] = &[
    "crate::ops",
    "crate::workflow",
    "crate::storage",
    "crate::runtime",
    "crate::policy",
    "crate::access",
    "crate::InternalError",
    "crate::{InternalError",
    "InternalErrorOrigin",
];

const FORBIDDEN_EFFECT_FRAGMENTS: &[&str] = &[
    "async fn",
    ".await",
    "spawn(",
    "set_timer",
    "clear_timer",
    "TimerOps",
    "IcOps",
    "MgmtOps",
    "CallOps",
    "ic_cdk::",
];

fn dto_roots() -> Vec<PathBuf> {
    let workspace = workspace_root();
    vec![
        workspace.join("crates/canic-core/src/dto"),
        workspace.join("crates/canic-control-plane/src/dto"),
    ]
}

fn is_public_error_boundary_adapter(path: &Path) -> bool {
    path.to_string_lossy()
        .ends_with("/crates/canic-core/src/dto/error.rs")
}

fn code_lines(contents: &str) -> impl Iterator<Item = (usize, &str)> {
    contents.lines().enumerate().filter_map(|(index, line)| {
        let trimmed = line.trim();
        (!trimmed.starts_with("//") && !trimmed.is_empty()).then_some((index + 1, trimmed))
    })
}

fn scan_rust_files(root: &Path, visitor: &mut impl FnMut(&Path, &str)) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_rust_files(&path, visitor);
            continue;
        }

        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }

        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        visitor(&path, &contents);
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}

fn display(path: &Path) -> String {
    path.strip_prefix(workspace_root())
        .unwrap_or(path)
        .display()
        .to_string()
}
