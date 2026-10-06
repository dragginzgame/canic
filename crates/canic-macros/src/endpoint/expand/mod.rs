mod access;

use crate::adapter::{EntryKind, RawAdapter};
use crate::endpoint::{EndpointKind, parse::QueryMode, returns_fallible, validate::ValidatedArgs};
use access::{AccessPlan, access_stage, build_access_plan, requires_decoded_auth_argument};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{ItemFn, Signature};

//
// ============================================================================
// expand - code generation only
// ============================================================================
//

pub(super) fn expand(kind: EndpointKind, args: ValidatedArgs, mut func: ItemFn) -> TokenStream2 {
    let attrs = func.attrs.clone();
    let orig_sig = func.sig.clone();
    let orig_name = orig_sig.ident.clone();
    let vis = func.vis.clone();
    let impl_async = orig_sig.asyncness.is_some();

    let access_plan = match build_access_plan(kind, &args, &orig_sig) {
        Ok(plan) => plan,
        Err(err) => return err.to_compile_error(),
    };
    if !returns_fallible(&orig_sig)
        && !matches!(access_plan, AccessPlan::None)
        && !args.reject_access
    {
        let message = "access-gated endpoints must return Result<_, Error> or select on_access_denied = \"reject\"";
        return syn::Error::new_spanned(&orig_sig.ident, message).to_compile_error();
    }

    let wrapper_async = impl_async || access_plan.requires_async();
    let uses_raw_adapter =
        args.payload_max_bytes.is_some() || args.decode.is_some() || args.reject_access;

    let impl_name = format_ident!("__canic_impl_{}", orig_name);
    func.sig.ident = impl_name.clone();

    if requires_decoded_auth_argument(&args.requires)
        && let Some(first_arg_ident) = first_typed_arg_ident(&orig_sig)
    {
        // Proof-bearing auth predicates decode ingress arg0 before dispatch.
        let keepalive: syn::Stmt = syn::parse_quote!(let _ = &#first_arg_ident;);
        func.block.stmts.insert(0, keepalive);
    }

    let cdk_attr = if uses_raw_adapter {
        quote!()
    } else {
        cdk_attr(kind, &args.forwarded)
    };
    let candid_attr = uses_raw_adapter.then(|| {
        let method_name = args
            .export_name
            .clone()
            .unwrap_or_else(|| syn::LitStr::new(&orig_name.to_string(), orig_name.span()));
        entry_kind(kind, args.query_mode).candid_attribute(&method_name)
    });
    let payload_registration = payload_registration(kind, &args, &orig_name);

    let wrapper_sig = wrapper_signature(&orig_sig, wrapper_async, args.reject_access);

    let call_ident = format_ident!("__canic_call");
    let exported_method = exported_method(&args, &orig_name);
    let call_decl = call_decl(kind, args.query_mode, &call_ident, &exported_method);
    let preflight = preflight(&orig_name);

    let startup_denial = if returns_fallible(&orig_sig) || args.reject_access {
        quote!(return Err(err.into());)
    } else {
        quote!(::canic::__internal::cdk::trap(format!("application startup rejected: {err}"));)
    };
    let startup_stage = quote! {
        #[cfg(target_arch = "wasm32")]
        if let Err(err) = ::canic::__internal::core::access::expr::eval_application_startup(#call_ident) {
            #startup_denial
        }
    };

    let access_stage = access_stage(&access_plan, &call_ident);

    let call_args = match extract_args(&orig_sig) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error(),
    };

    let dispatch_call = dispatch_call(
        wrapper_async,
        impl_async,
        &call_ident,
        impl_name,
        &call_args,
    );
    let raw_adapter = if uses_raw_adapter {
        match raw_adapter(kind, &args, &orig_sig, wrapper_async) {
            Ok(adapter) => adapter,
            Err(err) => return err.to_compile_error(),
        }
    } else {
        quote!()
    };
    let dispatch_call = if args.reject_access {
        quote!(Ok({ #dispatch_call }))
    } else {
        dispatch_call
    };
    let async_context_lint = async_context_attribute(impl_async);

    quote! {
        #payload_registration

        #(#attrs)*
        #[expect(clippy::missing_const_for_fn, clippy::unnecessary_wraps)]
        #async_context_lint
        #cdk_attr
        #vis #wrapper_sig {
            #call_decl
            #preflight(#call_ident);
            #startup_stage
            #access_stage
            #dispatch_call
        }

        #[expect(clippy::missing_const_for_fn, clippy::unnecessary_wraps)]
        #candid_attr
        #func

        #raw_adapter
    }
}

// Async instrumentation is deliberately local to the single-threaded IC executor.
fn async_context_attribute(impl_async: bool) -> Option<TokenStream2> {
    impl_async.then(|| quote! {
        #[expect(
            clippy::future_not_send,
            reason = "IC endpoint instrumentation retains invocation state on one canister thread"
        )]
    })
}

