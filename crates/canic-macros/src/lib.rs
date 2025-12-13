//! Canic proc macros.
//!
//! These macros provide thin, opinionated wrappers around IC CDK endpoint
//! attributes (`#[query]`, `#[update]`), routed through `canic::cdk::*` to
//! ensure a stable import surface.
//!
//! Responsibilities handled here:
//! - Attribute forwarding to the IC CDK
//! - Optional app-level guard injection
//! - Uniform dispatch (sync/async, query/update)
//! - Preserving the user-facing function signature

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Expr, ItemFn, Meta, Token, parse::Parser, parse_macro_input, punctuated::Punctuated};

// -----------------------------------------------------------------------------
// Macros
// -----------------------------------------------------------------------------

#[proc_macro_attribute]
pub fn canic_query(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_endpoint(EndpointKind::Query, attr, item)
}

#[proc_macro_attribute]
pub fn canic_update(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_endpoint(EndpointKind::Update, attr, item)
}

// -----------------------------------------------------------------------------
// Internal types
// -----------------------------------------------------------------------------

/// Distinguishes which IC CDK attribute and dispatch path to use.
#[derive(Clone, Copy)]
enum EndpointKind {
    Query,
    Update,
}

#[derive(Clone)]
enum AuthSpec {
    Any(Vec<Expr>),
    All(Vec<Expr>),
}

struct ParsedArgs {
    forwarded: Vec<TokenStream2>,
    app_guard: bool,
    user_guard: bool,
    auth: Option<AuthSpec>,
}

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

