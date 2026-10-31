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
        "CDK trap usage outside approved lifecycle, adapter and IC ops boundaries: {violations:?}"
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
            let tokens = contents
                .parse()
                .unwrap_or_else(|error| panic!("parse Rust tokens in {}: {error}", path.display()));
            if contains_cdk_trap(tokens) {
                violations.push(path.to_owned());
            }
        },
    );
}

fn contains_cdk_trap(tokens: proc_macro2::TokenStream) -> bool {
    use proc_macro2::TokenTree;

    let tokens: Vec<_> = tokens.into_iter().collect();
    for (index, token) in tokens.iter().enumerate() {
        if let TokenTree::Group(group) = token
            && contains_cdk_trap(group.stream())
        {
            return true;
        }
        let TokenTree::Ident(name) = token else {
            continue;
        };
        if name != "cdk" && name != "ic_cdk" {
            continue;
        }
        let mut tail = &tokens[index + 1..];
        if !take_path_separator(&mut tail) {
            continue;
        }
        if matches!(tail.first(), Some(TokenTree::Ident(name)) if name == "api") {
            tail = &tail[1..];
            if !take_path_separator(&mut tail) {
                continue;
            }
        }
        if matches!(tail.first(), Some(TokenTree::Ident(name)) if name == "trap") {
            return true;
        }
    }
    false
}

fn take_path_separator(tokens: &mut &[proc_macro2::TokenTree]) -> bool {
    use proc_macro2::TokenTree;

    if matches!(*tokens, [TokenTree::Punct(first), TokenTree::Punct(second), ..]
        if first.as_char() == ':' && second.as_char() == ':')
    {
        *tokens = &tokens[2..];
        true
    } else {
        false
    }
}

#[test]
fn detects_cdk_trap_paths_in_code_and_macro_bodies() {
    for source in [
        "fn f() { cdk::trap(\"failure\"); }",
        "fn f() { ic_cdk :: trap(\"failure\"); }",
        "fn f() { cdk::api::trap(\"failure\"); }",
        "fn f() { ic_cdk::api::trap(\"failure\"); }",
        "macro_rules! endpoint { () => { $crate::__internal::cdk :: trap(\"failure\") }; }",
        "install!(trap = ic_platform::cdk::api::trap);",
    ] {
        assert!(contains_cdk_trap(source.parse().unwrap()), "{source}");
    }
}

#[test]
fn ignores_comments_strings_and_unrelated_identifiers() {
    for source in [
        "// ic_cdk::trap(\"failure\")\nfn f() {}",
        "const TEXT: &str = \"cdk::api::trap\";",
        "fn f() { other_cdk::trap(); cdk::trap_count(); other::trap(); }",
    ] {
        assert!(!contains_cdk_trap(source.parse().unwrap()), "{source}");
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}
