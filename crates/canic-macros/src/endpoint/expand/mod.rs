mod access;

use crate::endpoint::{
    EndpointKind,
    parse::{AccessExprAst, AccessPredicateAst, BuiltinPredicate, CanisterRoleArg, QueryMode},
    validate::ValidatedArgs,
};
use access::{
    AccessPlan, access_stage, build_access_plan, exprs_have_attested_role_predicate,
    requires_authenticated,
};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{ItemFn, Signature};

//
// ============================================================================
// expand - code generation only
// ============================================================================
//

#[expect(clippy::default_trait_access)]
pub fn expand(kind: EndpointKind, args: ValidatedArgs, mut func: ItemFn) -> TokenStream2 {
    let attrs = func.attrs.clone();
    let orig_sig = func.sig.clone();
    let orig_name = orig_sig.ident.clone();
    let vis = func.vis.clone();
    let inputs = orig_sig.inputs.clone();
    let output = orig_sig.output.clone();
    let impl_async = orig_sig.asyncness.is_some();
    let returns_fallible = returns_fallible(&orig_sig);
    let protected_roles = match protected_internal_roles(&args.requires) {
        Ok(roles) => roles,
        Err(err) => return err.to_compile_error(),
    };
    let is_protected_internal = !protected_roles.is_empty();

    let access_plan = match build_access_plan(kind, &args, &orig_sig) {
        Ok(plan) => plan,
        Err(err) => return err.to_compile_error(),
    };
    if !returns_fallible && !matches!(access_plan, AccessPlan::None) {
        let message = "access-gated endpoints must return Result<_, Error> to avoid traps";
        return syn::Error::new_spanned(&orig_sig.ident, message).to_compile_error();
    }

    let wrapper_async = is_protected_internal || impl_async || access_plan.requires_async();

    let impl_name = format_ident!("__canic_impl_{}", orig_name);
    func.sig.ident = impl_name.clone();

    if requires_authenticated(&args.requires)
        && let Some(first_arg_ident) = first_typed_arg_ident(&orig_sig)
    {
        // authenticated([scope]) decodes ingress arg0 directly; keep the function arg lint-clean.
        let keepalive: syn::Stmt = syn::parse_quote!(let _ = &#first_arg_ident;);
        func.block.stmts.insert(0, keepalive);
    }

    let cdk_attr = cdk_attr(kind, &args.forwarded);
    let payload_registration = payload_registration(kind, &args, &orig_name);
    let dispatch_fn = dispatch(kind, wrapper_async);

    let wrapper_inputs = if is_protected_internal {
        Default::default()
    } else {
        inputs
    };

    let wrapper_sig = syn::Signature {
        ident: orig_name.clone(),
        asyncness: if wrapper_async {
            Some(Default::default())
        } else {
            None
        },
        inputs: wrapper_inputs,
        output,
        ..orig_sig.clone()
    };

    let call_ident = format_ident!("__canic_call");
    let exported_method = exported_method(&args, &orig_name);
    let call_decl = call_decl(kind, args.query_mode, &call_ident, &exported_method);

    let access_stage = if is_protected_internal {
        quote!()
    } else {
        access_stage(&access_plan, &call_ident)
    };

    let call_args = match extract_args(&orig_sig) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error(),
    };
    let protected_stage = if is_protected_internal {
        match protected_internal_stage(&orig_sig, &exported_method, &protected_roles) {
            Ok(stage) => stage,
            Err(err) => return err.to_compile_error(),
        }
    } else {
        quote!()
    };
    let protected_endpoint_descriptor = if is_protected_internal {
        protected_internal_endpoint_descriptor(&vis, &orig_name, &exported_method, &protected_roles)
    } else {
        quote!()
    };

    let dispatch_call = dispatch_call(
        wrapper_async,
        impl_async,
        dispatch_fn,
        &call_ident,
        impl_name,
        &call_args,
    );

    quote! {
        #payload_registration
        #protected_endpoint_descriptor

        #(#attrs)*
        #[expect(clippy::missing_const_for_fn, clippy::unnecessary_wraps)]
        #cdk_attr
        #vis #wrapper_sig {
            #call_decl
            #protected_stage
            #access_stage
            #dispatch_call
        }

        #[expect(clippy::missing_const_for_fn, clippy::unnecessary_wraps)]
        #func
    }
}

