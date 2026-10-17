//! Component entrypoints reuse Canic's export/executor owner without a framework runtime.

use crate::adapter::{EntryKind, entrypoint, raw_declaration};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Ident, ItemFn, LitStr, Path, Token,
    parse::{Parse, ParseStream},
};

struct Arguments {
    kind: EntryKind,
    name: LitStr,
    sdk: Path,
}

impl Parse for Arguments {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let kind: Ident = input.parse()?;
        let kind = match kind.to_string().as_str() {
            "query" => EntryKind::Query,
            "update" => EntryKind::Update,
            "init" => EntryKind::Init,
            "post_upgrade" => EntryKind::PostUpgrade,
            _ => {
                return Err(syn::Error::new_spanned(
                    kind,
                    "unsupported native entry kind",
                ));
            }
        };
        input.parse::<Token![,]>()?;
        let key: Ident = input.parse()?;
        if key != "name" {
            return Err(syn::Error::new_spanned(key, "expected name"));
        }
        input.parse::<Token![=]>()?;
        let name = input.parse()?;
        input.parse::<Token![,]>()?;
        let key: Ident = input.parse()?;
        if key != "sdk" {
            return Err(syn::Error::new_spanned(key, "expected sdk"));
        }
        input.parse::<Token![=]>()?;
        let sdk = input.parse()?;
        Ok(Self { kind, name, sdk })
    }
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "private expansion is called by the proc-macro root"
)]
pub(super) fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let Arguments { kind, name, sdk } = syn::parse2(attr)?;
    let func: ItemFn = syn::parse2(item)?;
    let sig = &func.sig;
    crate::adapter::validate_raw_signature(sig)?;
    let declaration = raw_declaration(&kind, &name.value(), sig)?;
    let attrs = func.attrs;
    let body = func.block;
    let entry = entrypoint(
        &kind,
        &name.value(),
        &sig.ident,
        quote!(#sdk),
        quote! {
            #declaration
            #body
        },
    );
    Ok(quote!(#(#attrs)* #entry))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_components_use_the_same_exact_export_and_executor_owner() {
        for (kind, export, executor) in [
            (
                "query",
                "canister_query icydb_query",
                "in_query_executor_context",
            ),
            (
                "update",
                "canister_update icydb_query",
                "in_executor_context",
            ),
            ("init", "canister_init", "in_executor_context"),
            (
                "post_upgrade",
                "canister_post_upgrade",
                "in_executor_context",
            ),
        ] {
            let attr = format!("{kind}, name = \"icydb_query\", sdk = crate::sdk")
                .parse()
                .unwrap();
            let output = expand(
                attr,
                quote!(
                    fn entry() {
                        crate::admitted_body();
                    }
                ),
            )
            .unwrap()
            .to_string();
            syn::parse2::<syn::File>(output.parse().unwrap()).unwrap();
            assert!(output.contains(export));
            assert!(output.contains(executor));
            assert!(output.contains("admitted_body"));
        }
    }

    #[test]
    fn native_entries_refuse_unhandled_signatures() {
        let attr = quote!(query, name = "q", sdk = crate::sdk);
        for item in [
            quote!(
                async fn q() {}
            ),
            quote!(
                fn q(x: u8) {}
            ),
            quote!(
                fn q() -> u8 {
                    1
                }
            ),
        ] {
            assert!(expand(attr.clone(), item).is_err());
        }
    }
}
