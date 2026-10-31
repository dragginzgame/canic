use super::*;
use crate::endpoint::parse::{AccessExprAst, AccessPredicateAst, BuiltinPredicate};

fn make_args(requires: Vec<AccessExprAst>) -> ValidatedArgs {
    ValidatedArgs {
        forwarded: Vec::new(),
        export_name: None,
        payload_max_bytes: None,
        decode: None,
        wire: None,
        reject_access: false,
        requires,
        internal: false,
        query_mode: QueryMode::Plain,
    }
}

#[test]
fn bounded_guards_consume_decoded_proofs_and_keep_the_fleet_guard() {
    for wire in [false, true] {
        let mut args = make_args(vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
            BuiltinPredicate::AttestedLocalSubnet,
        ))]);
        if wire {
            args.wire = Some(crate::endpoint::WireArgs {
                id: quote!(7),
                request: quote!(REQUEST),
                response: quote!(RESPONSE),
            });
        } else {
            args.decode = Some(quote!(LIMITS));
        }
        args.reject_access = true;
        let function: ItemFn = syn::parse_quote!(
            fn read(token: SignedRoleAttestation) -> u64 {
                1
            }
        );
        let expansion = expand(EndpointKind::Query, args, function).to_string();
        let compact = expansion.split_whitespace().collect::<String>();
        assert_eq!(
            compact.contains("cfg_attr(any(not(target_arch=\"wasm32\"),canic_export_candid)"),
            !wire
        );
        assert!(compact.contains("attested_local_subnet_argument(&token)"));
        assert!(compact.contains("fleet::is_queryable()"));
        assert!(compact.contains("in_tracking_query_executor_context"));
        assert!(compact.contains("msg_reject"));
        assert_eq!(
            compact.contains("candid_method(query,rename=\"read\")"),
            !wire
        );
        assert!(compact.contains("__canic_impl_read(token)"));
        assert!(!compact.contains("__canic_impl_read(token).await"));
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

    assert!(compact.contains("composite=true)"));
    assert!(compact.contains("::canic::__internal::cdk::query"));
    assert!(compact.contains("EndpointCallKind::QueryComposite"));
}

#[test]
fn access_checks_preserve_handler_synchrony() {
    for kind in [EndpointKind::Query, EndpointKind::Update] {
        for guarded in [false, true] {
            for asynchronous in [false, true] {
                for bounded in [false, true] {
                    let requires = if guarded {
                        vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
                            BuiltinPredicate::CallerIsController,
                        ))]
                    } else {
                        Vec::new()
                    };
                    let mut args = make_args(requires);
                    if bounded {
                        args.payload_max_bytes = Some(quote!(1024));
                    }
                    let mut function: ItemFn = syn::parse_quote!(
                        fn ping() -> Result<(), ::canic::Error> {
                            Ok(())
                        }
                    );
                    if asynchronous {
                        function.sig.asyncness = Some(syn::token::Async::default());
                    }
                    let output: syn::File =
                        syn::parse2(expand(kind, args, function)).expect("valid expanded items");
                    let wrapper = output
                        .items
                        .iter()
                        .find_map(|item| match item {
                            syn::Item::Fn(item) if item.sig.ident == "ping" => Some(item),
                            _ => None,
                        })
                        .expect("endpoint wrapper");
                    assert_eq!(wrapper.sig.asyncness.is_some(), asynchronous);
                }
            }
        }
    }
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
fn exported_store_endpoint_uses_the_role_checked_preflight() {
    let mut args = make_args(Vec::new());
    args.export_name = Some(syn::parse_quote!("canic_wasm_store_publish_chunk"));
    let func: ItemFn = syn::parse_quote!(
        fn publish_chunk(bytes: Vec<u8>) -> Result<(), ::canic::Error> {
            let _ = bytes;
            Ok(())
        }
    );

    let compact = expand(EndpointKind::Update, args, func)
        .to_string()
        .split_whitespace()
        .collect::<String>();

    assert!(compact.contains("preflight_endpoint(__canic_call)"));
    assert!(compact.contains("EndpointId::new(\"canic_wasm_store_publish_chunk\")"));
}

#[test]
fn attested_local_subnet_expands_to_the_local_proof_guard() {
    let mut args = make_args(vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
        BuiltinPredicate::AttestedLocalSubnet,
    ))]);
    args.wire = Some(crate::endpoint::WireArgs {
        id: quote!(7),
        request: quote!(REQUEST),
        response: quote!(RESPONSE),
    });
    let func: ItemFn = syn::parse_quote!(
        async fn local_call(
            attestation: ::canic::dto::role_attestation::SignedRoleAttestation,
        ) -> Result<(), ::canic::Error> {
            Ok(())
        }
    );

    let expanded = expand(EndpointKind::Update, args, func).to_string();
    let compact = expanded.split_whitespace().collect::<String>();

    assert!(compact.contains("access::expr::auth::attested_local_subnet_argument(&attestation)"));
    assert!(compact.contains("let_=&attestation"));
}

