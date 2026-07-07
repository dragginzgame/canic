// Category C - System-level artifact test (no embedded config).

use std::{
    fs,
    path::{Path, PathBuf},
};

#[test]
fn workflow_modules_do_not_use_or_define_workflow_preludes() {
    let workflow_root = source_root().join("workflow");
    let mut violations = Vec::new();

    scan_rust_files(&workflow_root, &mut |path, contents| {
        for (line_number, line) in code_lines(contents) {
            for forbidden in FORBIDDEN_WORKFLOW_PRELUDE_FRAGMENTS {
                if line.contains(forbidden) {
                    violations.push(format!(
                        "{}:{line_number} uses forbidden workflow prelude fragment `{forbidden}`",
                        display(path)
                    ));
                }
            }
        }
    });

    assert!(
        violations.is_empty(),
        "workflow prelude boundary changed: {violations:?}"
    );
}

const FORBIDDEN_WORKFLOW_PRELUDE_FRAGMENTS: &[&str] =
    &["workflow::prelude", "prelude::*", "pub mod prelude"];

fn code_lines(contents: &str) -> impl Iterator<Item = (usize, &str)> {
    contents.lines().enumerate().filter_map(|(index, line)| {
        let trimmed = line.trim();
        (!trimmed.starts_with("//") && !trimmed.is_empty()).then_some((index + 1, trimmed))
    })
}

fn source_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
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

fn display(path: &Path) -> String {
    path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .unwrap_or(path)
        .display()
        .to_string()
}