//
// ============================================================================
// helpers
// ============================================================================
//

fn returns_fallible(sig: &syn::Signature) -> bool {
    let syn::ReturnType::Type(_, ty) = &sig.output else {
        return false;
    };
    let syn::Type::Path(ty) = &**ty else {
        return false;
    };

    ty.path
        .segments
        .last()
        .is_some_and(|seg| seg.ident == "Result")
}

fn dispatch(kind: EndpointKind, asyncness: bool) -> TokenStream2 {
    match (kind, asyncness) {
        (EndpointKind::Query, false) => {
            quote!(::canic::__internal::core::dispatch::dispatch_query)
        }
        (EndpointKind::Query, true) => {
            quote!(::canic::__internal::core::dispatch::dispatch_query_async)
        }
        (EndpointKind::Update, false) => {
            quote!(::canic::__internal::core::dispatch::dispatch_update)
        }
        (EndpointKind::Update, true) => {
            quote!(::canic::__internal::core::dispatch::dispatch_update_async)
        }
    }
}

fn payload_registration(
    kind: EndpointKind,
    args: &ValidatedArgs,
    name: &syn::Ident,
) -> TokenStream2 {
    if !matches!(kind, EndpointKind::Update) {
        return quote!();
    }

    let register_name = format_ident!("__canic_register_payload_limit_{}", name);
    let ctor_name = format_ident!("__canic_ctor_payload_limit_{}", name);
    let method_name = if let Some(name) = &args.export_name {
        quote!(#name)
    } else {
        quote!(stringify!(#name))
    };
    let max_bytes = args.payload_max_bytes.clone().unwrap_or_else(|| {
        quote!(::canic::__internal::core::ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES)
    });

    quote! {
        const _: () = {
            fn #register_name() {
                ::canic::__internal::core::ingress::payload::register_update_limit(
                    #method_name,
                    #max_bytes,
                );
            }

            #[ ::canic::__internal::core::__reexports::ctor::ctor(
                unsafe,
                anonymous,
                crate_path = ::canic::__internal::core::__reexports::ctor
            ) ]
            fn #ctor_name() {
                #register_name();
            }
        };
    }
}

