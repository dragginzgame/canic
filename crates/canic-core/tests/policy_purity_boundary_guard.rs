// Category C - System-level artifact test (no embedded config).

mod source_scan;

use std::path::{Path, PathBuf};

#[test]
fn pure_policy_modules_do_not_import_side_effect_boundaries() {
    let policy_root = source_root().join("domain/policy/pure");
    let mut violations = Vec::new();

    scan_rust_files(&policy_root, &mut |path, contents| {
        for (line_number, line) in code_lines(contents) {
            for forbidden in FORBIDDEN_IMPORT_FRAGMENTS {
                if line.contains(forbidden) {
                    violations.push(format!(
                        "{}:{line_number} imports forbidden policy boundary fragment `{forbidden}`",
                        display(path)
                    ));
                }
            }
        }
    });

    assert!(
        violations.is_empty(),
        "pure policy import boundary changed: {violations:?}"
    );
}

#[test]
fn pure_policy_modules_do_not_perform_side_effects_or_wire_serialization() {
    let policy_root = source_root().join("domain/policy/pure");
    let mut violations = Vec::new();

    scan_rust_files(&policy_root, &mut |path, contents| {
        for (line_number, line) in code_lines(contents) {
            for forbidden in FORBIDDEN_EFFECT_FRAGMENTS {
                if line.contains(forbidden) {
                    violations.push(format!(
                        "{}:{line_number} uses forbidden policy side-effect fragment `{forbidden}`",
                        display(path)
                    ));
                }
            }
        }
    });

    assert!(
        violations.is_empty(),
        "pure policy side-effect boundary changed: {violations:?}"
    );
}

const FORBIDDEN_IMPORT_FRAGMENTS: &[&str] = &[
    "crate::storage",
    "crate::ops",
    "crate::workflow",
    "crate::dto",
    "crate::cdk",
    "crate::runtime",
    "ic_cdk",
    "ic_platform::cdk",
    "ic_cdk_timers",
    "serde_json",
    "serde_cbor",
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
    "CandidType",
    "Serialize",
    "Deserialize",
];

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
    source_scan::visit(root, &|_| false, visitor);
}

fn display(path: &Path) -> String {
    path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .unwrap_or(path)
        .display()
        .to_string()
}
