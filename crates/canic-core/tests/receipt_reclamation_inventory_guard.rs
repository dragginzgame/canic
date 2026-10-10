// Category C - System-level artifact test (no embedded config).

use canic_core::role_contract::allocation::memory::{application_receipt, intent, placement};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use syn::visit::Visit;

#[test]
fn receipt_backed_authority_respects_layer_boundaries() {
    let root = workspace_root();
    // Public facade consumers and tests may grow without changing storage ownership.
    for (symbol, owners) in [
        ("ReceiptBackedIntentWorkflow", &["api/", "workflow/"][..]),
        ("ReceiptBackedIntentOps", &["api/", "workflow/", "ops/"][..]),
        (
            "ReceiptBackedIntentStore",
            &["ops/storage/", "storage/"][..],
        ),
    ] {
        let callers = source_paths_using(&root, symbol);
        assert!(
            !callers.is_empty(),
            "missing production callers for {symbol}"
        );
        let violations: Vec<_> = callers
            .iter()
            .filter(|path| !belongs_to_owner(path, owners))
            .collect();
        assert!(
            violations.is_empty(),
            "{symbol} crosses its layer boundary: {violations:?}"
        );
    }
}

fn belongs_to_owner(path: &str, owners: &[&str]) -> bool {
    path.strip_prefix("crates/canic-core/src/")
        .is_some_and(|relative| owners.iter().any(|owner| relative.starts_with(owner)))
}

#[test]
fn ownership_allows_module_moves_but_rejects_storage_bypasses() {
    let owners = ["ops/storage/", "storage/"];
    for path in [
        "crates/canic-core/src/ops/storage/intent/mod.rs",
        "crates/canic-core/src/ops/storage/intent/new_reader/mod.rs",
        "crates/canic-core/src/storage/stable/intent.rs",
    ] {
        assert!(belongs_to_owner(path, &owners));
    }
    for path in [
        "crates/canic-core/src/workflow/intent/mod.rs",
        "crates/canic-core/src/policy/intent/mod.rs",
        "crates/canic-core/src/ops/storage_other/mod.rs",
        "canisters/example/src/ops/storage/mod.rs",
    ] {
        assert!(!belongs_to_owner(path, &owners));
    }
}

#[test]
fn ownership_scan_excludes_inline_tests_but_keeps_production_calls() {
    for (source, expected) in [
        (
            "#[cfg(test)] mod tests { fn check() { ReceiptBackedIntentStore::load(); } }",
            false,
        ),
        (
            "#[test] fn check() { ReceiptBackedIntentStore::load(); }",
            false,
        ),
        (
            "mod production { fn read() { ReceiptBackedIntentStore::load(); } }",
            true,
        ),
    ] {
        let syntax = syn::parse_file(source).unwrap();
        let mut visitor = SymbolUseVisitor::new("ReceiptBackedIntentStore");
        visitor.visit_file(&syntax);
        assert_eq!(visitor.found, expected);
    }
}

#[test]
fn receipt_backed_stable_allocations_remain_single_owner() {
    let root = workspace_root();
    let storage = read(&root.join("crates/canic-core/src/storage/stable/intent.rs"));

    assert_eq!(
        storage
            .matches("canic.core.intent.receipt_backed_records.v1")
            .count(),
        1,
        "receipt-backed primary stable authority must have one allocation",
    );
    assert_eq!(
        storage
            .matches("canic.core.placement.acknowledgement_index.v1")
            .count(),
        1,
        "placement acknowledgement must retain one separate derived index",
    );
    assert_eq!(
        storage
            .matches("canic.core.application_receipt.eligibility.v1")
            .count(),
        1,
        "application terminal eligibility must have one ordered allocation",
    );
    assert_eq!(
        intent::INTENT_RECEIPT_BACKED_RECORDS_KEY,
        "canic.core.intent.receipt_backed_records.v1"
    );
    assert_eq!(
        placement::PLACEMENT_ACKNOWLEDGEMENT_INDEX_KEY,
        "canic.core.placement.acknowledgement_index.v1"
    );
    assert_eq!(
        application_receipt::APPLICATION_RECEIPT_ELIGIBILITY_KEY,
        "canic.core.application_receipt.eligibility.v1"
    );
}

fn source_paths_using(root: &Path, symbol: &str) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for source_root in ["crates", "canisters", "apps"] {
        collect_rust_sources(&root.join(source_root), root, &mut |path, source| {
            if path
                .split('/')
                .any(|component| matches!(component, "tests" | "tests.rs"))
            {
                return;
            }
            let syntax = syn::parse_file(source)
                .unwrap_or_else(|err| panic!("parse Rust source {path}: {err}"));
            let mut visitor = SymbolUseVisitor::new(symbol);
            visitor.visit_file(&syntax);
            if visitor.found {
                paths.insert(path.to_string());
            }
        });
    }
    paths
}

struct SymbolUseVisitor<'a> {
    symbol: &'a str,
    found: bool,
}

impl<'a> SymbolUseVisitor<'a> {
    const fn new(symbol: &'a str) -> Self {
        Self {
            symbol,
            found: false,
        }
    }
}

fn is_test_only(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("test")
            || (attribute.path().is_ident("cfg")
                && attribute
                    .parse_args::<syn::Path>()
                    .is_ok_and(|path| path.is_ident("test")))
    })
}

impl<'ast> Visit<'ast> for SymbolUseVisitor<'_> {
    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        if !is_test_only(&module.attrs) {
            syn::visit::visit_item_mod(self, module);
        }
    }

    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        if !is_test_only(&function.attrs) {
            syn::visit::visit_item_fn(self, function);
        }
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        if path
            .segments
            .iter()
            .enumerate()
            .any(|(index, segment)| segment.ident == self.symbol && index + 1 < path.segments.len())
        {
            self.found = true;
        }
        syn::visit::visit_path(self, path);
    }
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
        let source = read(&path);
        visit(&relative, &source);
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(PathBuf::from)
        .expect("workspace root")
}
