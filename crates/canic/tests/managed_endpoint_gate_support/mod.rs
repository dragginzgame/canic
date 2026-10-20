//! Module: managed_endpoint_gate_support
//!
//! Responsibility: qualify exact raw fixture instrumentation outside readiness dispatch.
//! Does not own: application endpoints, authorization implementations or runtime behavior.
//! Boundary: exact unpublished probes exercise readiness, codecs, ingress limits and counters.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use syn::{Attribute, Expr, ItemFn, Lit, Meta, Token, punctuated::Punctuated, visit::Visit};

/// Exact test endpoint admission, including the complete allowed CDK options.
struct ReviewedEndpoint {
    path: String,
    name: &'static str,
    kind: &'static str,
    guard: Option<&'static str>,
    decoder: &'static str,
    encoder: &'static str,
}

fn reviewed_endpoints() -> Vec<ReviewedEndpoint> {
    vec![
        ReviewedEndpoint {
            path: "apps/test/user_hub/src/fixture_importer/mod.rs".into(),
            name: "test_release_fixture",
            kind: "update",
            guard: Some("require_controller"),
            decoder: "decode_release",
            encoder: "encode_release",
        },
        // This standalone probe must use the bare CDK adapter to qualify the
        // inherited inspector limit independently of Canic's update macro.
        ReviewedEndpoint {
            path: "canisters/test/payload_limit_probe/src/lib.rs".into(),
            name: "bare_echo",
            kind: "update",
            guard: None,
            decoder: "codec::decode_one",
            encoder: "codec::encode_reply",
        },
        // Simulator polling must observe invocation state and measurements
        // without dispatch instrumentation adding samples to the measured table.
        ReviewedEndpoint {
            path: "canisters/test/runtime_probe/src/perf_context/mod.rs".into(),
            name: "perf_context_entered",
            kind: "query",
            guard: None,
            decoder: "codec::decode_args",
            encoder: "codec::encode_reply",
        },
        ReviewedEndpoint {
            path: "canisters/test/runtime_probe/src/perf_context/mod.rs".into(),
            name: "perf_context_completed",
            kind: "query",
            guard: None,
            decoder: "codec::decode_args",
            encoder: "codec::encode_reply",
        },
        ReviewedEndpoint {
            path: "canisters/test/runtime_probe/src/perf_context/mod.rs".into(),
            name: "perf_context_metrics",
            kind: "query",
            guard: None,
            decoder: "codec::decode_args",
            encoder: "codec::encode_reply",
        },
    ]
}

fn raw_kind(attribute: &Attribute) -> Option<&'static str> {
    let path = attribute.path();
    let kind = path.segments.last()?.ident.to_string();
    let recognized_path = path.segments.len() == 1
        || (path.segments.len() == 2 && path.segments[0].ident == "ic_cdk")
        || (path.segments.len() == 3
            && path.segments[0].ident == "ic_platform"
            && path.segments[1].ident == "cdk");
    match (recognized_path, kind.as_str()) {
        (true, "query") => Some("query"),
        (true, "update") => Some("update"),
        _ => None,
    }
}

