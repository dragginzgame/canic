use crate::endpoint::{EndpointKind, parse::AuthSpec, validate::ValidatedArgs};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Expr, ItemFn};

//
// ============================================================================
// expand — code generation only
// ============================================================================
//

pub fn expand(kind: EndpointKind, args: ValidatedArgs, mut func: ItemFn) -> TokenStream2 {
    let attrs = func.attrs.clone();
    let orig_sig = func.sig.clone();
    let orig_name = orig_sig.ident.clone();
    let vis = func.vis.clone();
    let inputs = orig_sig.inputs.clone();
    let output = orig_sig.output.clone();
    let asyncness = orig_sig.asyncness.is_some();
    let returns_fallible = returns_fallible(&orig_sig);

    let impl_name = format_ident!("__canic_impl_{}", orig_name);
    func.sig.ident = impl_name.clone();

    let cdk_attr = cdk_attr(kind, &args.forwarded);
    let dispatch = dispatch(kind, asyncness);

    let wrapper_sig = syn::Signature {
        ident: orig_name.clone(),
        inputs,
        output,
        ..orig_sig.clone()
    };

    let call_ident = format_ident!("__canic_call");
    let call_decl = call_decl(kind, &call_ident, &orig_name);

    let attempted = attempted(&call_ident);
    let guard = guard(kind, args.app_guard, &call_ident);
    let auth = auth(args.auth.as_ref(), &call_ident);
    let env = env(&args.env, &call_ident);
    let rule = rule(&args.rules, &call_ident);

    let call_args = match extract_args(&orig_sig) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error(),
    };

    let dispatch_call = dispatch_call(asyncness, dispatch, &call_ident, impl_name, &call_args);
    let completion = completion(&call_ident, returns_fallible, dispatch_call);

    quote! {
        #(#attrs)*
        #cdk_attr
        #vis #wrapper_sig {
            #call_decl
            #attempted
            #guard
            #auth
            #env
            #rule
            #completion
        }

        #func
    }
}

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

fn call_decl(kind: EndpointKind, call_ident: &syn::Ident, orig_name: &syn::Ident) -> TokenStream2 {
    let call_kind = match kind {
        EndpointKind::Query => {
            quote!(::canic::__internal::core::ids::EndpointCallKind::Query)
        }
        EndpointKind::Update => {
            quote!(::canic::__internal::core::ids::EndpointCallKind::Update)
        }
    };

    quote! {
        let #call_ident = ::canic::__internal::core::ids::EndpointCall {
            endpoint: ::canic::__internal::core::ids::EndpointId::new(stringify!(#orig_name)),
            kind: #call_kind,
        };
    }
}

fn record_access_denied(call: &syn::Ident, kind: TokenStream2) -> TokenStream2 {
    quote! {
        ::canic::__internal::core::access::metrics::AccessMetrics::increment(#call, #kind);
    }
}

fn attempted(call: &syn::Ident) -> TokenStream2 {
    quote! {
        ::canic::__internal::core::access::metrics::EndpointAttemptMetrics::increment_attempted(#call);
    }
}

fn guard(kind: EndpointKind, enabled: bool, call: &syn::Ident) -> TokenStream2 {
    if !enabled {
        return quote!();
    }

    let metric = record_access_denied(
        call,
        quote!(::canic::__internal::core::ids::AccessMetricKind::Guard),
    );

    match kind {
        EndpointKind::Query => quote! {
            if let Err(err) = ::canic::__internal::core::access::guard::guard_app_query() {
                #metric
                return Err(::canic::Error::from(err).into());
            }
        },
        EndpointKind::Update => quote! {
            if let Err(err) = ::canic::__internal::core::access::guard::guard_app_update() {
                #metric
                return Err(::canic::Error::from(err).into());
            }
        },
    }
}

fn auth(auth: Option<&AuthSpec>, call: &syn::Ident) -> TokenStream2 {
    let metric = record_access_denied(
        call,
        quote!(::canic::__internal::core::ids::AccessMetricKind::Auth),
    );

    match auth {
        Some(AuthSpec::Any(rules)) => quote! {
            if let Err(err) = ::canic::auth_require_any!(#(#rules),*) {
                #metric
                return Err(::canic::Error::from(err).into());
            }
        },
        Some(AuthSpec::All(rules)) => quote! {
            if let Err(err) = ::canic::auth_require_all!(#(#rules),*) {
                #metric
                return Err(::canic::Error::from(err).into());
            }
        },
        None => quote!(),
    }
}

fn rule(rules: &[Expr], call: &syn::Ident) -> TokenStream2 {
    if rules.is_empty() {
        return quote!();
    }

    let metric = record_access_denied(
        call,
        quote!(::canic::__internal::core::ids::AccessMetricKind::Rule),
    );

    let checks = rules.iter().map(|expr| {
        quote! {
            if let Err(err) = #expr().await {
                #metric
                return Err(::canic::Error::from(err).into());
            }
        }
    });

    quote!(#(#checks)*)
}

fn env(envs: &[Expr], call: &syn::Ident) -> TokenStream2 {
    if envs.is_empty() {
        return quote!();
    }

    let metric = record_access_denied(
        call,
        quote!(::canic::__internal::core::ids::AccessMetricKind::Env),
    );

    let checks = envs.iter().map(|expr| {
        quote! {
            if let Err(err) = #expr().await {
                #metric
                return Err(::canic::Error::from(err).into());
            }
        }
    });

    quote!(#(#checks)*)
}

fn dispatch_call(
    asyncness: bool,
    dispatch: TokenStream2,
    call: &syn::Ident,
    impl_name: syn::Ident,
    call_args: &[TokenStream2],
) -> TokenStream2 {
    if asyncness {
        quote! {
            #dispatch(#call, || async move {
                #impl_name(#(#call_args),*).await
            }).await
        }
    } else {
        quote! {
            #dispatch(#call, || {
                #impl_name(#(#call_args),*)
            })
        }
    }
}

fn completion(
    call: &syn::Ident,
    returns_fallible: bool,
    dispatch_call: TokenStream2,
) -> TokenStream2 {
    let result_metrics = if returns_fallible {
        quote! {
            if out.is_ok() {
                ::canic::__internal::core::access::metrics::EndpointResultMetrics::increment_ok(#call);
            } else {
                ::canic::__internal::core::access::metrics::EndpointResultMetrics::increment_err(#call);
            }
        }
    } else {
        quote!()
    };

    quote! {
        {
            let out = #dispatch_call;
            ::canic::__internal::core::access::metrics::EndpointAttemptMetrics::increment_completed(#call);
            #result_metrics
            out
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
