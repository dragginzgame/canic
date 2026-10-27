//! Parse explicit Wire method identity and independent request/reply budgets.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Meta, Token, parse::Parser, punctuated::Punctuated};

/// One endpoint's native Wire contract selection.
#[derive(Debug)]
#[expect(
    clippy::redundant_pub_crate,
    reason = "shared by sibling macro adapter modules"
)]
pub(crate) struct WireArgs {
    pub id: TokenStream,
    pub request: TokenStream,
    pub response: TokenStream,
}

pub(super) fn parse(list: &syn::MetaList) -> syn::Result<WireArgs> {
    let fields = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(list.tokens.clone())?;
    let mut id = None;
    let mut request = None;
    let mut response = None;
    for field in fields {
        let Meta::NameValue(field) = field else {
            return Err(syn::Error::new_spanned(
                field,
                "expected id, request or response assignment",
            ));
        };
        if field.path.is_ident("id") && id.is_none() {
            let mut expression = &field.value;
            while let syn::Expr::Group(group) = expression {
                expression = &group.expr;
            }
            id = Some(match expression {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(value),
                    ..
                }) => {
                    let value = value.base10_parse::<u64>()?;
                    quote!(#value)
                }
                syn::Expr::Path(path) => quote!(#path),
                _ => {
                    return Err(syn::Error::new_spanned(
                        expression,
                        "Wire id must be a u64 literal or constant path",
                    ));
                }
            });
        } else if field.path.is_ident("request") && request.is_none() {
            let value = field.value;
            request = Some(quote!(#value));
        } else if field.path.is_ident("response") && response.is_none() {
            let value = field.value;
            response = Some(quote!(#value));
        } else {
            return Err(syn::Error::new_spanned(
                field,
                "unknown or duplicate Wire field",
            ));
        }
    }
    match (id, request, response) {
        (Some(id), Some(request), Some(response)) => Ok(WireArgs {
            id,
            request,
            response,
        }),
        _ => Err(syn::Error::new_spanned(
            list,
            "wire requires id, request and response",
        )),
    }
}
