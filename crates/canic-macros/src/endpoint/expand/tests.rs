use super::*;
use crate::endpoint::parse::AuthScopeArg;

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

#[test]
fn protected_internal_role_endpoint_exports_envelope_wrapper() {
    let mut args = make_args(vec![AccessExprAst::All(vec![AccessExprAst::Pred(
        AccessPredicateAst::Builtin(BuiltinPredicate::CallerHasRole {
            role: CanisterRoleArg::Literal("project_hub".to_string()),
        }),
    )])]);
    args.internal = true;
    let func: ItemFn = syn::parse_quote!(
        async fn system_add_project_to_user(
            user_id: ::canic::cdk::types::Principal,
            project_pid: ::canic::cdk::types::Principal,
        ) -> Result<(), ::canic::Error> {
            Ok(())
        }
    );

    let expanded = expand(EndpointKind::Update, args, func).to_string();
    let compact = expanded.split_whitespace().collect::<String>();

    assert!(compact.contains("msg_arg_data"));
    assert!(compact.contains("decode_one"));
    assert!(compact.contains("CanicInternalCallEnvelopeV1"));
    assert!(compact.contains("verify_internal_invocation_proof"));
    assert!(compact.contains("decode_args::<("));
    assert!(!compact.contains("eval_access"));

    let envelope_decode = compact
        .find("decode_one")
        .expect("protected wrapper must decode the envelope inside Canic");
    let verify = compact
        .find("verify_internal_invocation_proof")
        .expect("protected wrapper must verify proof");
    let args_decode = compact
        .find("decode_args")
        .expect("protected wrapper must decode original args");
    let dispatch = compact
        .find("dispatch_update_async")
        .expect("protected wrapper must dispatch after verification");
    assert!(envelope_decode < verify);
    assert!(verify < args_decode);
    assert!(args_decode < dispatch);
}

#[test]
fn protected_internal_role_endpoint_verifies_exported_method_name() {
    let mut args = make_args(vec![AccessExprAst::All(vec![AccessExprAst::Pred(
        AccessPredicateAst::Builtin(BuiltinPredicate::CallerHasRole {
            role: CanisterRoleArg::Literal("project_hub".to_string()),
        }),
    )])]);
    args.internal = true;
    args.export_name = Some(syn::LitStr::new(
        "wire_system_add_project_to_user",
        proc_macro2::Span::call_site(),
    ));
    let func: ItemFn = syn::parse_quote!(
        async fn system_add_project_to_user(
            user_id: ::canic::cdk::types::Principal,
            project_pid: ::canic::cdk::types::Principal,
        ) -> Result<(), ::canic::Error> {
            Ok(())
        }
    );

    let expanded = expand(EndpointKind::Update, args, func).to_string();
    let compact = expanded.split_whitespace().collect::<String>();

    assert!(compact.contains("let__canic_method=\"wire_system_add_project_to_user\";"));
    assert!(
        compact.contains("__canic_envelope.header.target_method!=__canic_method"),
        "envelope target method must be checked against the exported method name"
    );
    assert!(
        compact.contains("&__canic_envelope.proof,__canic_method,&__canic_accepted_roles"),
        "proof verification must bind the exported method name"
    );
}

#[test]
fn protected_internal_role_endpoint_emits_generated_client_descriptor() {
    let mut args = make_args(vec![AccessExprAst::All(vec![AccessExprAst::Pred(
        AccessPredicateAst::Builtin(BuiltinPredicate::CallerHasRole {
            role: CanisterRoleArg::Literal("project_hub".to_string()),
        }),
    )])]);
    args.internal = true;
    args.export_name = Some(syn::LitStr::new(
        "wire_system_add_project_to_user",
        proc_macro2::Span::call_site(),
    ));
    let func: ItemFn = syn::parse_quote!(
        pub async fn system_add_project_to_user(
            user_id: ::canic::cdk::types::Principal,
            project_pid: ::canic::cdk::types::Principal,
        ) -> Result<(), ::canic::Error> {
            Ok(())
        }
    );

    let expanded = expand(EndpointKind::Update, args, func).to_string();
    let compact = expanded.split_whitespace().collect::<String>();

    assert!(compact.contains("pubfncanic_internal_endpoint_system_add_project_to_user()"));
    assert!(compact.contains("::canic::api::ic::ProtectedInternalEndpoint::new"));
    assert!(compact.contains("\"wire_system_add_project_to_user\""));
    assert!(compact.contains("CanisterRole::new(\"project_hub\")"));
}

#[test]
fn protected_internal_any_role_descriptor_keeps_all_roles() {
    let mut args = make_args(vec![AccessExprAst::All(vec![AccessExprAst::Pred(
        AccessPredicateAst::Builtin(BuiltinPredicate::CallerHasAnyRole {
            roles: vec![
                CanisterRoleArg::Literal("project_hub".to_string()),
                CanisterRoleArg::Literal("admin_hub".to_string()),
            ],
        }),
    )])]);
    args.internal = true;
    let func: ItemFn = syn::parse_quote!(
        async fn protected() -> Result<(), ::canic::Error> {
            Ok(())
        }
    );

    let expanded = expand(EndpointKind::Update, args, func).to_string();
    let compact = expanded.split_whitespace().collect::<String>();

    assert!(compact.contains("canic_internal_endpoint_protected()"));
    assert!(compact.contains("CanisterRole::new(\"project_hub\")"));
    assert!(compact.contains("CanisterRole::new(\"admin_hub\")"));
}

#[test]
fn protected_internal_role_endpoint_rejects_mixed_access_predicates() {
    let mut args = make_args(vec![AccessExprAst::All(vec![
        AccessExprAst::Pred(AccessPredicateAst::Builtin(
            BuiltinPredicate::CallerHasRole {
                role: CanisterRoleArg::Literal("project_hub".to_string()),
            },
        )),
        AccessExprAst::Pred(AccessPredicateAst::Builtin(
            BuiltinPredicate::CallerIsController,
        )),
    ])]);
    args.internal = true;
    let func: ItemFn = syn::parse_quote!(
        async fn protected() -> Result<(), ::canic::Error> {
            Ok(())
        }
    );

    let expanded = expand(EndpointKind::Update, args, func).to_string();

    assert!(expanded.contains("protected endpoints may only combine attested role predicates"));
}

#[test]
fn protected_internal_any_role_endpoint_exports_multiple_accepted_roles() {
    let mut args = make_args(vec![AccessExprAst::All(vec![AccessExprAst::Pred(
        AccessPredicateAst::Builtin(BuiltinPredicate::CallerHasAnyRole {
            roles: vec![
                CanisterRoleArg::Literal("project_hub".to_string()),
                CanisterRoleArg::Literal("admin_hub".to_string()),
            ],
        }),
    )])]);
    args.internal = true;
    let func: ItemFn = syn::parse_quote!(
        async fn protected() -> Result<(), ::canic::Error> {
            Ok(())
        }
    );

    let expanded = expand(EndpointKind::Update, args, func).to_string();

    assert!(expanded.contains("project_hub"));
    assert!(expanded.contains("admin_hub"));
    assert!(expanded.contains("verify_internal_invocation_proof"));
}
