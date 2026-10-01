//! Entrypoint decoding for the facade's lifecycle macros; lifecycle work stays synchronous.

use crate::adapter::{EntryKind, RawAdapter};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemFn, Meta, Token, parse::Parser, punctuated::Punctuated};

#[cfg(test)]
mod tests;

#[expect(
    clippy::redundant_pub_crate,
    reason = "called by the parent proc-macro entrypoint"
)]
pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let function: ItemFn = syn::parse2(item)?;
    let options = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(attr)?;
    let mut options = options.into_iter();
    let kind = match options.next() {
        Some(Meta::Path(path)) if path.is_ident("init") => EntryKind::Init,
        Some(Meta::Path(path)) if path.is_ident("post_upgrade") => EntryKind::PostUpgrade,
        other => {
            return Err(syn::Error::new_spanned(
                quote!(#other),
                "expected init or post_upgrade",
            ));
        }
    };
    let decode = match options.next() {
        Some(Meta::NameValue(value)) if value.path.is_ident("decode") => {
            let value = value.value;
            Some(quote!(#value))
        }
        None => None,
        Some(other) => return Err(syn::Error::new_spanned(other, "expected decode = LIMITS")),
    };
    if let Some(other) = options.next() {
        return Err(syn::Error::new_spanned(
            other,
            "duplicate lifecycle decoder",
        ));
    }
    if decode.is_none() {
        let attribute = if matches!(kind, EntryKind::Init) {
            quote!(#[::canic::__internal::cdk::init])
        } else {
            quote!(#[::canic::__internal::cdk::post_upgrade])
        };
        return Ok(quote!(#attribute #function));
    }
    if function.sig.asyncness.is_some() || !matches!(function.sig.output, syn::ReturnType::Default)
    {
        return Err(syn::Error::new_spanned(
            &function.sig,
            "lifecycle entrypoints must be synchronous and return no value",
        ));
    }
    let attribute = kind.candid_attribute(&syn::LitStr::new("init", function.sig.ident.span()));
    let adapter = RawAdapter {
        signature: &function.sig,
        kind,
        method: function.sig.ident.to_string(),
        decode: decode.as_ref(),
        max_bytes: None,
        reject_access: false,
        wrapper_async: false,
    }
    .expand()?;
    Ok(quote! { #attribute #function #adapter })
}