fn wrapper_signature(original: &Signature, asynchronous: bool, reject_access: bool) -> Signature {
    let mut signature = original.clone();
    signature.asyncness = asynchronous.then(syn::token::Async::default);
    if reject_access {
        let ty = match &original.output {
            syn::ReturnType::Default => quote!(()),
            syn::ReturnType::Type(_, ty) => quote!(#ty),
        };
        signature.output = syn::parse_quote!(-> ::core::result::Result<#ty, ::canic::Error>);
    }
    signature
}

fn raw_adapter(
    kind: EndpointKind,
    args: &ValidatedArgs,
    signature: &Signature,
    wrapper_async: bool,
) -> syn::Result<TokenStream2> {
    RawAdapter {
        signature,
        kind: entry_kind(kind, args.query_mode),
        method: args
            .export_name
            .as_ref()
            .map_or_else(|| signature.ident.to_string(), syn::LitStr::value),
        decode: args.decode.as_ref(),
        max_bytes: args.payload_max_bytes.as_ref(),
        reject_access: args.reject_access,
        wrapper_async,
    }
    .expand()
}

const fn entry_kind(kind: EndpointKind, mode: QueryMode) -> EntryKind {
    match (kind, mode) {
        (EndpointKind::Update, _) => EntryKind::Update,
        (EndpointKind::Query, QueryMode::Plain) => EntryKind::Query,
        (EndpointKind::Query, QueryMode::Composite) => EntryKind::CompositeQuery,
    }
}

//
// ============================================================================
// helpers
// ============================================================================
//

fn preflight(name: &syn::Ident) -> TokenStream2 {
    if matches!(
        name.to_string().as_str(),
        "canic_wasm_store_chunk"
            | "canic_wasm_store_publish_chunk"
            | "canic_wasm_store_fixture_chunk"
            | "canic_wasm_store_publish_fixture"
    ) {
        quote!(::canic::__internal::core::dispatch::preflight_store_data_endpoint)
    } else {
        quote!(::canic::__internal::core::dispatch::preflight_endpoint)
    }
}

fn payload_registration(
    kind: EndpointKind,
    args: &ValidatedArgs,
    name: &syn::Ident,
) -> TokenStream2 {
    if !matches!(kind, EndpointKind::Update)
        || (args.payload_max_bytes.is_none() && args.decode.is_none())
    {
        return quote!();
    }

    let register_name = format_ident!("__canic_register_payload_limit_{}", name);
    let ctor_name = format_ident!("__canic_ctor_payload_limit_{}", name);
    let method_name = if let Some(name) = &args.export_name {
        quote!(#name)
    } else {
        quote!(stringify!(#name))
    };
    let max_bytes = args.decode.as_ref().map_or_else(
        || args.payload_max_bytes.clone().expect("explicit byte limit"),
        |limits| quote!((#limits).max_bytes),
    );

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
    call: &syn::Ident,
    impl_name: syn::Ident,
    args: &[TokenStream2],
) -> TokenStream2 {
    if wrapper_async && impl_async {
        quote! {
            ::canic::__internal::core::dispatch::measure_endpoint_async(
                #call, async { #impl_name(#(#args),*).await }
            ).await
        }
    } else {
        quote! {
            ::canic::__internal::core::dispatch::measure_endpoint(
                #call, || #impl_name(#(#args),*)
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

fn cdk_attr(kind: EndpointKind, forwarded: &[TokenStream2]) -> TokenStream2 {
    match kind {
        EndpointKind::Query => {
            if forwarded.is_empty() {
                quote!(#[::canic::__internal::cdk::query])
            } else {
                quote!(#[::canic::__internal::cdk::query(#(#forwarded),*)])
            }
        }
        EndpointKind::Update => {
            if forwarded.is_empty() {
                quote!(#[::canic::__internal::cdk::update])
            } else {
                quote!(#[::canic::__internal::cdk::update(#(#forwarded),*)])
            }
        }
    }
}

#[cfg(test)]
mod tests;
