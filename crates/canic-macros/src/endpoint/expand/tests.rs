use super::*;
use crate::endpoint::parse::{AccessExprAst, AccessPredicateAst, AuthScopeArg, BuiltinPredicate};

fn make_args(requires: Vec<AccessExprAst>) -> ValidatedArgs {
    ValidatedArgs {
        forwarded: Vec::new(),
        export_name: None,
        payload_max_bytes: None,
        requires,
        internal: false,
        query_mode: QueryMode::Plain,
    }
}

#[test]
fn endpoint_expansion_omits_removed_endpoint_metric_hooks() {
    let args = make_args(Vec::new());
    let func: ItemFn = syn::parse_quote!(
        fn ping() -> Result<(), ::canic::Error> {
            Ok(())
        }
    );

    let expanded = expand(EndpointKind::Query, args, func).to_string();

    assert!(!expanded.contains("EndpointAttemptMetrics"));
    assert!(!expanded.contains("EndpointResultMetrics"));
}

#[test]
fn update_expansion_registers_payload_limit_for_exported_name() {
    let mut args = make_args(Vec::new());
    args.export_name = Some(syn::LitStr::new(
        "wire_ping",
        proc_macro2::Span::call_site(),
    ));
    args.payload_max_bytes = Some(quote!(64 * 1024));
    let func: ItemFn = syn::parse_quote!(
        fn ping() -> Result<(), ::canic::Error> {
            Ok(())
        }
    );

    let expanded = expand(EndpointKind::Update, args, func).to_string();

    assert!(expanded.contains("register_update_limit"));
    assert!(expanded.contains("\"wire_ping\""));
    assert!(expanded.contains("64 * 1024"));
}

#[test]
fn composite_query_expansion_forwards_cdk_attr_and_call_kind() {
    let mut args = make_args(Vec::new());
    args.forwarded.push(quote!(composite = true));
    args.query_mode = QueryMode::Composite;
    let func: ItemFn = syn::parse_quote!(
        fn ping() -> Result<(), ::canic::Error> {
            Ok(())
        }
    );

    let expanded = expand(EndpointKind::Query, args, func).to_string();
    let compact = expanded.split_whitespace().collect::<String>();

    assert!(compact.contains("query(composite=true)"));
    assert!(compact.contains("EndpointCallKind::QueryComposite"));
}

#[test]
fn default_app_guard_keeps_sync_wrapper_sync() {
    let sig: Signature = syn::parse_quote!(fn ping() -> Result<(), ::canic::Error>);
    let args = make_args(Vec::new());
    let plan = build_access_plan(EndpointKind::Update, &args, &sig).expect("access plan");

    assert!(!plan.requires_async());
    assert!(!(sig.asyncness.is_some() || plan.requires_async()));
}

#[test]
fn explicit_requires_forces_async_wrapper() {
    let sig: Signature = syn::parse_quote!(fn ping() -> Result<(), ::canic::Error>);
    let args = make_args(vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
        BuiltinPredicate::CallerIsController,
    ))]);
    let plan = build_access_plan(EndpointKind::Update, &args, &sig).expect("access plan");

    assert!(plan.requires_async());
    assert!(sig.asyncness.is_some() || plan.requires_async());
}

#[test]
fn app_command_endpoints_skip_app_guard_and_reject_gating() {
    let sig: Signature = syn::parse_quote!(
        fn apply(cmd: ::canic::dto::state::AppCommand) -> Result<(), ::canic::Error>
    );

    let args = make_args(Vec::new());
    let plan = build_access_plan(EndpointKind::Update, &args, &sig).expect("access plan");
    std::assert_matches!(plan, AccessPlan::None);

    let args = make_args(vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
        BuiltinPredicate::AppAllowsUpdates,
    ))]);
    let err = build_access_plan(EndpointKind::Update, &args, &sig).unwrap_err();
    assert!(
        err.to_string()
            .contains("AppCommand endpoints must never be gated on application state.")
    );
}

#[test]
fn access_stage_expr_builds_context_from_resolved_identity() {
    let sig: Signature = syn::parse_quote!(fn ping() -> Result<(), ::canic::Error>);
    let args = make_args(vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
        BuiltinPredicate::CallerIsController,
    ))]);
    let plan = build_access_plan(EndpointKind::Update, &args, &sig).expect("access plan");
    let call = format_ident!("__canic_call");
    let stage = access_stage(&plan, &call).to_string();
    let compact = stage.split_whitespace().collect::<String>();

    assert!(compact.contains("resolve_authenticated_identity("));
    assert!(compact.contains("caller:__canic_authenticated_identity.transport_caller"));
    assert!(
        compact
            .contains("authenticated_caller:__canic_authenticated_identity.authenticated_subject")
    );
    assert!(compact.contains("identity_source:__canic_authenticated_identity.identity_source"));
}

#[test]
fn access_stage_default_guard_marks_identity_source_raw_caller() {
    let sig: Signature = syn::parse_quote!(fn ping() -> Result<(), ::canic::Error>);
    let args = make_args(Vec::new());
    let plan = build_access_plan(EndpointKind::Update, &args, &sig).expect("access plan");
    let call = format_ident!("__canic_call");
    let stage = access_stage(&plan, &call).to_string();
    let compact = stage.split_whitespace().collect::<String>();

    assert!(
        compact.contains("identity_source::canic::__internal::core::access::auth::AuthenticatedIdentitySource::RawCaller")
            || compact.contains("identity_source:::canic::__internal::core::access::auth::AuthenticatedIdentitySource::RawCaller")
    );
}

#[test]
fn authenticated_endpoint_expansion_evaluates_access_before_dispatch() {
    let args = make_args(vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
        BuiltinPredicate::Authenticated {
            required_scope: Some(AuthScopeArg::Literal(String::from("write"))),
        },
    ))]);
    let func: ItemFn = syn::parse_quote!(
        async fn write(token: ::canic::dto::auth::DelegatedToken) -> Result<(), ::canic::Error> {
            Ok(())
        }
    );

    let expanded = expand(EndpointKind::Update, args, func).to_string();

    let access = expanded
        .find("eval_access")
        .expect("expanded endpoint must evaluate access");
    let dispatch = expanded
        .find("dispatch_update_async")
        .expect("expanded endpoint must dispatch update after access");
    let impl_call = expanded
        .find("__canic_impl_write")
        .expect("expanded endpoint must call implementation");

    assert!(access < dispatch);
    assert!(dispatch < impl_call);
}
