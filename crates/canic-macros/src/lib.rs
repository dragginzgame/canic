//! Canic proc macros.
//!
//! Thin, opinionated wrappers around IC CDK endpoint attributes
//! (`#[query]`, `#[update]`), routed through `canic::cdk::*`.
//!
//! Pipeline enforced by generated wrappers:
//!   guard → auth → rule → dispatch

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Expr, ItemFn, Meta, Token, parse::Parser, parse_macro_input, punctuated::Punctuated};

//
// ============================================================================
// Public entry points
// ============================================================================
//

#[proc_macro_attribute]
pub fn canic_query(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_entry(EndpointKind::Query, attr, item)
}

#[proc_macro_attribute]
pub fn canic_update(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_entry(EndpointKind::Update, attr, item)
}

//
// ============================================================================
// Shared internal types
// ============================================================================
//

#[derive(Clone, Copy)]
enum EndpointKind {
    Query,
    Update,
}

//
// ============================================================================
// parse — attribute grammar only
// ============================================================================
//

mod parse {
    use super::*;

    #[derive(Clone, Debug)]
    pub enum AuthSpec {
        Any(Vec<Expr>),
        All(Vec<Expr>),
    }

    #[derive(Debug)]
    pub struct ParsedArgs {
        pub forwarded: Vec<TokenStream2>,
        pub app_guard: bool,
        pub user_guard: bool,
        pub auth: Option<AuthSpec>,
        pub rules: Vec<Expr>,
    }

