use crate::endpoint::{
    EndpointKind,
    parse::{AuthSymbol, EnvSymbol, GuardSymbol, RuleSymbol},
    validate::ValidatedArgs,
};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::ItemFn;

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
    let dispatch_fn = dispatch(kind, asyncness);

    let wrapper_sig = syn::Signature {
        ident: orig_name.clone(),
        inputs,
        output,
        ..orig_sig.clone()
    };

    let call_ident = format_ident!("__canic_call");
    let call_decl = call_decl(kind, &call_ident, &orig_name);

    let attempted = attempted(&call_ident);

    let guard_stage = guard_stage(kind, &args.guard, &call_ident);
    let auth_stage = auth_stage(&args.auth, &call_ident);
    let env_stage = env_stage(&args.env, &call_ident);
    let rule_stage = rule_stage(&args.rules, &call_ident);

    let call_args = match extract_args(&orig_sig) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error(),
    };

    let dispatch_call = dispatch_call(asyncness, dispatch_fn, &call_ident, impl_name, &call_args);
    let completion = completion(&call_ident, returns_fallible, dispatch_call);

    quote! {
        #(#attrs)*
        #cdk_attr
        #vis #wrapper_sig {
            #call_decl
            #attempted
            #guard_stage
            #auth_stage
            #env_stage
            #rule_stage
            #completion
        }

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

fn call_decl(kind: EndpointKind, call: &syn::Ident, name: &syn::Ident) -> TokenStream2 {
    let call_kind = match kind {
        EndpointKind::Query => {
            quote!(::canic::__internal::core::ids::EndpointCallKind::Query)
        }
        EndpointKind::Update => {
            quote!(::canic::__internal::core::ids::EndpointCallKind::Update)
        }
    };

    quote! {
        let #call = ::canic::__internal::core::ids::EndpointCall {
            endpoint: ::canic::__internal::core::ids::EndpointId::new(stringify!(#name)),
            kind: #call_kind,
        };
    }
}

fn attempted(call: &syn::Ident) -> TokenStream2 {
    quote! {
        ::canic::__internal::core::access::metrics::EndpointAttemptMetrics::increment_attempted(#call);
    }
}

fn record_denied(call: &syn::Ident, kind: TokenStream2) -> TokenStream2 {
    quote! {
        ::canic::__internal::core::access::metrics::AccessMetrics::increment(#call, #kind);
    }
}

//
// ============================================================================
// stages
// ============================================================================
//

fn guard_stage(kind: EndpointKind, guard: &[GuardSymbol], call: &syn::Ident) -> TokenStream2 {
    if guard.is_empty() {
        return quote!();
    }

    let metric = record_denied(
        call,
        quote!(::canic::__internal::core::ids::AccessMetricKind::Guard),
    );

    let checks = guard.iter().map(|sym| {
        let call = guard_call(sym, kind);
        quote! {
            if let Err(err) = #call {
                #metric
                return Err(err.into());
            }
        }
    });

    quote!(#(#checks)*)
}

fn auth_stage(auth: &[AuthSymbol], call: &syn::Ident) -> TokenStream2 {
    if auth.is_empty() {
        return quote!();
    }

    let metric = record_denied(
        call,
        quote!(::canic::__internal::core::ids::AccessMetricKind::Auth),
    );

    let caller = format_ident!("__canic_caller");

    let checks = auth.iter().map(|sym| {
        let call = auth_call(sym, &caller);
        quote! {
            if let Err(err) = #call.await {
                #metric
                return Err(err.into());
            }
        }
    });

    quote! {
        let #caller = ::canic::cdk::api::msg_caller();
        #(#checks)*
    }
}

fn env_stage(env: &[EnvSymbol], call: &syn::Ident) -> TokenStream2 {
    if env.is_empty() {
        return quote!();
    }

    let metric = record_denied(
        call,
        quote!(::canic::__internal::core::ids::AccessMetricKind::Env),
    );

    let checks = env.iter().map(|sym| {
        let call = env_call(sym);
        quote! {
            if let Err(err) = #call.await {
                #metric
                return Err(err.into());
            }
        }
    });

    quote!(#(#checks)*)
}

fn rule_stage(rules: &[RuleSymbol], call: &syn::Ident) -> TokenStream2 {
    if rules.is_empty() {
        return quote!();
    }

    let metric = record_denied(
        call,
        quote!(::canic::__internal::core::ids::AccessMetricKind::Rule),
    );

    let checks = rules.iter().map(|sym| {
        let call = rule_call(sym);
        quote! {
            if let Err(err) = #call {
                #metric
                return Err(err.into());
            }
        }
    });

    quote!(#(#checks)*)
}

//
// ============================================================================
// symbol → API mapping
// ============================================================================
//

fn auth_call(sym: &AuthSymbol, caller: &syn::Ident) -> TokenStream2 {
    match sym {
        AuthSymbol::CallerIsController => {
            quote!(::canic::api::access::AuthAccessApi::is_controller(#caller))
        }
        AuthSymbol::CallerIsParent => {
            quote!(::canic::api::access::AuthAccessApi::is_parent(#caller))
        }
        AuthSymbol::CallerIsChild => {
            quote!(::canic::api::access::AuthAccessApi::is_child(#caller))
        }
        AuthSymbol::CallerIsRoot => {
            quote!(::canic::api::access::AuthAccessApi::is_root(#caller))
        }
        AuthSymbol::CallerIsSameCanister => {
            quote!(::canic::api::access::AuthAccessApi::is_same_canister(#caller))
        }
        AuthSymbol::CallerIsRegisteredToSubnet => {
            quote!(::canic::api::access::AuthAccessApi::is_registered_to_subnet(#caller))
        }
        AuthSymbol::CallerIsWhitelisted => {
            quote!(::canic::api::access::AuthAccessApi::is_whitelisted(#caller))
        }
    }
}

fn guard_call(sym: &GuardSymbol, kind: EndpointKind) -> TokenStream2 {
    match (sym, kind) {
        (GuardSymbol::AppIsLive, EndpointKind::Query) => {
            quote!(::canic::api::access::GuardAccessApi::guard_app_query())
        }
        (GuardSymbol::AppIsLive, EndpointKind::Update) => {
            quote!(::canic::api::access::GuardAccessApi::guard_app_update())
        }
    }
}

fn env_call(sym: &EnvSymbol) -> TokenStream2 {
    match sym {
        EnvSymbol::SelfIsPrimeSubnet => {
            quote!(::canic::api::access::EnvAccessApi::is_prime_subnet())
        }
        EnvSymbol::SelfIsPrimeRoot => {
            quote!(::canic::api::access::EnvAccessApi::is_prime_root())
        }
    }
}

fn rule_call(sym: &RuleSymbol) -> TokenStream2 {
    match sym {
        RuleSymbol::BuildIcOnly => {
            quote!(::canic::api::access::RuleAccessApi::require_ic())
        }
        RuleSymbol::BuildLocalOnly => {
            quote!(::canic::api::access::RuleAccessApi::require_local())
        }
    }
}

//
// ============================================================================
// dispatch + completion
// ============================================================================
//

fn dispatch_call(
    asyncness: bool,
    dispatch: TokenStream2,
    call: &syn::Ident,
    impl_name: syn::Ident,
    args: &[TokenStream2],
) -> TokenStream2 {
    if asyncness {
        quote! {
            #dispatch(#call, || async move {
                #impl_name(#(#args),*).await
            }).await
        }
    } else {
        quote! {
            #dispatch(#call, || {
                #impl_name(#(#args),*)
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
