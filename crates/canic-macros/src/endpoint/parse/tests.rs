use super::*;
use quote::quote;

#[test]
fn name_only_is_forwarded_without_requires() {
    let parsed = parse_args(quote!(name = "icrc10_supported_standards"))
        .expect("name-only args should parse");

    assert_eq!(parsed.forwarded.len(), 1);
    assert_eq!(
        parsed.export_name.as_ref().map(LitStr::value).as_deref(),
        Some("icrc10_supported_standards")
    );
    assert!(parsed.requires.is_empty());
    assert!(!parsed.internal);
}

#[test]
fn composite_query_marker_is_forwarded_without_requires() {
    let parsed = parse_args(quote!(composite)).expect("composite-only args should parse");

    assert_eq!(parsed.query_mode, QueryMode::Composite);
    assert_eq!(parsed.forwarded.len(), 1);
    assert_eq!(parsed.forwarded[0].to_string(), "composite = true");
    assert!(parsed.requires.is_empty());
    assert!(!parsed.internal);
}

#[test]
fn composite_query_true_is_forwarded() {
    let parsed = parse_args(quote!(name = "wire_query", composite = true)).expect("parse args");

    assert_eq!(parsed.query_mode, QueryMode::Composite);
    assert_eq!(parsed.forwarded.len(), 2);
    assert!(
        parsed
            .forwarded
            .iter()
            .any(|tokens| tokens.to_string() == "composite = true")
    );
}

#[test]
fn public_marker_is_parsed() {
    let parsed = parse_args(quote!(public)).expect("public marker should parse");

    assert!(parsed.public);
    assert!(parsed.requires.is_empty());
    assert!(parsed.forwarded.is_empty());
}

#[test]
fn public_false_marker_is_rejected() {
    let err = parse_args(quote!(public = false)).expect_err("false public");

    assert!(err.to_string().contains("public must be true"));
}

#[test]
fn composite_query_false_is_rejected() {
    let err = parse_args(quote!(composite = false)).expect_err("false composite");

    assert!(err.to_string().contains("composite must be true"));
}

#[test]
fn duplicate_composite_query_marker_is_rejected() {
    let err = parse_args(quote!(composite, composite = true)).expect_err("duplicate");

    assert!(err.to_string().contains("must appear only once"));
}

#[test]
fn internal_false_marker_is_rejected() {
    let err = parse_args(quote!(internal = false)).expect_err("false internal");

    assert!(err.to_string().contains("internal must be true"));
}

#[test]
fn internal_non_boolean_marker_is_rejected() {
    let err = parse_args(quote!(internal = "yes")).expect_err("non-boolean internal");

    assert!(
        err.to_string()
            .contains("internal must be set to a boolean literal")
    );
}

#[test]
fn composite_non_boolean_marker_is_rejected() {
    let err = parse_args(quote!(composite = "yes")).expect_err("non-boolean composite");

    assert!(
        err.to_string()
            .contains("composite must be set to a boolean literal")
    );
}

#[test]
fn payload_max_bytes_is_parsed() {
    let parsed =
        parse_args(quote!(payload(max_bytes = 64 * 1024))).expect("payload args should parse");

    assert_eq!(
        parsed.payload_max_bytes.expect("payload limit").to_string(),
        "64 * 1024"
    );
}

#[test]
fn duplicate_name_is_rejected() {
    let err = parse_args(quote!(name = "a", name = "b")).expect_err("duplicate name");
    assert!(err.to_string().contains("must appear only once"));
}

#[test]
fn duplicate_payload_is_rejected() {
    let err = parse_args(quote!(payload(max_bytes = 1024), payload(max_bytes = 2048)))
        .expect_err("duplicate payload");
    assert!(err.to_string().contains("must appear only once"));
}

#[test]
fn authenticated_allows_no_scope_argument() {
    let parsed = parse_args(quote!(requires(auth::authenticated()))).expect("parse args");
    let AccessExprAst::All(exprs) = &parsed.requires[0] else {
        panic!("expected requires(all)");
    };
    let AccessExprAst::Pred(AccessPredicateAst::Builtin(BuiltinPredicate::Authenticated {
        required_scope,
    })) = &exprs[0]
    else {
        panic!("expected authenticated predicate");
    };
    assert!(required_scope.is_none());
}

#[test]
fn grouped_access_expression_is_unwrapped() {
    let parsed = parse_args(quote!(requires((caller::is_controller())))).expect("parse args");
    let AccessExprAst::All(exprs) = &parsed.requires[0] else {
        panic!("expected requires(all)");
    };
    let AccessExprAst::Pred(AccessPredicateAst::Builtin(BuiltinPredicate::CallerIsController)) =
        &exprs[0]
    else {
        panic!("expected caller::is_controller predicate");
    };
}

#[test]
fn authenticated_allows_string_scope_argument() {
    let parsed =
        parse_args(quote!(requires(auth::authenticated("scope:test")))).expect("parse args");
    let AccessExprAst::All(exprs) = &parsed.requires[0] else {
        panic!("expected requires(all)");
    };
    let AccessExprAst::Pred(AccessPredicateAst::Builtin(BuiltinPredicate::Authenticated {
        required_scope,
    })) = &exprs[0]
    else {
        panic!("expected authenticated predicate");
    };
    let Some(AuthScopeArg::Literal(required_scope)) = required_scope else {
        panic!("expected literal scope");
    };
    assert_eq!(required_scope, "scope:test");
}

#[test]
fn authenticated_allows_path_scope_argument() {
    let parsed =
        parse_args(quote!(requires(auth::authenticated(cap::VERIFY)))).expect("parse args");
    let AccessExprAst::All(exprs) = &parsed.requires[0] else {
        panic!("expected requires(all)");
    };
    let AccessExprAst::Pred(AccessPredicateAst::Builtin(BuiltinPredicate::Authenticated {
        required_scope,
    })) = &exprs[0]
    else {
        panic!("expected authenticated predicate");
    };
    let Some(AuthScopeArg::Expr(required_scope)) = required_scope else {
        panic!("expected expr scope");
    };
    assert_eq!(required_scope.to_string(), "cap :: VERIFY");
}

#[test]
fn authenticated_rejects_multiple_arguments() {
    let err = parse_args(quote!(requires(auth::authenticated("a", "b"))))
        .expect_err("authenticated with two args must fail");
    assert!(
        err.to_string()
            .contains("authenticated(...) accepts zero arguments or one string literal/path scope")
    );
}

#[test]
fn authenticated_requires_builtin_namespace() {
    let err = parse_args(quote!(requires(authenticated()))).expect_err("unqualified path fails");
    assert!(
        err.to_string()
            .contains("built-in predicates must use short paths like auth::authenticated()")
    );
}
