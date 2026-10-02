//! Check maintained product generations through Rust syntax, excluding test and documentation data.

use std::{
    fs,
    path::{Path, PathBuf},
};
use syn::{
    Attribute, Expr, Item, Lit, Member,
    visit::{self, Visit},
};

#[derive(Default)]
struct ProductGenerations {
    violations: Vec<String>,
}

fn generation_field(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    let parts = upper.split('_').collect::<Vec<_>>();
    parts.contains(&"VERSION")
        && parts.iter().any(|part| {
            matches!(
                *part,
                "SCHEMA"
                    | "PROTOCOL"
                    | "MANIFEST"
                    | "FORMAT"
                    | "WIRE"
                    | "CONFIG"
                    | "JOURNAL"
                    | "PLAN"
                    | "STATE"
                    | "LAYOUT"
                    | "DATA"
                    | "SNAPSHOT"
                    | "RECORD"
                    | "RESPONSE"
                    | "POLICY"
            )
        })
}

fn literal_generation(expr: &Expr) -> Option<u64> {
    match expr {
        Expr::Lit(expr) => match &expr.lit {
            Lit::Int(value) => value.base10_parse().ok(),
            _ => None,
        },
        Expr::Paren(expr) => literal_generation(&expr.expr),
        _ => None,
    }
}

fn test_only(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("test")
            || (attr.path().is_ident("cfg")
                && attr
                    .parse_args::<syn::Path>()
                    .is_ok_and(|path| path.is_ident("test")))
    })
}

fn versioned_domain(value: &str) -> bool {
    let Some(rest) = value
        .strip_prefix("canic")
        .filter(|rest| rest.starts_with([':', '/', '.']))
    else {
        return false;
    };
    rest.split([':', '/', '.']).any(|part| {
        part.strip_prefix('v')
            .and_then(|version| version.trim_end_matches('\0').parse::<u64>().ok())
            .is_some_and(|version| version > 1)
    })
}

impl<'ast> Visit<'ast> for ProductGenerations {
    // Rustdoc and comments are explanatory evidence, never product declarations.
    fn visit_attribute(&mut self, _attribute: &'ast Attribute) {}

    fn visit_item(&mut self, item: &'ast Item) {
        let attrs = match item {
            Item::Fn(item) => &item.attrs,
            Item::Mod(item) => &item.attrs,
            Item::Const(item) => &item.attrs,
            Item::Static(item) => &item.attrs,
            Item::Struct(item) => &item.attrs,
            Item::Enum(item) => &item.attrs,
            Item::Type(item) => &item.attrs,
            _ => {
                visit::visit_item(self, item);
                return;
            }
        };
        if !test_only(attrs) {
            visit::visit_item(self, item);
        }
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        if generation_field(&item.ident.to_string())
            && literal_generation(&item.expr).is_some_and(|version| version > 1)
        {
            self.violations.push(item.ident.to_string());
        }
        visit::visit_item_const(self, item);
    }

    fn visit_expr_struct(&mut self, expr: &'ast syn::ExprStruct) {
        for field in &expr.fields {
            if let Member::Named(name) = &field.member
                && generation_field(&name.to_string())
                && literal_generation(&field.expr).is_some_and(|version| version > 1)
            {
                self.violations.push(name.to_string());
            }
        }
        visit::visit_expr_struct(self, expr);
    }

    fn visit_lit(&mut self, literal: &'ast Lit) {
        let value = match literal {
            Lit::Str(value) => Some(value.value()),
            Lit::ByteStr(value) => String::from_utf8(value.value()).ok(),
            _ => None,
        };
        if let Some(value) = value
            && versioned_domain(&value)
        {
            self.violations.push(value);
        }
    }
}

fn inspect(source: &str) -> Vec<String> {
    let syntax = syn::parse_file(source).expect("maintained Rust source must parse");
    let mut generations = ProductGenerations::default();
    generations.visit_file(&syntax);
    generations.violations
}

fn collect_sources(directory: &Path, sources: &mut Vec<PathBuf>) {
    if !directory.exists() {
        return;
    }
    for entry in fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if entry.file_type().unwrap().is_dir() {
            if !matches!(
                entry.file_name().to_str(),
                Some(
                    "tests"
                        | "testing"
                        | "test_support"
                        | "fixture"
                        | "fixtures"
                        | ".canic"
                        | "target"
                )
            ) {
                collect_sources(&path, sources);
            }
        } else if path.extension().is_some_and(|extension| extension == "rs")
            && path.file_name().is_none_or(|name| name != "tests.rs")
        {
            sources.push(path);
        }
    }
}

#[test]
fn maintained_product_generations_remain_v1() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let mut sources = Vec::new();
    for package in [
        "canic",
        "canic-backup",
        "canic-cli",
        "canic-control-plane",
        "canic-core",
        "canic-host",
        "canic-macros",
    ] {
        collect_sources(&root.join("crates").join(package).join("src"), &mut sources);
    }
    collect_sources(&root.join("canisters"), &mut sources);
    collect_sources(&root.join("apps"), &mut sources);
    assert!(!sources.is_empty());
    let violations = sources
        .into_iter()
        .flat_map(|path| {
            inspect(&fs::read_to_string(&path).unwrap())
                .into_iter()
                .map(move |name| format!("{}: {name}", path.display()))
        })
        .collect::<Vec<_>>();
    assert!(
        violations.is_empty(),
        "maintained product generation violations: {violations:?}"
    );
}

#[test]
fn syntax_distinguishes_product_authority_from_documentation_and_negative_cases() {
    assert!(
        inspect(
            r"
        /// Unsupported input example: schema_version: 2
        const SCHEMA_VERSION: u32 = 1;
        /* const PROTOCOL_VERSION: u32 = 2; */
        const UPSTREAM_VERSION: u32 = 3;
        const AUDIT_METHOD_REVISION: u32 = 3;
        #[cfg(test)] mod tests {
            const SCHEMA_VERSION: u32 = 2;
        }
        #[test] fn rejects_unknown_generation() { let _ = Input { schema_version: 2 }; }
    "
        )
        .is_empty()
    );
    for source in [
        "const SCHEMA_VERSION: u32 = 2;",
        "const JOURNAL_VERSION: u32 = 10;",
        "fn input() { let _ = Input { schema_version: 2 }; }",
        "const DOMAIN: &[u8] = b\"canic:example:v2\";",
        "const DOMAIN: &str = \"canic/example/v10\";",
    ] {
        assert!(
            !inspect(source).is_empty(),
            "product authority was missed: {source}"
        );
    }
}
