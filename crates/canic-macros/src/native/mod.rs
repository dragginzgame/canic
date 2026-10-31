//! Component entrypoints reuse Canic's export/executor owner without a framework runtime.

use crate::adapter::{EntryKind, export_entrypoint, raw_declaration};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Expr, Ident, ItemFn, LitStr, Path, Token,
    parse::{Parse, ParseStream},
};

struct Arguments {
    kind: EntryKind,
    name: LitStr,
    sdk: Path,
    max_bytes: Option<Expr>,
    requires_metrics_capability: bool,
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
        let max_bytes = if input.is_empty() {
            None
        } else {
            input.parse::<Token![,]>()?;
            let key: Ident = input.parse()?;
            if key != "max_bytes" {
                return Err(syn::Error::new_spanned(key, "expected max_bytes"));
            }
            input.parse::<Token![=]>()?;
            Some(input.parse()?)
        };
        let requires_metrics_capability = if input.is_empty() {
            false
        } else {
            input.parse::<Token![,]>()?;
            let key: Ident = input.parse()?;
            if key != "observability" {
                return Err(syn::Error::new_spanned(key, "expected observability"));
            }
            input.parse::<Token![=]>()?;
            let provider: Ident = input.parse()?;
            if provider != "metrics" {
                return Err(syn::Error::new_spanned(
                    provider,
                    "expected metrics provider",
                ));
            }
            true
        };
        Ok(Self {
            kind,
            name,
            sdk,
            max_bytes,
            requires_metrics_capability,
        })
    }
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "private expansion is called by the proc-macro root"
)]
pub(super) fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let Arguments {
        kind, name, sdk, ..
    } = syn::parse2(attr)?;
    let func: ItemFn = syn::parse2(item)?;
    let sig = &func.sig;
    crate::adapter::validate_raw_signature(sig, false)?;
    let declaration = raw_declaration(&kind, &name.value(), sig)?;
    let attrs = func.attrs;
    let body = func.block;
    let executor = if matches!(kind, EntryKind::Query) {
        quote!(#sdk::futures::internals::in_query_executor_context)
    } else {
        quote!(#sdk::futures::internals::in_executor_context)
    };
    let entry = export_entrypoint(
        &kind,
        &name.value(),
        &sig.ident,
        executor,
        quote! {
            #declaration
            #body
        },
    );
    Ok(quote!(#(#attrs)* #entry))
}

/// Use the maintained Canic raw adapter for a managed component's methods.
#[expect(clippy::redundant_pub_crate, reason = "called by the proc-macro root")]
pub(super) fn expand_managed(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let Arguments {
        kind,
        name,
        max_bytes,
        requires_metrics_capability,
        ..
    } = syn::parse2(attr)?;
    let kind = match kind {
        EntryKind::Query => crate::endpoint::EndpointKind::Query,
        EntryKind::Update => crate::endpoint::EndpointKind::Update,
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "managed components must use the host lifecycle participant",
            ));
        }
    };
    let max_bytes = max_bytes.ok_or_else(|| {
        syn::Error::new_spanned(&name, "managed component entry requires max_bytes")
    })?;
    let expanded = crate::endpoint::raw_entry(
        kind,
        quote!(public, name = #name, payload(max_bytes = #max_bytes)),
        item,
    )?;
    if requires_metrics_capability {
        // Keep exports, contract metadata and update-limit registration under
        // the same consumer-role capability. No handler or storage change.
        Ok(quote! {
            #[cfg(canic_capability_observability_metrics)]
            const _: () = { #expanded };
        })
    } else {
        Ok(expanded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_metrics_gate_encloses_exports_metadata_and_registration() {
        for kind in [quote!(query), quote!(update)] {
            let attr = quote!(#kind, name = "adapter_metrics", sdk = crate::sdk,
                max_bytes = LIMIT, observability = metrics);
            let item = quote!(
                fn endpoint() {
                    owner::serve();
                }
            );
            let expanded = expand_managed(attr.clone(), item.clone()).unwrap();
            let file: syn::File = syn::parse2(expanded).unwrap();
            let [syn::Item::Const(gate)] = file.items.as_slice() else {
                panic!("the complete expansion must be one gated item");
            };
            let condition: syn::Meta = gate.attrs[0].parse_args().unwrap();
            assert!(
                matches!(condition, syn::Meta::Path(path) if path.is_ident("canic_capability_observability_metrics"))
            );
            assert!(gate.attrs[0].path().is_ident("cfg"));
            let syn::Expr::Block(body) = gate.expr.as_ref() else {
                panic!("gate body");
            };
            assert!(body.block.stmts.iter().any(|stmt| matches!(stmt, syn::Stmt::Item(syn::Item::Fn(function)) if function.sig.ident == "__canic_raw_endpoint")));
            if kind.to_string() == "update" {
                assert!(
                    body.block
                        .stmts
                        .iter()
                        .any(|stmt| matches!(stmt, syn::Stmt::Item(syn::Item::Const(_))))
                );
            }
            // Standalone adapters use the same annotation without a Canic role policy.
            let standalone: syn::ItemFn = syn::parse2(expand(attr, item).unwrap()).unwrap();
            assert!(
                standalone
                    .attrs
                    .iter()
                    .all(|attr| !attr.path().is_ident("cfg"))
            );
        }
        assert!(
            syn::parse2::<Arguments>(quote!(
                query,
                name = "q",
                sdk = crate::sdk,
                max_bytes = LIMIT,
                observability = unknown
            ))
            .is_err()
        );
    }

    #[test]
    fn managed_components_use_framework_admission_and_require_request_bounds() {
        let item = quote!(
            fn endpoint() {
                owner::serve();
            }
        );
        for kind in [quote!(query), quote!(update)] {
            let output = expand_managed(
                quote!(#kind, name = "component", sdk = crate::sdk, max_bytes = LIMIT),
                item.clone(),
            )
            .unwrap()
            .to_string();
            assert!(output.contains("__canic_admission"));
            assert!(output.contains("msg_arg_data_size"));
            assert!(output.contains("owner :: serve"));
            syn::parse2::<syn::File>(output.parse().unwrap()).unwrap();
        }
        assert!(expand_managed(quote!(query, name = "q", sdk = crate::sdk), item.clone()).is_err());
        assert!(expand_managed(quote!(init, name = "", sdk = crate::sdk), item).is_err());
    }

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
