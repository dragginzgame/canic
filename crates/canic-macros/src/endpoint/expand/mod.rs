mod access;

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

    let access_plan = match build_access_plan(kind, &args, &orig_sig) {
        Ok(plan) => plan,
        Err(err) => return err.to_compile_error(),
    };
    if !returns_fallible && !matches!(access_plan, AccessPlan::None) {
        let message = "access-gated endpoints must return Result<_, Error> to avoid traps";
        return syn::Error::new_spanned(&orig_sig.ident, message).to_compile_error();
    }

    let wrapper_async = impl_async || access_plan.requires_async();
    let uses_raw_update_adapter =
        matches!(kind, EndpointKind::Update) && args.payload_max_bytes.is_some();

    let impl_name = format_ident!("__canic_impl_{}", orig_name);
    func.sig.ident = impl_name.clone();

    if requires_decoded_auth_argument(&args.requires)
        && let Some(first_arg_ident) = first_typed_arg_ident(&orig_sig)
    {
        // Proof-bearing auth predicates decode ingress arg0 before dispatch.
        let keepalive: syn::Stmt = syn::parse_quote!(let _ = &#first_arg_ident;);
        func.block.stmts.insert(0, keepalive);
    }

    let cdk_attr = if uses_raw_update_adapter {
        quote!()
    } else {
        cdk_attr(kind, &args.forwarded)
    };
    let candid_attr = uses_raw_update_adapter.then(|| {
        let method_name = args
            .export_name
            .clone()
            .unwrap_or_else(|| syn::LitStr::new(&orig_name.to_string(), orig_name.span()));
        quote!(#[::candid::candid_method(update, rename = #method_name)])
    });
    let payload_registration = payload_registration(kind, &args, &orig_name);
    let dispatch_fn = dispatch(kind, wrapper_async);

    let wrapper_sig = syn::Signature {
        ident: orig_name.clone(),
        asyncness: if wrapper_async {
            Some(Default::default())
        } else {
            None
        },
        inputs,
        output,
        ..orig_sig.clone()
    };

    let call_ident = format_ident!("__canic_call");
    let exported_method = exported_method(&args, &orig_name);
    let call_decl = call_decl(kind, args.query_mode, &call_ident, &exported_method);

    let access_stage = access_stage(&access_plan, &call_ident);

    let call_args = match extract_args(&orig_sig) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error(),
    };

    let dispatch_call = dispatch_call(
        wrapper_async,
        impl_async,
        dispatch_fn,
        &call_ident,
        impl_name,
        &call_args,
    );
    let raw_update_adapter = if uses_raw_update_adapter {
        match raw_update_adapter(
            &orig_sig,
            &orig_name,
            args.export_name.as_ref(),
            args.payload_max_bytes
                .as_ref()
                .expect("raw update adapter requires explicit payload limit"),
            wrapper_async,
        ) {
            Ok(adapter) => adapter,
            Err(err) => return err.to_compile_error(),
        }
    } else {
        quote!()
    };

    quote! {
        #payload_registration

        #(#attrs)*
        #candid_attr
        #[expect(clippy::missing_const_for_fn, clippy::unnecessary_wraps)]
        #cdk_attr
        #vis #wrapper_sig {
            #call_decl
            ::canic::__internal::core::dispatch::preflight_endpoint(#call_ident);
            #access_stage
            #dispatch_call
        }

        #[expect(clippy::missing_const_for_fn, clippy::unnecessary_wraps)]
        #func

        #raw_update_adapter
    }
}

//
// ============================================================================
// helpers
// ============================================================================
//

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