fn exported_method(args: &ValidatedArgs, name: &syn::Ident) -> TokenStream2 {
    if let Some(export_name) = &args.export_name {
        quote!(#export_name)
    } else {
        quote!(stringify!(#name))
    }
}

fn call_decl(
    kind: EndpointKind,
    query_mode: QueryMode,
    call: &syn::Ident,
    method_name: &TokenStream2,
) -> TokenStream2 {
    let call_kind = match (kind, query_mode) {
        (EndpointKind::Query, QueryMode::Composite) => {
            quote!(::canic::__internal::core::ids::EndpointCallKind::QueryComposite)
        }
        (EndpointKind::Query, QueryMode::Plain) => {
            quote!(::canic::__internal::core::ids::EndpointCallKind::Query)
        }
        (EndpointKind::Update, _) => {
            quote!(::canic::__internal::core::ids::EndpointCallKind::Update)
        }
    };

    quote! {
        let #call = ::canic::__internal::core::ids::EndpointCall {
            endpoint: ::canic::__internal::core::ids::EndpointId::new(#method_name),
            kind: #call_kind,
        };
    }
}

fn protected_internal_roles(requires: &[AccessExprAst]) -> syn::Result<Vec<TokenStream2>> {
    if !exprs_have_attested_role_predicate(requires) {
        return Ok(Vec::new());
    }

    let mut roles = Vec::new();
    for expr in requires {
        collect_protected_role_expr(expr, &mut roles)?;
    }
    Ok(roles)
}

fn collect_protected_role_expr(
    expr: &AccessExprAst,
    roles: &mut Vec<TokenStream2>,
) -> syn::Result<()> {
    match expr {
        AccessExprAst::All(exprs) => {
            for expr in exprs {
                collect_protected_role_expr(expr, roles)?;
            }
            Ok(())
        }
        AccessExprAst::Pred(AccessPredicateAst::Builtin(BuiltinPredicate::CallerHasRole {
            role,
        })) => {
            roles.push(role_to_tokens(role));
            Ok(())
        }
        AccessExprAst::Pred(AccessPredicateAst::Builtin(BuiltinPredicate::CallerHasAnyRole {
            roles: any_roles,
        })) => {
            roles.extend(any_roles.iter().map(role_to_tokens));
            Ok(())
        }
        AccessExprAst::Any(_) | AccessExprAst::Not(_) | AccessExprAst::Pred(_) => {
            Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "caller::has_role(...) protected endpoints may only combine attested role predicates",
            ))
        }
    }
}

fn role_to_tokens(role: &CanisterRoleArg) -> TokenStream2 {
    match role {
        CanisterRoleArg::Literal(role) => {
            quote!(::canic::__internal::core::ids::CanisterRole::new(#role))
        }
        CanisterRoleArg::Expr(role) => quote!(#role),
    }
}

fn first_typed_arg_ident(sig: &Signature) -> Option<syn::Ident> {
    let first = sig.inputs.first()?;
    let syn::FnArg::Typed(pat) = first else {
        return None;
    };
    let syn::Pat::Ident(id) = &*pat.pat else {
        return None;
    };
    Some(id.ident.clone())
}

//
// ============================================================================
// dispatch + completion
// ============================================================================
//

fn dispatch_call(
    wrapper_async: bool,
    impl_async: bool,
    dispatch: TokenStream2,
    call: &syn::Ident,
    impl_name: syn::Ident,
    args: &[TokenStream2],
) -> TokenStream2 {
    if wrapper_async {
        if impl_async {
            quote! {
                #dispatch(#call, || async move {
                    #impl_name(#(#args),*).await
                }).await
            }
        } else {
            quote! {
                #dispatch(#call, || async move {
                    #impl_name(#(#args),*)
                }).await
            }
        }
    } else {
        quote! {
            #dispatch(#call, || {
                #impl_name(#(#args),*)
            })
        }
    }
}

