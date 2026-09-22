//! Module: managed_endpoint_gate_support
//!
//! Responsibility: qualify exact raw fixture instrumentation outside readiness dispatch.
//! Does not own: application endpoints, authorization implementations or runtime behavior.
//! Boundary: exact unpublished probes exercise readiness, codecs and bare-CDK ingress limits.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use syn::{Attribute, Expr, ItemFn, Lit, Meta, Token, punctuated::Punctuated, visit::Visit};

const PROBE: &str = "canisters/test/canic_icydb_lifecycle_probe/src";
const CONTROLLER: &str = "crate::require_test_controller";

/// Exact test endpoint admission, including the complete allowed CDK options.
struct ReviewedEndpoint {
    path: String,
    name: &'static str,
    kind: &'static str,
    guard: Option<&'static str>,
}

fn reviewed_endpoints() -> Vec<ReviewedEndpoint> {
    let mut reviewed = vec![
        ReviewedEndpoint {
            path: "apps/test/user_hub/src/fixture_importer/mod.rs".into(),
            name: "test_release_fixture",
            kind: "update",
            guard: Some("require_controller"),
        },
        // This standalone probe must use the bare CDK adapter to qualify the
        // inherited inspector limit independently of Canic's update macro.
        ReviewedEndpoint {
            path: "canisters/test/payload_limit_probe/src/lib.rs".into(),
            name: "bare_echo",
            kind: "update",
            guard: None,
        },
    ];
    for (file, name, kind, guard) in [
        (
            "lib.rs",
            "lifecycle_composition_snapshot",
            "query",
            Some(CONTROLLER),
        ),
        (
            "fixture_provisioning/mod.rs",
            "fixture_progress",
            "query",
            Some(CONTROLLER),
        ),
        (
            "fixture_provisioning/mod.rs",
            "fixture_first_row",
            "query",
            Some(CONTROLLER),
        ),
        (
            "fixture_provisioning/consumer/mod.rs",
            "fixture_consumer_status",
            "query",
            Some(CONTROLLER),
        ),
        (
            "fixture_provisioning/consumer/mod.rs",
            "fixture_consumer_fault",
            "update",
            Some(CONTROLLER),
        ),
        (
            "fixture_provisioning/consumer/mod.rs",
            "fixture_consumer_fetch_pending",
            "query",
            Some(CONTROLLER),
        ),
        (
            "fixture_provisioning/transport/mod.rs",
            "fixture_malformed_reads",
            "query",
            Some(CONTROLLER),
        ),
        // This isolated peer must accept the consumer canister as its caller.
        (
            "fixture_provisioning/transport/mod.rs",
            "canic_wasm_store_fixture_chunk",
            "update",
            None,
        ),
    ] {
        reviewed.push(ReviewedEndpoint {
            path: format!("{PROBE}/{file}"),
            name,
            kind,
            guard,
        });
    }
    reviewed
}

fn raw_kind(attribute: &Attribute) -> Option<&'static str> {
    let path = attribute.path();
    let kind = path.segments.last()?.ident.to_string();
    let recognized_path = path.segments.len() == 1
        || (path.segments.len() == 2 && path.segments[0].ident == "ic_cdk");
    match (recognized_path, kind.as_str()) {
        (true, "query") => Some("query"),
        (true, "update") => Some("update"),
        _ => None,
    }
}

fn options_match(attribute: &Attribute, guard: Option<&str>) -> bool {
    let args = match &attribute.meta {
        Meta::Path(_) => Punctuated::<Meta, Token![,]>::new(),
        Meta::List(_) => {
            match attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) {
                Ok(args) => args,
                Err(_) => return false,
            }
        }
        Meta::NameValue(_) => return false,
    };
    let Some(expected) = guard else {
        return args.is_empty();
    };
    if args.len() != 1 {
        return false;
    }
    let Some(Meta::NameValue(value)) = args.first() else {
        return false;
    };
    let Expr::Lit(value_expr) = &value.value else {
        return false;
    };
    let Lit::Str(actual) = &value_expr.lit else {
        return false;
    };
    value.path.is_ident("guard") && actual.value() == expected
}

struct EndpointVisitor<'a> {
    path: &'a str,
    reviewed: &'a [ReviewedEndpoint],
    seen: &'a mut BTreeSet<usize>,
    violations: &'a mut Vec<String>,
}

impl<'ast> Visit<'ast> for EndpointVisitor<'_> {
    fn visit_item_fn(&mut self, function: &'ast ItemFn) {
        for attribute in &function.attrs {
            let Some(kind) = raw_kind(attribute) else {
                continue;
            };
            let accepted = self.reviewed.iter().enumerate().find(|(_, entry)| {
                entry.path == self.path
                    && function.sig.ident == entry.name
                    && entry.kind == kind
                    && options_match(attribute, entry.guard)
            });
            if let Some((index, _)) = accepted {
                if !self.seen.insert(index) {
                    self.violations.push(format!(
                        "{} duplicates reviewed endpoint {}",
                        self.path, function.sig.ident
                    ));
                }
            } else {
                self.violations.push(format!(
                    "{} exports unreviewed raw {kind} endpoint {}",
                    self.path, function.sig.ident
                ));
            }
        }
        syn::visit::visit_item_fn(self, function);
    }
}

