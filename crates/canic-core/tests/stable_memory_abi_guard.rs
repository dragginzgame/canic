// Category C - System-level artifact test (no embedded config).

mod source_scan;

use std::path::{Path, PathBuf};

const CANIC_MANAGED_RUNTIME_CRATES: &[&str] =
    &["canic", "canic-core", "canic-control-plane", "canic-macros"];

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
    source_scan::visit(
        root,
        &is_managed_memory_runtime_boundary,
        &mut |path, contents| {
            if !contents.starts_with("#![cfg(test)]") && has_forbidden_memory_pattern(contents) {
                violations.push(path.to_owned());
            }
        },
    );
}

fn has_forbidden_memory_pattern(contents: &str) -> bool {
    let tokens = contents
        .parse::<proc_macro2::TokenStream>()
        .expect("valid Rust tokens");
    forbidden_tokens(tokens)
}

fn forbidden_tokens(tokens: proc_macro2::TokenStream) -> bool {
    use proc_macro2::{Delimiter, TokenTree};
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    for (index, token) in tokens.iter().enumerate() {
        if let TokenTree::Group(group) = token {
            if forbidden_tokens(group.stream()) {
                return true;
            }
            continue;
        }
        let TokenTree::Ident(identifier) = token else {
            continue;
        };
        let name = identifier.to_string();
        let next = tokens.get(index + 1);
        if matches!(
            name.as_str(),
            "stable_read" | "stable_write" | "stable_grow" | "stable_size"
        ) && matches!(next, Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis)
        {
            return true;
        }
    }
    false
}

#[test]
fn managed_memory_guard_matches_calls_without_rejecting_observation_names() {
    assert!(has_forbidden_memory_pattern("let pages = stable_grow(1);"));
    assert!(has_forbidden_memory_pattern(
        "let pages = ic_cdk::api::stable::stable_size();"
    ));
    assert!(!has_forbidden_memory_pattern(
        "let pages = memory.maximum_stable_growth_pages();"
    ));
    assert!(!has_forbidden_memory_pattern(
        "let id = allocation.allocation_id;"
    ));
    assert!(!has_forbidden_memory_pattern(
        r#"let example = "stable_grow(1)"; // MEMORY_MANAGER"#
    ));
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
