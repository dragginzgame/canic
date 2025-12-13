//! Canic proc macros.
//!
//! Currently these are small convenience wrappers around the IC CDK endpoint
//! attributes, routed through `canic::cdk::*` for a stable import surface.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{ItemFn, parse_macro_input};

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
// Attribute wrappers
// -----------------------------------------------------------------------------

///
/// EndpointKind
///

#[derive(Clone, Copy)]
enum EndpointKind {
    Query,
    Update,
}

fn expand_endpoint(kind: EndpointKind, attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_args: TokenStream2 = attr.into();
    let mut func: ItemFn = parse_macro_input!(item as ItemFn);

    // Clone signature parts BEFORE we mutate `func`
    let orig_sig = func.sig.clone();
    let orig_name = orig_sig.ident.clone();
    let vis = func.vis.clone(); // or borrow later after mutation
    let inputs = orig_sig.inputs.clone();
    let output = orig_sig.output.clone();
    let asyncness = orig_sig.asyncness.is_some();

    let impl_name = format_ident!("__canic_impl_{}", orig_name);

    // Rename user function → impl
    func.sig.ident = impl_name.clone();

    let cdk_attr = match kind {
        EndpointKind::Query => {
            if attr_args.is_empty() {
                quote!(#[::canic::cdk::query])
            } else {
                quote!(#[::canic::cdk::query(#attr_args)])
            }
        }
        EndpointKind::Update => {
            if attr_args.is_empty() {
                quote!(#[::canic::cdk::update])
            } else {
                quote!(#[::canic::cdk::update(#attr_args)])
            }
        }
    };

    let dispatch = match (kind, asyncness) {
        (EndpointKind::Query, false) => quote!(::canic::core::dispatch::dispatch_query),
        (EndpointKind::Query, true) => quote!(::canic::core::dispatch::dispatch_query_async),
        (EndpointKind::Update, false) => quote!(::canic::core::dispatch::dispatch_update),
        (EndpointKind::Update, true) => quote!(::canic::core::dispatch::dispatch_update_async),
    };

    // IMPORTANT NOTE:
    // `#inputs` is a full parameter list, not the argument list.
    // For now, keep this as a placeholder; see note below.
    let wrapper_sig = syn::Signature {
        ident: orig_name.clone(),
        inputs,
        output,
        ..orig_sig
    };

    let label = orig_name.to_string();

    let args = orig_sig.inputs.iter().map(|arg| match arg {
        syn::FnArg::Typed(pat) => {
            let ident = match &*pat.pat {
                syn::Pat::Ident(ident) => &ident.ident,
                _ => panic!("canic endpoints do not support destructuring parameters"),
            };
            quote!(#ident)
        }
        syn::FnArg::Receiver(_) => {
            panic!("canic endpoints must not take self")
        }
    });

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

    quote! {
        #cdk_attr
        #vis #wrapper_sig {
            #call
        }

        #func
    }
    .into()
}