pub fn violations(workspace: &Path, paths: &BTreeSet<PathBuf>) -> Vec<String> {
    let reviewed = reviewed_endpoints();
    let mut violations = Vec::new();
    let mut seen = BTreeSet::new();
    let packages = reviewed
        .iter()
        .map(|entry| entry.path.split_once("/src/").unwrap().0)
        .collect::<BTreeSet<_>>();
    for package in packages {
        let manifest: toml::Value = toml::from_str(
            &fs::read_to_string(workspace.join(package).join("Cargo.toml"))
                .expect("read reviewed fixture manifest"),
        )
        .expect("parse reviewed fixture manifest");
        assert_eq!(
            manifest["package"]["publish"].as_bool(),
            Some(false),
            "raw fixture instrumentation must stay in unpublished test packages"
        );
    }
    for path in paths {
        let source = fs::read_to_string(path).expect("read managed source");
        let syntax = syn::parse_file(&source).expect("parse managed source");
        let relative = path.strip_prefix(workspace).unwrap().to_str().unwrap();
        EndpointVisitor {
            path: relative,
            reviewed: &reviewed,
            seen: &mut seen,
            violations: &mut violations,
        }
        .visit_file(&syntax);
    }
    for (index, entry) in reviewed.iter().enumerate() {
        if !seen.contains(&index) {
            violations.push(format!(
                "{} missing reviewed endpoint {} with exact guard",
                entry.path, entry.name
            ));
        }
    }
    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str, path: &str) -> Vec<String> {
        let syntax = syn::parse_file(source).unwrap();
        let reviewed = reviewed_endpoints();
        let mut violations = Vec::new();
        EndpointVisitor {
            path,
            reviewed: &reviewed,
            seen: &mut BTreeSet::new(),
            violations: &mut violations,
        }
        .visit_file(&syntax);
        violations
    }

    #[test]
    fn reviewed_instrumentation_requires_exact_guard_and_export_options() {
        let path = format!("{PROBE}/fixture_provisioning/consumer/mod.rs");
        assert!(check(r#"#[ic_cdk::query(guard = "crate::require_test_controller")] fn fixture_consumer_status() {}"#, &path).is_empty());
        for attribute in [
            "#[ic_cdk::query]",
            r#"#[ic_cdk::query(guard = "different_guard")]"#,
            r#"#[ic_cdk::query(guard = "crate::require_test_controller", name = "application_method")]"#,
            r#"#[ic_cdk::update(guard = "crate::require_test_controller")]"#,
        ] {
            assert!(
                !check(
                    &format!("{attribute} fn fixture_consumer_status() {{}}"),
                    &path
                )
                .is_empty()
            );
        }
    }

    #[test]
    fn bare_payload_probe_exception_is_exact() {
        let path = "canisters/test/payload_limit_probe/src/lib.rs";
        let probe = "#[ic_cdk::update] fn bare_echo(payload: String) -> usize { payload.len() }";
        assert!(check(probe, path).is_empty());
        assert!(!check(probe, "apps/production/src/lib.rs").is_empty());
        for source in [
            "#[ic_cdk::update] fn another_update() {}",
            "#[ic_cdk::query] fn bare_echo() {}",
            r#"#[ic_cdk::update(name = "application_method")] fn bare_echo() {}"#,
            r#"#[ic_cdk::update(guard = "different_guard")] fn bare_echo() {}"#,
        ] {
            assert!(!check(source, path).is_empty());
        }
        assert!(!check(&format!("{probe}\n{probe}"), path).is_empty());
    }

    #[test]
    fn raw_application_endpoints_and_additions_to_reviewed_files_are_rejected() {
        let source = "#[::ic_cdk::update] fn application_write() {}";
        assert!(!check(source, "apps/production/src/lib.rs").is_empty());
        let peer = "#[ic_cdk::update] fn canic_wasm_store_fixture_chunk() -> u8 { 0 }";
        assert!(
            check(
                peer,
                &format!("{PROBE}/fixture_provisioning/transport/mod.rs")
            )
            .is_empty()
        );
        assert!(!check(peer, "apps/production/src/lib.rs").is_empty());
        assert!(!check(source, &format!("{PROBE}/lib.rs")).is_empty());
        assert!(
            !check(
                "mod nested { #[query] fn application_read() {} }",
                &format!("{PROBE}/lib.rs")
            )
            .is_empty()
        );
    }
}