fn raw_update_adapter(
    signature: &Signature,
    name: &syn::Ident,
    export_name: Option<&syn::LitStr>,
    max_bytes: &TokenStream2,
    wrapper_async: bool,
) -> syn::Result<TokenStream2> {
    let method_name = export_name.map_or_else(|| name.to_string(), syn::LitStr::value);
    let wasm_export = syn::LitStr::new(
        &format!("canister_update {method_name}"),
        proc_macro2::Span::call_site(),
    );
    let host_export = syn::LitStr::new(
        &format!("canister_update.{method_name}").replace(['-', '<', '>'], "_"),
        proc_macro2::Span::call_site(),
    );
    let adapter_name = format_ident!("__canic_raw_update_{}", name);

    let (names, types) = raw_update_arguments(signature)?;

    let decode = if names.is_empty() {
        quote!()
    } else {
        quote! {
            let mut __canic_decoder_config = ::candid::DecoderConfig::new();
            __canic_decoder_config.set_skipping_quota(10_000);
            let (#(#names,)*): (#(#types,)*) =
                ::candid::utils::decode_args_with_config(
                    &__canic_arg_bytes,
                    &__canic_decoder_config,
                )
                .unwrap_or_else(|error| {
                    ::canic::__internal::cdk::trap(format!(
                        "failed to decode update payload: {error}"
                    ))
                });
        }
    };

    let invoke = if wrapper_async {
        quote!(#name(#(#names),*).await)
    } else {
        quote!(#name(#(#names),*))
    };
    let encode = match &signature.output {
        syn::ReturnType::Default => {
            quote!(::candid::utils::encode_one(()))
        }
        syn::ReturnType::Type(_, ty) => match &**ty {
            syn::Type::Tuple(tuple) if tuple.elems.len() > 1 => {
                quote!(::candid::utils::encode_args(__canic_result))
            }
            _ => quote!(::candid::utils::encode_one(__canic_result)),
        },
    };
    let execute = quote! {
        let __canic_payload_len =
            ::canic::__internal::cdk::raw::msg_arg_data_size();
        if __canic_payload_len > #max_bytes {
            ::canic::__internal::cdk::trap(format!(
                "update payload is {__canic_payload_len} bytes; maximum is {}",
                #max_bytes,
            ));
        }
        let mut __canic_arg_bytes = vec![0_u8; __canic_payload_len];
        ::canic::__internal::cdk::raw::msg_arg_data_copy(
            &mut __canic_arg_bytes,
            0,
        );
        #decode
        let __canic_result = #invoke;
        let __canic_reply = #encode.unwrap_or_else(|error| {
            ::canic::__internal::cdk::trap(format!(
                "failed to encode update response: {error}"
            ))
        });
        ::canic::__internal::cdk::api::msg_reply(__canic_reply);
    };
    let body = if wrapper_async {
        quote! {
            ::canic::__internal::cdk::futures::internals::in_executor_context(|| {
                ::canic::__internal::cdk::futures::spawn(async {
                    #execute
                });
            });
        }
    } else {
        quote! {
            ::canic::__internal::cdk::futures::internals::in_executor_context(|| {
                #execute
            });
        }
    };

    Ok(quote! {
        #[cfg_attr(target_family = "wasm", unsafe(export_name = #wasm_export))]
        #[cfg_attr(not(target_family = "wasm"), unsafe(export_name = #host_export))]
        fn #adapter_name() {
            #body
        }
    })
}

fn raw_update_arguments(signature: &Signature) -> syn::Result<(Vec<syn::Ident>, Vec<syn::Type>)> {
    let mut names = Vec::new();
    let mut types = Vec::new();
    for input in &signature.inputs {
        let syn::FnArg::Typed(input) = input else {
            return Err(syn::Error::new_spanned(
                input,
                "`self` is unsupported on canic endpoints",
            ));
        };
        let syn::Pat::Ident(ident) = &*input.pat else {
            return Err(syn::Error::new_spanned(
                &input.pat,
                "destructuring parameters not supported",
            ));
        };
        names.push(ident.ident.clone());
        types.push(input.ty.as_ref().clone());
    }
    Ok((names, types))
}

#[cfg(test)]
mod tests;