fn protected_internal_stage(
    sig: &syn::Signature,
    exported_method: &TokenStream2,
    roles: &[TokenStream2],
) -> syn::Result<TokenStream2> {
    let typed_args = extract_typed_args(sig)?;
    let arg_idents: Vec<_> = typed_args.iter().map(|(ident, _)| ident).collect();
    let arg_types: Vec<_> = typed_args.iter().map(|(_, ty)| ty).collect();

    let decode_stage = if typed_args.is_empty() {
        quote! {
            if let Err(_err) = ::canic::cdk::candid::decode_args::<()>(&__canic_envelope.args) {
                return Err(::canic::Error::new(
                    ::canic::dto::error::ErrorCode::InternalRpcMalformed,
                    "malformed Canic internal call envelope".to_string(),
                ).into());
            }
        }
    } else {
        quote! {
            let (#(#arg_idents,)*): (#(#arg_types,)*) =
                match ::canic::cdk::candid::decode_args::<(#(#arg_types,)*)>(&__canic_envelope.args) {
                    Ok(args) => args,
                    Err(_err) => {
                        return Err(::canic::Error::new(
                            ::canic::dto::error::ErrorCode::InternalRpcMalformed,
                            "malformed Canic internal call envelope".to_string(),
                        ).into());
                    }
                };
        }
    };

    Ok(quote! {
        let __canic_raw_args = ::canic::cdk::api::msg_arg_data();
        let __canic_envelope: ::canic::dto::auth::CanicInternalCallEnvelopeV1 =
            match ::canic::cdk::candid::decode_one(&__canic_raw_args) {
                Ok(envelope) => envelope,
                Err(_err) => {
                    return Err(::canic::Error::new(
                        ::canic::dto::error::ErrorCode::InternalRpcMalformed,
                        "malformed Canic internal call envelope".to_string(),
                    ).into());
                }
            };

        let __canic_method = #exported_method;
        if __canic_envelope.version != 1
            || __canic_envelope.header.target_canister != ::canic::cdk::api::canister_self()
            || __canic_envelope.header.target_method != __canic_method
        {
            return Err(::canic::Error::new(
                ::canic::dto::error::ErrorCode::InternalRpcMalformed,
                "invalid Canic internal call envelope".to_string(),
            ).into());
        }

        let __canic_accepted_roles = [#(#roles),*];
        if let Err(err) = ::canic::__internal::core::api::auth::AuthApi::verify_internal_invocation_proof(
            &__canic_envelope.proof,
            __canic_method,
            &__canic_accepted_roles,
        ).await {
            return Err(err.into());
        }

        #decode_stage
    })
}

fn protected_internal_endpoint_descriptor(
    vis: &syn::Visibility,
    name: &syn::Ident,
    exported_method: &TokenStream2,
    roles: &[TokenStream2],
) -> TokenStream2 {
    let descriptor_name = format_ident!("canic_internal_endpoint_{}", name);
    quote! {
        #vis fn #descriptor_name() -> ::canic::api::ic::ProtectedInternalEndpoint {
            ::canic::api::ic::ProtectedInternalEndpoint::new(
                #exported_method,
                [#(#roles),*],
            )
        }
    }
}

fn extract_args(sig: &syn::Signature) -> syn::Result<Vec<TokenStream2>> {
    let mut out = Vec::new();
    for input in &sig.inputs {
        match input {
            syn::FnArg::Typed(pat) => match &*pat.pat {
                syn::Pat::Ident(id) => out.push(quote!(#id)),
                _ => {
                    return Err(syn::Error::new_spanned(
                        &pat.pat,
                        "destructuring parameters not supported",
                    ));
                }
            },
            syn::FnArg::Receiver(r) => {
                return Err(syn::Error::new_spanned(
                    r,
                    "`self` not supported in canic endpoints",
                ));
            }
        }
    }
    Ok(out)
}

fn extract_typed_args(sig: &syn::Signature) -> syn::Result<Vec<(syn::Ident, Box<syn::Type>)>> {
    let mut out = Vec::new();
    for input in &sig.inputs {
        match input {
            syn::FnArg::Typed(pat) => match &*pat.pat {
                syn::Pat::Ident(id) => out.push((id.ident.clone(), pat.ty.clone())),
                _ => {
                    return Err(syn::Error::new_spanned(
                        &pat.pat,
                        "destructuring parameters not supported",
                    ));
                }
            },
            syn::FnArg::Receiver(r) => {
                return Err(syn::Error::new_spanned(
                    r,
                    "`self` not supported in canic endpoints",
                ));
            }
        }
    }
    Ok(out)
}

fn cdk_attr(kind: EndpointKind, forwarded: &[TokenStream2]) -> TokenStream2 {
    match kind {
        EndpointKind::Query => {
            if forwarded.is_empty() {
                quote!(#[::canic::cdk::query])
            } else {
                quote!(#[::canic::cdk::query(#(#forwarded),*)])
            }
        }
        EndpointKind::Update => {
            if forwarded.is_empty() {
                quote!(#[::canic::cdk::update])
            } else {
                quote!(#[::canic::cdk::update(#(#forwarded),*)])
            }
        }
    }
}

#[cfg(test)]
mod tests;