fn options_match(attribute: &Attribute, endpoint: &ReviewedEndpoint) -> bool {
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
    let mut expected = vec![
        ("decode_with", endpoint.decoder),
        ("encode_with", endpoint.encoder),
    ];
    if let Some(guard) = endpoint.guard {
        expected.push(("guard", guard));
    }
    if args.len() != expected.len() {
        return false;
    }
    expected.into_iter().all(|(name, expected)| {
        args.iter()
            .filter(|arg| {
                let Meta::NameValue(value) = arg else {
                    return false;
                };
                let Expr::Lit(value_expr) = &value.value else {
                    return false;
                };
                let Lit::Str(actual) = &value_expr.lit else {
                    return false;
                };
                value.path.is_ident(name) && actual.value() == expected
            })
            .count()
            == 1
    })
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
                    && options_match(attribute, entry)
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

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
        let path = "apps/test/user_hub/src/fixture_importer/mod.rs";
        let probe = r#"#[ic_platform::cdk::update(guard = "require_controller", decode_with = "decode_release", encode_with = "encode_release")] fn test_release_fixture() {}"#;
        assert!(check(probe, path).is_empty());
        for invalid in [
            probe.replace(r#"guard = "require_controller", "#, ""),
            probe.replace("require_controller", "different_guard"),
            probe.replace("decode_release", "other_decoder"),
            probe.replace("encode_release", "other_encoder"),
            probe.replace("guard =", r#"name = "application_method", guard ="#),
            probe.replace("cdk::update", "cdk::query"),
        ] {
            assert!(!check(&invalid, path).is_empty());
        }
    }

    #[test]
    fn bare_payload_probe_exception_is_exact() {
        let path = "canisters/test/payload_limit_probe/src/lib.rs";
        let probe = r#"#[ic_platform::cdk::update(decode_with = "codec::decode_one", encode_with = "codec::encode_reply")] fn bare_echo(payload: String) -> usize { payload.len() }"#;
        assert!(check(probe, path).is_empty());
        for invalid in [
            probe.replace("codec::decode_one", "codec::decode_args"),
            probe.replace("codec::encode_reply", "other_encoder"),
            probe.replace(r#"decode_with = "codec::decode_one", "#, ""),
            probe.replace(
                r#"encode_with = "codec::encode_reply""#,
                r#"name = "bare_echo""#,
            ),
            probe.replace(
                "decode_with =",
                r#"decode_with = "codec::decode_one", decode_with ="#,
            ),
            probe.replace("decode_with =", "composite = true, decode_with ="),
        ] {
            assert!(!check(&invalid, path).is_empty());
        }
        assert!(!check(probe, "apps/production/src/lib.rs").is_empty());
        for source in [
            "#[ic_platform::cdk::update] fn another_update() {}",
            "#[ic_platform::cdk::query] fn bare_echo() {}",
            r#"#[ic_platform::cdk::update(name = "application_method")] fn bare_echo() {}"#,
            r#"#[ic_platform::cdk::update(guard = "different_guard")] fn bare_echo() {}"#,
        ] {
            assert!(!check(source, path).is_empty());
        }
        assert!(!check(&format!("{probe}\n{probe}"), path).is_empty());
    }

    #[test]
    fn performance_observation_exceptions_require_exact_exports() {
        let path = "canisters/test/runtime_probe/src/perf_context/mod.rs";
        for name in [
            "perf_context_entered",
            "perf_context_completed",
            "perf_context_metrics",
        ] {
            let probe = format!(
                r#"#[ic_platform::cdk::query(decode_with = "codec::decode_args", encode_with = "codec::encode_reply")] fn {name}() {{}}"#
            );
            assert!(check(&probe, path).is_empty());
            assert!(!check(&probe, "apps/production/src/lib.rs").is_empty());
            assert!(!check(&format!("{probe}\n{probe}"), path).is_empty());
            for invalid in [
                probe.replace("cdk::query", "cdk::update"),
                probe.replace(
                    "decode_with =",
                    r#"name = "application_method", decode_with ="#,
                ),
                probe.replace("decode_with =", "composite = true, decode_with ="),
                probe.replace(
                    "decode_with =",
                    r#"guard = "different_guard", decode_with ="#,
                ),
                probe.replace("codec::decode_args", "codec::decode_one"),
                probe.replace("codec::encode_reply", "other_encoder"),
            ] {
                assert!(!check(&invalid, path).is_empty());
            }
        }
        assert!(!check("#[ic_platform::cdk::query] fn another_query() {}", path).is_empty());
    }

    #[test]
    fn raw_application_endpoints_and_additions_to_reviewed_files_are_rejected() {
        let source = "#[::ic_platform::cdk::update] fn application_write() {}";
        assert!(!check(source, "apps/production/src/lib.rs").is_empty());
        assert!(!check(source, "apps/test/user_hub/src/fixture_importer/mod.rs").is_empty());
        assert!(
            !check(
                "mod nested { #[query] fn application_read() {} }",
                "apps/test/user_hub/src/fixture_importer/mod.rs"
            )
            .is_empty()
        );
    }
}
