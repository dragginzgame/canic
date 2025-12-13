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
use syn::{ItemFn, Meta, Token, parse::Parser, parse_macro_input, punctuated::Punctuated};

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
///
/// All other arguments are forwarded verbatim to the underlying
/// IC CDK attribute.
fn parse_forwarded_args(
    attr: TokenStream,
    orig_name: &syn::Ident,
) -> Result<(Vec<TokenStream2>, bool), TokenStream> {
    let attr: TokenStream2 = attr.into();

    // Attempt structured parsing first so we can reason about arguments.
    let Ok(metas) = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(attr.clone()) else {
        // If parsing fails but tokens are present, fall back to raw forwarding.
        // This keeps compatibility with CDK arguments we don't explicitly model.
        if attr.is_empty() {
            return Ok((Vec::new(), false));
        }

        return Ok((vec![attr], false));
    };

    let mut app_guard = None::<bool>;
    let mut user_guard = None::<Meta>;
    let mut forwarded = Vec::<TokenStream2>::new();

    for meta in metas {
        match meta {
            // `#[canic_* (app)]`
            Meta::Path(path) if path.is_ident("app") => {
                let new = true;

                if let Some(prev) = app_guard
                    && prev != new
                {
                    return Err(
                        syn::Error::new_spanned(orig_name, "conflicting `app` values")
                            .to_compile_error()
                            .into(),
                    );
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
                            )
                            .to_compile_error()
                            .into());
                        }

                        app_guard = Some(new);
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            nv,
                            "expected `app` or `app = true|false`",
                        )
                        .to_compile_error()
                        .into());
                    }
                },
                _ => {
                    return Err(syn::Error::new_spanned(
                        nv,
                        "expected `app` or `app = true|false`",
                    )
                    .to_compile_error()
                    .into());
                }
            },

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
        )
        .to_compile_error()
        .into());
    }

    Ok((forwarded, app_guard))
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
    let (forwarded, app_guard) = match parse_forwarded_args(attr, &orig_name) {
        Ok(v) => v,
        Err(err) => return err,
    };

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

    // Extract argument identifiers for forwarding.
    let args = orig_sig.inputs.iter().map(|arg| match arg {
        syn::FnArg::Typed(pat) => {
            let ident = match &*pat.pat {
                syn::Pat::Ident(ident) => &ident.ident,
                _ => panic!("canic endpoints do not support destructuring parameters"),
            };
            quote!(#ident)
        }
        syn::FnArg::Receiver(_) => {
            panic!("canic endpoints must not take `self`")
        }
    });

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
            #call
        }

        #func
    }
    .into()
}
