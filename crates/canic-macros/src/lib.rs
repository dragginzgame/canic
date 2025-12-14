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
use syn::{
    Expr, Ident, ItemFn, Meta, Path, Token, parse::Parser, parse_macro_input,
    punctuated::Punctuated,
};

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
        pub policies: Vec<Path>,
    }

    pub fn parse_args(attr: TokenStream2, orig_name: &Ident) -> syn::Result<ParsedArgs> {
        let Ok(metas) = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(attr.clone()) else {
            if attr.is_empty() {
                return Ok(empty());
            }
            return Ok(ParsedArgs {
                forwarded: vec![attr],
                ..empty()
            });
        };

        let mut forwarded = Vec::new();
        let mut app_guard = None::<bool>;
        let mut user_guard = false;
        let mut auth = None::<AuthSpec>;
        let mut policies = Vec::<Path>::new();

        for meta in metas {
            match meta {
                // app
                Meta::Path(path) if path.is_ident("app") => {
                    set_bool(&mut app_guard, true, orig_name)?;
                }

                // app = true|false
                Meta::NameValue(nv) if nv.path.is_ident("app") => {
                    let val = extract_bool(&nv)?;
                    set_bool(&mut app_guard, val, orig_name)?;
                }

                // guard(app) or guard(app = bool)
                Meta::List(list) if list.path.is_ident("guard") => {
                    let inner = Punctuated::<Meta, Token![,]>::parse_terminated
                        .parse2(list.tokens.clone())?
                        .into_iter()
                        .collect::<Vec<_>>();

                    if inner.len() != 1 {
                        return Err(syn::Error::new_spanned(
                            list,
                            "`guard(...)` expects exactly one argument",
                        ));
                    }

                    match &inner[0] {
                        Meta::Path(p) if p.is_ident("app") => {
                            set_bool(&mut app_guard, true, orig_name)?;
                        }
                        Meta::NameValue(nv) if nv.path.is_ident("app") => {
                            let val = extract_bool(nv)?;
                            set_bool(&mut app_guard, val, orig_name)?;
                        }
                        _ => {
                            return Err(syn::Error::new_spanned(
                                list,
                                "only `guard(app)` is supported",
                            ));
                        }
                    }
                }

                // auth_any / any
                Meta::List(list) if list.path.is_ident("auth_any") || list.path.is_ident("any") => {
                    if auth.is_some() {
                        return Err(conflicting_auth(&list));
                    }
                    let rules = parse_rules(&list)?;
                    auth = Some(AuthSpec::Any(rules));
                }

                // auth_all / all
                Meta::List(list) if list.path.is_ident("auth_all") || list.path.is_ident("all") => {
                    if auth.is_some() {
                        return Err(conflicting_auth(&list));
                    }
                    let rules = parse_rules(&list)?;
                    auth = Some(AuthSpec::All(rules));
                }

                // explicit CDK guard = ...
                Meta::NameValue(nv) if nv.path.is_ident("guard") => {
                    user_guard = true;
                    forwarded.push(quote!(#nv));
                }

                // policy(...)
                Meta::List(list) if list.path.is_ident("policy") => {
                    let parsed = Punctuated::<Path, Token![,]>::parse_terminated
                        .parse2(list.tokens.clone())?
                        .into_iter()
                        .collect::<Vec<_>>();

                    if parsed.is_empty() {
                        return Err(syn::Error::new_spanned(
                            list,
                            "`policy(...)` expects at least one policy",
                        ));
                    }

                    policies.extend(parsed);
                }

                _ => forwarded.push(quote!(#meta)),
            }
        }

        Ok(ParsedArgs {
            forwarded,
            app_guard: app_guard.unwrap_or(false),
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

    fn extract_bool(nv: &syn::MetaNameValue) -> syn::Result<bool> {
        match &nv.value {
            Expr::Lit(lit) => match &lit.lit {
                syn::Lit::Bool(b) => Ok(b.value),
                _ => Err(syn::Error::new_spanned(nv, "expected boolean")),
            },
            _ => Err(syn::Error::new_spanned(nv, "expected boolean")),
        }
    }

    fn set_bool(slot: &mut Option<bool>, val: bool, ident: &Ident) -> syn::Result<()> {
        if let Some(prev) = *slot
            && prev != val
        {
            return Err(syn::Error::new_spanned(ident, "conflicting `app` values"));
        }
        *slot = Some(val);

        Ok(())
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
        pub policies: Vec<Path>,
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
                    "authorized endpoints must return `Result<_, _>`",
                ));
            }
        }

        if parsed.app_guard && !returns_result(sig) {
            return Err(syn::Error::new_spanned(
                &sig.output,
                "`app` guard requires `Result<_, _>`",
            ));
        }

        if !parsed.policies.is_empty() && !returns_result(sig) {
            return Err(syn::Error::new_spanned(
                &sig.output,
                "`policy(...)` requires `Result<_, _>`",
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
        let orig_sig = func.sig.clone();
        let orig_name = orig_sig.ident.clone();
        let vis = func.vis.clone();
        let inputs = orig_sig.inputs.clone();
        let output = orig_sig.output.clone();
        let asyncness = orig_sig.asyncness.is_some();

        let impl_name = format_ident!("__canic_impl_{}", orig_name);
        func.sig.ident = impl_name.clone();

        let cdk_attr = cdk_attr(kind, &args.forwarded);

        let dispatch = match (kind, asyncness) {
            (EndpointKind::Query, false) => quote!(::canic::core::dispatch::dispatch_query),
            (EndpointKind::Query, true) => quote!(::canic::core::dispatch::dispatch_query_async),
            (EndpointKind::Update, false) => quote!(::canic::core::dispatch::dispatch_update),
            (EndpointKind::Update, true) => quote!(::canic::core::dispatch::dispatch_update_async),
        };

        let wrapper_sig = syn::Signature {
            ident: orig_name.clone(),
            inputs,
            output,
            ..orig_sig.clone()
        };

        let label = orig_name.to_string();

        let guard = if args.app_guard {
            match kind {
                EndpointKind::Query => {
                    quote!(::canic::core::guard::guard_app_query()?;)
                }
                EndpointKind::Update => {
                    quote!(::canic::core::guard::guard_app_update()?;)
                }
            }
        } else {
            quote!()
        };

        let auth = match args.auth {
            Some(AuthSpec::Any(rules)) => {
                quote!(::canic::core::auth_require_any!(#(#rules),*)?;)
            }
            Some(AuthSpec::All(rules)) => {
                quote!(::canic::core::auth_require_all!(#(#rules),*)?;)
            }
            None => quote!(),
        };

        let policy = if args.policies.is_empty() {
            quote!()
        } else {
            let checks = args.policies.iter().map(|policy_path| {
                if policy_path.leading_colon.is_none() && policy_path.segments.len() == 1 {
                    let ident = &policy_path.segments[0].ident;
                    quote!(::canic::core::policy::#ident()?;)
                } else {
                    quote!(#policy_path()?;)
                }
            });
            quote!(#(#checks)*)
        };

        let call_args = extract_args(&orig_sig).unwrap();

        let call = if asyncness {
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
        };

        quote! {
            #cdk_attr
            #vis #wrapper_sig {
                #guard
                #auth
                #policy
                #call
            }

            #func
        }
        .into()
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

    let parsed = match parse::parse_args(attr.into(), &sig.ident) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    let validated = match validate::validate(parsed, &sig, asyncness) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    expand::expand(kind, validated, func)
}