#[test]
fn access_reader_selection_follows_nested_declared_predicates() {
    struct SelectedReader(Vec<bool>);
    impl<'ast> syn::visit::Visit<'ast> for SelectedReader {
        fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
            if let syn::Expr::Path(path) = call.func.as_ref()
                && let Some(segment) = path.path.segments.last()
                && segment.ident == "eval_access_selected"
            {
                let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
                    panic!("selected evaluator requires a const selection");
                };
                let Some(syn::GenericArgument::Const(syn::Expr::Lit(lit))) = args.args.first()
                else {
                    panic!("selection must be a const boolean");
                };
                let syn::Lit::Bool(selected) = &lit.lit else {
                    panic!("selection must be boolean");
                };
                self.0.push(selected.value);
            }
            syn::visit::visit_expr_call(self, call);
        }
    }
    let ordinary = AccessExprAst::Pred(AccessPredicateAst::Builtin(
        BuiltinPredicate::CallerIsParent,
    ));
    let admitted = AccessExprAst::Pred(AccessPredicateAst::Builtin(
        BuiltinPredicate::CallerIsFleetAdmitted,
    ));
    let custom = AccessExprAst::Pred(AccessPredicateAst::Custom(quote!(custom_guard())));
    let cases = [
        (ordinary.clone(), false),
        (custom, false),
        (admitted.clone(), true),
        (
            AccessExprAst::All(vec![ordinary.clone(), admitted.clone()]),
            true,
        ),
        (
            AccessExprAst::Any(vec![
                ordinary.clone(),
                AccessExprAst::Not(Box::new(admitted)),
            ]),
            true,
        ),
        (
            AccessExprAst::Not(Box::new(AccessExprAst::Any(vec![ordinary]))),
            false,
        ),
    ];
    for (expr, expected) in cases {
        let args = make_args(vec![expr]);
        let sig = syn::parse_quote!(fn probe() -> Result<(), ::canic::Error>);
        let plan = build_access_plan(EndpointKind::Query, &args, &sig).unwrap();
        let stage = access_stage(&plan, &format_ident!("call"));
        let block: syn::Block = syn::parse2(quote!({ #stage })).unwrap();
        let mut reader = SelectedReader(Vec::new());
        syn::visit::Visit::visit_block(&mut reader, &block);
        assert_eq!(reader.0, vec![expected]);
    }
}

#[test]
fn application_startup_gate_precedes_boolean_guards_and_internal_dispatch() {
    for internal in [false, true] {
        let mut args = make_args(vec![AccessExprAst::Any(vec![
            AccessExprAst::Pred(AccessPredicateAst::Builtin(
                BuiltinPredicate::CallerIsController,
            )),
            AccessExprAst::Not(Box::new(AccessExprAst::Pred(AccessPredicateAst::Builtin(
                BuiltinPredicate::CallerIsSameCanister,
            )))),
        ])]);
        args.internal = internal;
        let function: ItemFn = syn::parse_quote!(
            fn guarded() -> Result<(), ::canic::Error> {
                Ok(())
            }
        );
        let expanded = expand(EndpointKind::Update, args, function).to_string();
        let compact = expanded.split_whitespace().collect::<String>();
        let mandatory = compact
            .find("eval_application_startup")
            .expect("mandatory startup admission");
        let dispatch = compact
            .rfind("__canic_impl_guarded(")
            .expect("application dispatch");
        assert!(mandatory < dispatch);
        assert_eq!(compact.matches("eval_application_startup").count(), 1);
        if let Some(predicate) = compact.find("caller::is_controller") {
            assert!(mandatory < predicate);
        }
    }
}

#[test]
fn wire_adapter_keeps_guards_and_checks_size_before_copy() {
    let parsed = crate::endpoint::parse::parse_args(quote!(
        requires(caller::is_controller()),
        on_access_denied = "reject",
        wire(id = 7, request = REQUEST, response = RESPONSE)
    ))
    .unwrap();
    let function: ItemFn = syn::parse_quote!(
        fn native(value: Principal) -> Principal {
            value
        }
    );
    let args =
        crate::endpoint::validate::validate(EndpointKind::Update, parsed, &function.sig).unwrap();
    let expansion = expand(EndpointKind::Update, args, function);
    syn::parse2::<syn::File>(expansion.clone()).unwrap();
    let text = expansion.to_string();
    for required in [
        "decode_request",
        "encode_response",
        "preflight_endpoint",
        "eval_application_startup",
        "register_update_limit",
        "in_tracking_executor_context",
        "msg_reject",
    ] {
        assert!(text.contains(required));
    }
    assert!(text.find("msg_arg_data_size").unwrap() < text.find("msg_arg_data_copy").unwrap());
    assert!(!text.contains("candid"));
    assert!(!text.contains("futures :: spawn"));
}

#[test]
fn wire_result_endpoint_selects_native_io_without_transport_rejection() {
    let parsed = crate::endpoint::parse::parse_args(quote!(
        requires(caller::is_controller()),
        wire(id = 9, request = REQUEST, response = RESPONSE)
    ))
    .unwrap();
    let function: ItemFn = syn::parse_quote!(
        fn native(value: u64) -> Result<u64, ::canic::Error> {
            Ok(value)
        }
    );
    let args =
        crate::endpoint::validate::validate(EndpointKind::Update, parsed, &function.sig).unwrap();
    let expansion = expand(EndpointKind::Update, args, function);
    syn::parse2::<syn::File>(expansion.clone()).unwrap();
    let text = expansion.to_string();
    assert!(text.contains("decode_request"));
    assert!(text.contains("encode_response"));
    assert!(text.contains("eval_access_selected"));
    assert!(!text.contains("candid"));
    assert!(!text.contains("msg_reject"));
}
