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
fn default_update_expansion_uses_runtime_fallback_without_registration() {
    let args = make_args(Vec::new());
    let func: ItemFn = syn::parse_quote!(
        fn ping() -> Result<(), ::canic::Error> {
            Ok(())
        }
    );

    let expanded = expand(EndpointKind::Update, args, func).to_string();

    assert!(!expanded.contains("register_update_limit"));
    assert!(!expanded.contains("__canic_ctor_payload_limit_ping"));
}

#[test]
fn explicit_update_payload_limit_uses_raw_predecode_adapter() {
    let mut args = make_args(Vec::new());
    args.payload_max_bytes = Some(quote!(16 * 1024));
    let func: ItemFn = syn::parse_quote!(
        fn ping(payload: String) -> Result<usize, ::canic::Error> {
            Ok(payload.len())
        }
    );

    let expanded = expand(EndpointKind::Update, args, func).to_string();
    let compact = expanded.split_whitespace().collect::<String>();
    let size = compact
        .find("msg_arg_data_size")
        .expect("scalar size check");
    let allocation = compact
        .find("vec![0_u8;__canic_payload_len]")
        .expect("bounded allocation");
    let copy = compact
        .find("msg_arg_data_copy")
        .expect("argument copy after size check");
    let decode = compact
        .find("decode_args_with_config")
        .expect("Candid decode after copy");

    assert!(compact.contains("candid_method(update,rename=\"ping\")"));
    assert!(compact.contains("export_name=\"canister_updateping\""));
    assert!(size < allocation);
    assert!(allocation < copy);
    assert!(copy < decode);
    assert!(!compact.contains("::canic::__internal::cdk::update"));
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
    assert!(compact.contains("::canic::__internal::cdk::query"));
    assert!(compact.contains("EndpointCallKind::QueryComposite"));
}

#[test]
fn default_fleet_guard_keeps_sync_wrapper_sync() {
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
fn fleet_command_endpoints_skip_fleet_guard_and_reject_gating() {
    let sig: Signature = syn::parse_quote!(
        fn apply(cmd: ::canic::dto::state::FleetCommand) -> Result<(), ::canic::Error>
    );

    let args = make_args(Vec::new());
    let plan = build_access_plan(EndpointKind::Update, &args, &sig).expect("access plan");
    std::assert_matches!(plan, AccessPlan::None);

    let args = make_args(vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
        BuiltinPredicate::FleetAllowsUpdates,
    ))]);
    let err = build_access_plan(EndpointKind::Update, &args, &sig).unwrap_err();
    assert!(
        err.to_string()
            .contains("FleetCommand endpoints must never be gated on Fleet state.")
    );
}

#[test]
fn access_stage_expr_builds_context_from_the_exact_transport_caller() {
    let sig: Signature = syn::parse_quote!(fn ping() -> Result<(), ::canic::Error>);
    let args = make_args(vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
        BuiltinPredicate::CallerIsController,
    ))]);
    let plan = build_access_plan(EndpointKind::Update, &args, &sig).expect("access plan");
    let call = format_ident!("__canic_call");
    let stage = access_stage(&plan, &call).to_string();
    let compact = stage.split_whitespace().collect::<String>();

    assert!(compact.contains("::canic::__internal::cdk::api::msg_caller"));
    assert!(compact.contains("caller:__canic_caller"));
    assert!(!compact.contains("authenticated_caller"));
}

#[test]
fn access_stage_default_guard_skips_the_unused_transport_caller() {
    let sig: Signature = syn::parse_quote!(fn ping() -> Result<(), ::canic::Error>);
    let args = make_args(Vec::new());
    let plan = build_access_plan(EndpointKind::Update, &args, &sig).expect("access plan");
    let call = format_ident!("__canic_call");
    let stage = access_stage(&plan, &call).to_string();
    let compact = stage.split_whitespace().collect::<String>();

    assert!(compact.contains("eval_default_fleet_guard"));
    assert!(compact.contains("__canic_call"));
    assert!(!compact.contains("msg_caller"));
}

#[test]
fn authenticated_endpoint_expansion_fences_before_access_and_dispatch() {
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
    let compact = expanded.split_whitespace().collect::<String>();

    let fence = expanded
        .find("preflight_endpoint")
        .expect("expanded endpoint must enforce the activation fence");
    let access = expanded
        .find("eval_access")
        .expect("expanded endpoint must evaluate access");
    let enter = expanded
        .find("enter_endpoint")
        .expect("expanded endpoint must enter instrumentation after access");
    let impl_call = expanded
        .find("__canic_impl_write")
        .expect("expanded endpoint must call implementation");
    let exit = expanded
        .find("exit_endpoint")
        .expect("expanded endpoint must exit instrumentation after implementation");

    assert!(fence < access);
    assert!(access < enter);
    assert!(enter < impl_call);
    assert!(impl_call < exit);
    assert!(compact.contains("::canic::application_scope!(\"write\").as_str()"));
    assert!(!compact.contains("authenticated_with_scope(\"write\")"));
}

#[test]
fn store_data_endpoint_expansion_selects_store_only_preflight() {
    let args = make_args(Vec::new());
    let func: ItemFn = syn::parse_quote!(
        fn canic_wasm_store_publish_chunk(bytes: Vec<u8>) -> Result<(), ::canic::Error> {
            let _ = bytes;
            Ok(())
        }
    );

    let compact = expand(EndpointKind::Update, args, func)
        .to_string()
        .split_whitespace()
        .collect::<String>();

    assert!(compact.contains("preflight_store_data_endpoint(__canic_call)"));
    assert!(!compact.contains("preflight_endpoint(__canic_call)"));
}

#[test]
fn attested_local_subnet_expands_to_the_local_proof_guard() {
    let args = make_args(vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
        BuiltinPredicate::AttestedLocalSubnet,
    ))]);
    let func: ItemFn = syn::parse_quote!(
        async fn local_call(
            attestation: ::canic::dto::auth::SignedRoleAttestation,
        ) -> Result<(), ::canic::Error> {
            Ok(())
        }
    );

    let expanded = expand(EndpointKind::Update, args, func).to_string();
    let compact = expanded.split_whitespace().collect::<String>();

    assert!(compact.contains("access::expr::auth::attested_local_subnet()"));
    assert!(compact.contains("let_=&attestation"));
}
