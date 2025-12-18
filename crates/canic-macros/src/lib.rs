//! Canic proc macros.
//!
//! Thin, opinionated wrappers around IC CDK endpoint attributes
//! (`#[query]`, `#[update]`), routed through `canic::cdk::*`.
//!
//! Pipeline enforced by generated wrappers:
//!   guard → auth → policy → dispatch

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
        pub policies: Vec<Expr>,
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
        let mut policies = Vec::<Expr>::new();

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

                // policy(...)
                //
                // Parse as Expr so you can do policy(local_only()), policy(max_rounds(rounds, 10_000)), etc.
                Meta::List(list) if list.path.is_ident("policy") => {
                    let parsed = Punctuated::<Expr, Token![,]>::parse_terminated
                        .parse2(list.tokens.clone())?
                        .into_iter()
                        .collect::<Vec<_>>();

                    if parsed.is_empty() {
                        return Err(syn::Error::new_spanned(
                            list,
                            "`policy(...)` expects at least one policy expression",
                        ));
                    }

                    policies.extend(parsed);
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
            policies,
        })
    }
    const fn empty() -> ParsedArgs {
        ParsedArgs {
            forwarded: Vec::new(),
            app_guard: false,
            user_guard: false,
            auth: None,
            policies: Vec::new(),
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
        pub policies: Vec<Expr>,
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

        if !parsed.policies.is_empty() && !returns_result(sig) {
            return Err(syn::Error::new_spanned(
                &sig.output,
                "`policy(...)` requires `Result<_, From<canic::Error>>`",
            ));
        }

        Ok(ValidatedArgs {
            forwarded: parsed.forwarded,
            app_guard: parsed.app_guard,
            auth: parsed.auth,
            policies: parsed.policies,
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

        let label = orig_name.to_string();

        let attempted = attempted(&label);
        let guard = guard(kind, args.app_guard, &label);
        let auth = auth(args.auth.as_ref(), &label);
        let policy = policy(&args.policies, &label);

        let call_args = match extract_args(&orig_sig) {
            Ok(v) => v,
            Err(e) => return e.to_compile_error().into(),
        };

        let call = call(asyncness, dispatch, &label, impl_name, &call_args);
        let completion = completion(&label, returns_result, call);

        quote! {
           #(#attrs)*
           #cdk_attr
            #vis #wrapper_sig {
                #attempted
                #guard
                #auth
                #policy
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

    fn record_access_denied(label: &String, kind: TokenStream2) -> TokenStream2 {
        quote! {
            ::canic::core::ops::runtime::metrics::AccessMetrics::increment(#label, #kind);
        }
    }

    fn attempted(label: &String) -> TokenStream2 {
        quote! {
            ::canic::core::ops::runtime::metrics::EndpointAttemptMetrics::increment_attempted(#label);
        }
    }

    fn guard(kind: EndpointKind, enabled: bool, label: &String) -> TokenStream2 {
        if !enabled {
            return quote!();
        }

        let metric = record_access_denied(
            label,
            quote!(::canic::core::ops::runtime::metrics::AccessMetricKind::Guard),
        );

        match kind {
            EndpointKind::Query => quote! {
                if let Err(err) = ::canic::core::guard::guard_app_query() {
                    #metric
                    return Err(err.into());
                }
            },
            EndpointKind::Update => quote! {
                if let Err(err) = ::canic::core::guard::guard_app_update() {
                    #metric
                    return Err(err.into());
                }
            },
        }
    }

    fn auth(auth: Option<&AuthSpec>, label: &String) -> TokenStream2 {
        let metric = record_access_denied(
            label,
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

    fn policy(policies: &[Expr], label: &String) -> TokenStream2 {
        if policies.is_empty() {
            return quote!();
        }

        let metric = record_access_denied(
            label,
            quote!(::canic::core::ops::runtime::metrics::AccessMetricKind::Policy),
        );

        let checks = policies.iter().map(|expr| {
            quote! {
                if let Err(err) = #expr().await {
                    #metric
                    return Err(err.into());
                }
            }
        });
        quote!(#(#checks)*)
    }

    fn call(
        asyncness: bool,
        dispatch: TokenStream2,
        label: &String,
        impl_name: syn::Ident,
        call_args: &[TokenStream2],
    ) -> TokenStream2 {
        if asyncness {
            quote! {
                #dispatch(#label, || async move {
                    #impl_name(#(#call_args),*).await
                }).await
            }
        } else {
            quote! {
                #dispatch(#label, || {
                    #impl_name(#(#call_args),*)
                })
            }
        }
    }

    fn completion(label: &String, returns_result: bool, call: TokenStream2) -> TokenStream2 {
        let result_metrics = if returns_result {
            quote! {
                if out.is_ok() {
                    ::canic::core::ops::runtime::metrics::EndpointResultMetrics::increment_ok(#label);
                } else {
                    ::canic::core::ops::runtime::metrics::EndpointResultMetrics::increment_err(#label);
                }
            }
        } else {
            quote!()
        };

        quote! {
            {
                let out = #call;
                ::canic::core::ops::runtime::metrics::EndpointAttemptMetrics::increment_completed(#label);
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