    pub fn parse_args(attr: TokenStream2) -> syn::Result<ParsedArgs> {
        let Ok(metas) = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(attr.clone()) else {
            // If the attr doesn't parse as Meta list, fall back to forwarding raw tokens to the CDK.
            // This preserves compatibility with CDK syntax we don't model.
            if attr.is_empty() {
                return Ok(empty());
            }

            return Ok(ParsedArgs {
                forwarded: vec![attr],
                ..empty()
            });
        };

        let mut forwarded = Vec::new();
        let mut app_guard = false;
        let mut user_guard = false;
        let mut auth = None::<AuthSpec>;
        let mut rules = Vec::<Expr>::new();

        for meta in metas {
            match meta {
                // guard(...)
                //
                // Canic-specific guard stage. Top-level `app` is no longer accepted.
                Meta::List(list) if list.path.is_ident("guard") => {
                    let inner = Punctuated::<Meta, Token![,]>::parse_terminated
                        .parse2(list.tokens.clone())?
                        .into_iter()
                        .collect::<Vec<_>>();

                    if inner.is_empty() {
                        return Err(syn::Error::new_spanned(
                            list,
                            "`guard(...)` expects at least one argument (e.g., `guard(app)`)",
                        ));
                    }

                    // For now, support only guard(app). You can widen this later.
                    for item in inner {
                        match item {
                            Meta::Path(p) if p.is_ident("app") => {
                                app_guard = true;
                            }
                            other => {
                                return Err(syn::Error::new_spanned(
                                    other,
                                    "only `guard(app)` is supported",
                                ));
                            }
                        }
                    }
                }

                // auth_any(...)
                Meta::List(list) if list.path.is_ident("auth_any") => {
                    if auth.is_some() {
                        return Err(conflicting_auth(&list));
                    }
                    let rules = parse_rules(&list)?;
                    auth = Some(AuthSpec::Any(rules));
                }

                // auth_all(...)
                Meta::List(list) if list.path.is_ident("auth_all") => {
                    if auth.is_some() {
                        return Err(conflicting_auth(&list));
                    }
                    let rules = parse_rules(&list)?;
                    auth = Some(AuthSpec::All(rules));
                }

                // rule(...)
                //
                // Parse as Expr so you can do rule(local_only()), rule(max_rounds(rounds, 10_000)), etc.
                Meta::List(list) if list.path.is_ident("rule") => {
                    let parsed = Punctuated::<Expr, Token![,]>::parse_terminated
                        .parse2(list.tokens.clone())?
                        .into_iter()
                        .collect::<Vec<_>>();

                    if parsed.is_empty() {
                        return Err(syn::Error::new_spanned(
                            list,
                            "`rule(...)` expects at least one rule expression",
                        ));
                    }

                    rules.extend(parsed);
                }

                // explicit CDK guard = ...
                //
                // We still forward it, but track that it exists so validation can ban combinations.
                Meta::NameValue(nv) if nv.path.is_ident("guard") => {
                    user_guard = true;
                    forwarded.push(quote!(#nv));
                }

                // Everything else is forwarded to the CDK attribute unchanged.
                _ => forwarded.push(quote!(#meta)),
            }
        }

        Ok(ParsedArgs {
            forwarded,
            app_guard,
            user_guard,
            auth,
            rules,
        })
    }
    const fn empty() -> ParsedArgs {
        ParsedArgs {
            forwarded: Vec::new(),
            app_guard: false,
            user_guard: false,
            auth: None,
            rules: Vec::new(),
        }
    }

    fn parse_rules(list: &syn::MetaList) -> syn::Result<Vec<Expr>> {
        let rules = Punctuated::<Expr, Token![,]>::parse_terminated
            .parse2(list.tokens.clone())?
            .into_iter()
            .collect::<Vec<_>>();

        if rules.is_empty() {
            return Err(syn::Error::new_spanned(
                list,
                "authorization requires at least one rule",
            ));
        }

        Ok(rules)
    }

    fn conflicting_auth(list: &syn::MetaList) -> syn::Error {
        syn::Error::new_spanned(list, "conflicting authorization composition")
    }
}

//
// ============================================================================
// validate — semantic constraints
// ============================================================================
//

mod validate {
    use super::*;
    use parse::{AuthSpec, ParsedArgs};

    pub struct ValidatedArgs {
        pub forwarded: Vec<TokenStream2>,
        pub app_guard: bool,
        pub auth: Option<AuthSpec>,
        pub rules: Vec<Expr>,
    }

    pub fn validate(
        parsed: ParsedArgs,
        sig: &syn::Signature,
        asyncness: bool,
    ) -> syn::Result<ValidatedArgs> {
        if parsed.app_guard && parsed.user_guard {
            return Err(syn::Error::new_spanned(
                &sig.ident,
                "`app` cannot be combined with `guard = ...`",
            ));
        }

        if parsed.auth.is_some() && parsed.user_guard {
            return Err(syn::Error::new_spanned(
                &sig.ident,
                "authorization cannot be combined with `guard = ...`",
            ));
        }

        if parsed.auth.is_some() {
            if !asyncness {
                return Err(syn::Error::new_spanned(
                    &sig.ident,
                    "authorization requires `async fn`",
                ));
            }
            if !returns_result(sig) {
                return Err(syn::Error::new_spanned(
                    &sig.output,
                    "authorized endpoints must return `Result<_, From<canic::Error>>`",
                ));
            }
        }

        if parsed.app_guard && !returns_result(sig) {
            return Err(syn::Error::new_spanned(
                &sig.output,
                "`app` guard requires `Result<_, From<canic::Error>>`",
            ));
        }

        if !parsed.rules.is_empty() && !returns_result(sig) {
            return Err(syn::Error::new_spanned(
                &sig.output,
                "`rule(...)` requires `Result<_, From<canic::Error>>`",
            ));
        }

        Ok(ValidatedArgs {
            forwarded: parsed.forwarded,
            app_guard: parsed.app_guard,
            auth: parsed.auth,
            rules: parsed.rules,
        })
    }

    fn returns_result(sig: &syn::Signature) -> bool {
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
}

//
// ============================================================================
// expand — code generation only
// ============================================================================
//

mod expand {
    use super::*;
    use parse::AuthSpec;
    use validate::ValidatedArgs;

    pub fn expand(kind: EndpointKind, args: ValidatedArgs, mut func: ItemFn) -> TokenStream {
        let attrs = func.attrs.clone();
        let orig_sig = func.sig.clone();
        let orig_name = orig_sig.ident.clone();
        let vis = func.vis.clone();
        let inputs = orig_sig.inputs.clone();
        let output = orig_sig.output.clone();
        let asyncness = orig_sig.asyncness.is_some();
        let returns_result = returns_result(&orig_sig);

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
        let rule = rule(&args.rules, &call_ident);

        let call_args = match extract_args(&orig_sig) {
            Ok(v) => v,
            Err(e) => return e.to_compile_error().into(),
        };

        let dispatch_call = dispatch_call(asyncness, dispatch, &call_ident, impl_name, &call_args);
        let completion = completion(&call_ident, returns_result, dispatch_call);

        quote! {
           #(#attrs)*
           #cdk_attr
            #vis #wrapper_sig {
                #call_decl
                #attempted
                #guard
                #auth
                #rule
                #completion
            }

            #func
        }
        .into()
    }

    fn returns_result(sig: &syn::Signature) -> bool {
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
            (EndpointKind::Query, false) => quote!(::canic::core::dispatch::dispatch_query),
            (EndpointKind::Query, true) => quote!(::canic::core::dispatch::dispatch_query_async),
            (EndpointKind::Update, false) => quote!(::canic::core::dispatch::dispatch_update),
            (EndpointKind::Update, true) => quote!(::canic::core::dispatch::dispatch_update_async),
        }
    }

    fn call_decl(
        kind: EndpointKind,
        call_ident: &syn::Ident,
        orig_name: &syn::Ident,
    ) -> TokenStream2 {
        let call_kind = match kind {
            EndpointKind::Query => quote!(::canic::core::api::EndpointCallKind::Query),
            EndpointKind::Update => quote!(::canic::core::api::EndpointCallKind::Update),
        };

        quote! {
            let #call_ident = ::canic::core::api::EndpointCall {
                endpoint: ::canic::core::api::EndpointId::new(stringify!(#orig_name)),
                kind: #call_kind,
            };
        }
    }

    fn record_access_denied(call: &syn::Ident, kind: TokenStream2) -> TokenStream2 {
        quote! {
            ::canic::core::ops::runtime::metrics::AccessMetrics::increment(#call, #kind);
        }
    }

    fn attempted(call: &syn::Ident) -> TokenStream2 {
        quote! {
            ::canic::core::ops::runtime::metrics::EndpointAttemptMetrics::increment_attempted(#call);
        }
    }

    fn guard(kind: EndpointKind, enabled: bool, call: &syn::Ident) -> TokenStream2 {
        if !enabled {
            return quote!();
        }

        let metric = record_access_denied(
            call,
            quote!(::canic::core::ops::runtime::metrics::AccessMetricKind::Guard),
        );

        match kind {
            EndpointKind::Query => quote! {
                if let Err(err) = ::canic::core::access::guard::guard_app_query() {
                    #metric
                    return Err(err.into());
                }
            },
            EndpointKind::Update => quote! {
                if let Err(err) = ::canic::core::access::guard::guard_app_update() {
                    #metric
                    return Err(err.into());
                }
            },
        }
    }

    fn auth(auth: Option<&AuthSpec>, call: &syn::Ident) -> TokenStream2 {
        let metric = record_access_denied(
            call,
            quote!(::canic::core::ops::runtime::metrics::AccessMetricKind::Auth),
        );

        match auth {
            Some(AuthSpec::Any(rules)) => quote! {
                if let Err(err) = ::canic::core::auth_require_any!(#(#rules),*) {
                    #metric
                    return Err(err.into());
                }
            },
            Some(AuthSpec::All(rules)) => quote! {
                if let Err(err) = ::canic::core::auth_require_all!(#(#rules),*) {
                    #metric
                    return Err(err.into());
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
            quote!(::canic::core::ops::runtime::metrics::AccessMetricKind::Rule),
        );

        let checks = rules.iter().map(|expr| {
            quote! {
                if let Err(err) = #expr().await {
                    #metric
                    return Err(err.into());
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
        returns_result: bool,
        dispatch_call: TokenStream2,
    ) -> TokenStream2 {
        let result_metrics = if returns_result {
            quote! {
                if out.is_ok() {
                    ::canic::core::ops::runtime::metrics::EndpointResultMetrics::increment_ok(#call);
                } else {
                    ::canic::core::ops::runtime::metrics::EndpointResultMetrics::increment_err(#call);
                }
            }
        } else {
            quote!()
        };

        quote! {
            {
                let out = #dispatch_call;
                ::canic::core::ops::runtime::metrics::EndpointAttemptMetrics::increment_completed(#call);
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

//
// ============================================================================
// Entry dispatcher
// ============================================================================
//

fn expand_entry(kind: EndpointKind, attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let sig = func.sig.clone();
    let asyncness = sig.asyncness.is_some();

    let parsed = match parse::parse_args(attr.into()) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    let validated = match validate::validate(parsed, &sig, asyncness) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    expand::expand(kind, validated, func)
}