/// Returns `true` if the function's return type is `Result<_, _>`.
///
/// This is required when injecting app guards so that we can use `?`
/// without altering the user-visible signature.
fn return_type_supports_app_guard(sig: &syn::Signature) -> bool {
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

/// Parse and classify arguments passed to `#[canic_query(...)]` or
/// `#[canic_update(...)]`.
///
/// Supported special arguments:
/// - `app` or `app = true|false`
/// - `auth_any(rule, ...)`
/// - `auth_all(rule, ...)`
///
/// All other arguments are forwarded verbatim to the underlying
/// IC CDK attribute.
fn parse_forwarded_args(attr: TokenStream2, orig_name: &syn::Ident) -> syn::Result<ParsedArgs> {
    // Attempt structured parsing first so we can reason about arguments.
    let Ok(metas) = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(attr.clone()) else {
        // If parsing fails but tokens are present, fall back to raw forwarding.
        // This keeps compatibility with CDK arguments we don't explicitly model.
        if attr.is_empty() {
            return Ok(ParsedArgs {
                forwarded: Vec::new(),
                app_guard: false,
                user_guard: false,
                auth: None,
            });
        }

        return Ok(ParsedArgs {
            forwarded: vec![attr],
            app_guard: false,
            user_guard: false,
            auth: None,
        });
    };

    let mut app_guard = None::<bool>;
    let mut user_guard = None::<Meta>;
    let mut auth = None::<AuthSpec>;
    let mut forwarded = Vec::<TokenStream2>::new();

    for meta in metas {
        match meta {
            // `#[canic_* (app)]`
            Meta::Path(path) if path.is_ident("app") => {
                let new = true;

                if let Some(prev) = app_guard
                    && prev != new
                {
                    return Err(syn::Error::new_spanned(
                        orig_name,
                        "conflicting `app` values",
                    ));
                }

                app_guard = Some(new);
            }

            // `#[canic_* (app = true|false)]`
            Meta::NameValue(ref nv) if nv.path.is_ident("app") => match &nv.value {
                syn::Expr::Lit(expr) => match &expr.lit {
                    syn::Lit::Bool(b) => {
                        let new = b.value;

                        if let Some(prev) = app_guard
                            && prev != new
                        {
                            return Err(syn::Error::new_spanned(
                                orig_name,
                                "conflicting `app` values",
                            ));
                        }

                        app_guard = Some(new);
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            nv,
                            "expected `app` or `app = true|false`",
                        ));
                    }
                },
                _ => {
                    return Err(syn::Error::new_spanned(
                        nv,
                        "expected `app` or `app = true|false`",
                    ));
                }
            },

            // `#[canic_* (auth_any(rule, ...))]`
            // Backwards compat: also accept `any(...)`.
            Meta::List(list) if list.path.is_ident("auth_any") || list.path.is_ident("any") => {
                if auth.is_some() {
                    return Err(syn::Error::new_spanned(
                        list,
                        "conflicting authorization composition; use only one of `auth_any(...)` or `auth_all(...)`",
                    ));
                }

                let rules = Punctuated::<Expr, Token![,]>::parse_terminated
                    .parse2(list.tokens.clone())?
                    .into_iter()
                    .collect::<Vec<_>>();

                if rules.is_empty() {
                    return Err(syn::Error::new_spanned(
                        list,
                        "`auth_any(...)` requires at least one rule",
                    ));
                }

                auth = Some(AuthSpec::Any(rules));
            }

            // `#[canic_* (auth_all(rule, ...))]`
            // Backwards compat: also accept `all(...)`.
            Meta::List(list) if list.path.is_ident("auth_all") || list.path.is_ident("all") => {
                if auth.is_some() {
                    return Err(syn::Error::new_spanned(
                        list,
                        "conflicting authorization composition; use only one of `auth_any(...)` or `auth_all(...)`",
                    ));
                }

                let rules = Punctuated::<Expr, Token![,]>::parse_terminated
                    .parse2(list.tokens.clone())?
                    .into_iter()
                    .collect::<Vec<_>>();

                if rules.is_empty() {
                    return Err(syn::Error::new_spanned(
                        list,
                        "`auth_all(...)` requires at least one rule",
                    ));
                }

                auth = Some(AuthSpec::All(rules));
            }

            // Explicit `guard = ...` is forwarded unchanged, but tracked
            // so we can prevent invalid combinations.
            Meta::NameValue(nv) if nv.path.is_ident("guard") => {
                user_guard = Some(Meta::NameValue(nv.clone()));
                forwarded.push(quote!(#nv));
            }

            // Any other arguments are forwarded verbatim to the CDK.
            _ => forwarded.push(quote!(#meta)),
        }
    }

    let app_guard = app_guard.unwrap_or(false);

    // Prevent ambiguous guard semantics.
    if app_guard && user_guard.is_some() {
        return Err(syn::Error::new_spanned(
            orig_name,
            "`app` cannot be combined with an explicit `guard = ...`",
        ));
    }

    // Prevent ordering bypass via CDK guards when auth is present.
    if auth.is_some() && user_guard.is_some() {
        return Err(syn::Error::new_spanned(
            orig_name,
            "`auth_any(...)` / `auth_all(...)` cannot be combined with `guard = ...`",
        ));
    }

    Ok(ParsedArgs {
        forwarded,
        app_guard,
        user_guard: user_guard.is_some(),
        auth,
    })
}

fn extract_arg_idents(sig: &syn::Signature) -> syn::Result<Vec<TokenStream2>> {
    let mut args = Vec::new();

    for input in &sig.inputs {
        match input {
            syn::FnArg::Typed(pat) => match &*pat.pat {
                syn::Pat::Ident(ident) => args.push(quote!(#ident)),
                _ => {
                    return Err(syn::Error::new_spanned(
                        &pat.pat,
                        "canic endpoints do not support destructuring parameters",
                    ));
                }
            },
            syn::FnArg::Receiver(receiver) => {
                return Err(syn::Error::new_spanned(
                    receiver,
                    "canic endpoints must not take `self`",
                ));
            }
        }
    }

    Ok(args)
}

// -----------------------------------------------------------------------------
// Core expansion logic
// -----------------------------------------------------------------------------

fn expand_endpoint(kind: EndpointKind, attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the user function.
    let mut func: ItemFn = parse_macro_input!(item as ItemFn);

    // Clone signature components before mutating the function.
    let orig_sig = func.sig.clone();
    let orig_name = orig_sig.ident.clone();
    let vis = func.vis.clone();
    let inputs = orig_sig.inputs.clone();
    let output = orig_sig.output.clone();
    let asyncness = orig_sig.asyncness.is_some();

    // Internal implementation function name.
    let impl_name = format_ident!("__canic_impl_{}", orig_name);

    // Rename the user function to the internal implementation.
    func.sig.ident = impl_name.clone();

    // Parse attribute arguments.
    let parsed = match parse_forwarded_args(attr.into(), &orig_name) {
        Ok(v) => v,
        Err(err) => return err.to_compile_error().into(),
    };
    let forwarded = parsed.forwarded;
    let app_guard = parsed.app_guard;
    let auth = parsed.auth;

    // Select the IC CDK attribute.
    let cdk_attr = match kind {
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
    };

    // Select the appropriate dispatch helper.
    let dispatch = match (kind, asyncness) {
        (EndpointKind::Query, false) => quote!(::canic::core::dispatch::dispatch_query),
        (EndpointKind::Query, true) => quote!(::canic::core::dispatch::dispatch_query_async),
        (EndpointKind::Update, false) => quote!(::canic::core::dispatch::dispatch_update),
        (EndpointKind::Update, true) => quote!(::canic::core::dispatch::dispatch_update_async),
    };

    // Reconstruct the public wrapper signature.
    let wrapper_sig = syn::Signature {
        ident: orig_name.clone(),
        inputs,
        output,
        ..orig_sig.clone()
    };

    // Label used for dispatch / perf instrumentation.
    // NOTE: This currently allocates; consider switching to a `LitStr`
    // to guarantee a `'static` label and avoid runtime formatting.
    let label = orig_name.to_string();

    // Optional app guard injection.
    let guard_stmt = if app_guard {
        if !return_type_supports_app_guard(&orig_sig) {
            return syn::Error::new_spanned(
                &orig_sig.output,
                "`app` endpoints must return `Result<_, _>` so the guard can use `?`",
            )
            .to_compile_error()
            .into();
        }

        match kind {
            EndpointKind::Query => quote!(::canic::core::guard::guard_query()?;),
            EndpointKind::Update => quote!(::canic::core::guard::guard_update()?;),
        }
    } else {
        quote!()
    };

    // Optional auth injection.
    let auth_stmt = match auth {
        Some(AuthSpec::Any(rules)) => {
            if !asyncness {
                return syn::Error::new_spanned(
                    &orig_sig.ident,
                    "`auth_any(...)` authorization requires `async fn` endpoints",
                )
                .to_compile_error()
                .into();
            }

            if !return_type_supports_app_guard(&orig_sig) {
                return syn::Error::new_spanned(
                    &orig_sig.output,
                    "authorized endpoints must return `Result<_, _>` so authorization can use `?`",
                )
                .to_compile_error()
                .into();
            }

            quote!(::canic::core::auth_require_any!(#(#rules),*)?;)
        }
        Some(AuthSpec::All(rules)) => {
            if !asyncness {
                return syn::Error::new_spanned(
                    &orig_sig.ident,
                    "`auth_all(...)` authorization requires `async fn` endpoints",
                )
                .to_compile_error()
                .into();
            }

            if !return_type_supports_app_guard(&orig_sig) {
                return syn::Error::new_spanned(
                    &orig_sig.output,
                    "authorized endpoints must return `Result<_, _>` so authorization can use `?`",
                )
                .to_compile_error()
                .into();
            }

            quote!(::canic::core::auth_require_all!(#(#rules),*)?;)
        }
        None => quote!(),
    };

    // Policy stage placeholder (always emitted to guarantee ordering).
    let policy_stmt = quote!(::canic::core::policy::policy_noop(););

    // Extract argument identifiers for forwarding.
    let args = match extract_arg_idents(&orig_sig) {
        Ok(v) => v,
        Err(err) => return err.to_compile_error().into(),
    };

    // Construct the call into the internal implementation.
    let call = if asyncness {
        quote! {
            #dispatch(#label, || async move {
                #impl_name(#(#args),*).await
            }).await
        }
    } else {
        quote! {
            #dispatch(#label, || {
                #impl_name(#(#args),*)
            })
        }
    };

    // Final expansion.
    quote! {
        #cdk_attr
        #vis #wrapper_sig {
            #guard_stmt
            #auth_stmt
            #policy_stmt
            #call
        }

        #func
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    fn parse_ok(attr: TokenStream2) -> ParsedArgs {
        parse_forwarded_args(attr, &syn::Ident::new("f", proc_macro2::Span::call_site()))
            .expect("parse args")
    }

    #[test]
    fn parses_any_rules() {
        let parsed = parse_ok(quote!(auth_any(foo, bar)));
        assert!(!parsed.app_guard);
        assert!(parsed.forwarded.is_empty());
        assert!(matches!(parsed.auth, Some(AuthSpec::Any(rules)) if rules.len() == 2));
    }

    #[test]
    fn rejects_any_empty() {
        let err = parse_forwarded_args(
            quote!(auth_any()),
            &syn::Ident::new("f", proc_macro2::Span::call_site()),
        )
        .unwrap_err();
        assert!(err.to_string().contains("requires at least one rule"));
    }

    #[test]
    fn rejects_mixed_any_all() {
        let err = parse_forwarded_args(
            quote!(auth_any(foo), auth_all(bar)),
            &syn::Ident::new("f", proc_macro2::Span::call_site()),
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("conflicting authorization composition")
        );
    }

    #[test]
    fn rejects_auth_with_cdk_guard() {
        let err = parse_forwarded_args(
            quote!(auth_any(foo), guard = my_guard),
            &syn::Ident::new("f", proc_macro2::Span::call_site()),
        )
        .unwrap_err();
        assert!(err.to_string().contains("cannot be combined"));
    }

    #[test]
    fn accepts_legacy_any_all() {
        let parsed = parse_ok(quote!(any(foo)));
        assert!(matches!(parsed.auth, Some(AuthSpec::Any(rules)) if rules.len() == 1));

        let parsed = parse_ok(quote!(all(foo)));
        assert!(matches!(parsed.auth, Some(AuthSpec::All(rules)) if rules.len() == 1));
    }

    #[test]
    fn rejects_destructuring_params() {
        let sig: syn::Signature = syn::parse2(quote!(
            fn f((a, b): (u8, u8)) {}
        ))
        .unwrap();
        let err = extract_arg_idents(&sig).unwrap_err();
        assert!(err.to_string().contains("do not support destructuring"));
    }

    #[test]
    fn rejects_self_receiver() {
        let sig: syn::Signature = syn::parse2(quote!(
            fn f(&self) {}
        ))
        .unwrap();
        let err = extract_arg_idents(&sig).unwrap_err();
        assert!(err.to_string().contains("must not take `self`"));
    }
}
