// Category C - System-level artifact test (no embedded config).

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const CANIC_MANAGED_RUNTIME_CRATES: &[&str] =
    &["canic", "canic-core", "canic-control-plane", "canic-macros"];

#[test]
fn canic_runtime_memory_has_one_package_identity() {
    let mut command = Command::new(env!("CARGO"));
    command.current_dir(workspace_root()).args([
        "tree",
        "--locked",
        "--offline",
        "--target",
        "wasm32-unknown-unknown",
        "--edges",
        "normal",
        "--all-features",
        "--prefix",
        "none",
        "--format",
        "{p}",
    ]);
    for package in CANIC_MANAGED_RUNTIME_CRATES {
        command.args(["-p", package]);
    }
    let output = command
        .output()
        .expect("resolve Canic runtime dependency graph");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8(output.stdout).expect("Cargo package identities are UTF-8");
    let memories = memory_package_identities(&tree);
    assert_eq!(
        memories.len(),
        1,
        "Canic's deployed runtime must share one ic-memory package identity: {memories:?}"
    );
}

fn memory_package_identities(tree: &str) -> BTreeSet<&str> {
    tree.lines()
        .filter(|line| line.starts_with("ic-memory "))
        .map(|line| line.strip_suffix(" (*)").unwrap_or(line))
        .collect()
}

#[test]
fn runtime_memory_identity_collection_preserves_distinct_packages() {
    assert_eq!(
        memory_package_identities("ic-memory v0.14.1\nic-memory v0.14.1 (*)"),
        BTreeSet::from(["ic-memory v0.14.1"])
    );
    assert_eq!(
        memory_package_identities("ic-memory v0.13.3\nic-memory v0.14.1\nic-memory-extra v1.0.0"),
        BTreeSet::from(["ic-memory v0.13.3", "ic-memory v0.14.1"])
    );
    assert!(memory_package_identities("canic-core v0.110.21").is_empty());
}

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
        if matches!(name.as_str(), "MEMORY_MANAGER" | "RestrictedMemory") {
            return true;
        }
        let next = tokens.get(index + 1);
        if matches!(
            name.as_str(),
            "stable_read" | "stable_write" | "stable_grow" | "stable_size"
        ) && matches!(next, Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis)
        {
            return true;
        }
        if name == "ic_memory"
            && matches!(next, Some(TokenTree::Punct(mark)) if mark.as_char() == '!')
        {
            return true;
        }
        if matches!(tokens.get(index + 1), Some(TokenTree::Punct(mark)) if mark.as_char() == ':')
            && matches!(tokens.get(index + 2), Some(TokenTree::Punct(mark)) if mark.as_char() == ':')
            && let Some(TokenTree::Ident(method)) = tokens.get(index + 3)
        {
            let method = method.to_string();
            if (name == "MemoryApi" && matches!(method.as_str(), "register" | "register_with_key"))
                || (name == "MemoryManager" && (method == "init" || method.starts_with("init_")))
            {
                return true;
            }
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
        "let count = ic_memory::MEMORY_MANAGER_INVALID_ID;"
    ));
    assert!(!has_forbidden_memory_pattern(
        r#"let example = "stable_grow(1)"; // MEMORY_MANAGER"#
    ));
    assert!(has_forbidden_memory_pattern(
        "MemoryManager :: init_with_bucket_size(memory, 1)"
    ));
    assert!(has_forbidden_memory_pattern("ic_memory ! (slot)"));
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
