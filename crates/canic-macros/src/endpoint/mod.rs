//! Canic proc macros.
//!
//! Thin, opinionated wrappers around IC CDK endpoint attributes
//! (`#[query]`, `#[update]`), routed through hidden Canic macro plumbing.
//!
//! The macro pipeline is strictly staged and deterministic:
//!
//!   parse → validate → expand
//!
//! Semantics are handled entirely during expansion and at runtime.
//! This crate only enforces syntax and structural correctness.

mod expand;
mod parse;
mod validate;
#[expect(
    clippy::redundant_pub_crate,
    reason = "shared by sibling macro adapter modules"
)]
pub(crate) use parse::WireArgs;

use proc_macro::TokenStream;
use syn::{ItemFn, Signature, parse_macro_input};

///
/// EndpointKind
///
/// Internal classification used during expansion to determine
/// whether the generated wrapper uses `query` or `update`.
///

#[derive(Clone, Copy, Debug)]
#[expect(
    clippy::redundant_pub_crate,
    reason = "the parent proc-macro entrypoints consume this private-module type"
)]
pub(super) enum EndpointKind {
    Query,
    Update,
}

fn returns_fallible(sig: &Signature) -> bool {
    let syn::ReturnType::Type(_, ty) = &sig.output else {
        return false;
    };
    let mut ty = ty.as_ref();
    loop {
        match ty {
            syn::Type::Group(group) => ty = &group.elem,
            syn::Type::Paren(paren) => ty = &paren.elem,
            syn::Type::Path(path) => {
                return path
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "Result");
            }
            _ => return false,
        }
    }
}

///
/// expand_entry
///
/// Shared entrypoint for all canic endpoint macros.
///
/// Responsibilities:
/// - parse attribute DSL into symbols
/// - validate structural invariants
/// - expand into executable Rust code
///
/// This function deliberately performs **no semantic checks**.
///

#[expect(
    clippy::redundant_pub_crate,
    reason = "the parent proc-macro entrypoints consume this private-module function"
)]
pub(super) fn expand_entry(
    kind: EndpointKind,
    attr: TokenStream,
    item: TokenStream,
) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let sig = func.sig.clone();

    // ---------------------------------------------------------------------
    // Parse phase (syntax only)
    // ---------------------------------------------------------------------

    let parsed = match parse::parse_args(attr.into()) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    // ---------------------------------------------------------------------
    // Validate phase (structural invariants only)
    // ---------------------------------------------------------------------

    let validated = match validate::validate(kind, parsed, &sig) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    // ---------------------------------------------------------------------
    // Expansion phase (code generation)
    // ---------------------------------------------------------------------

    expand::expand(kind, validated, func).into()
}

/// Protocol-neutral entrypoint; the consumer owns bounded IO/replies.
#[expect(
    clippy::redundant_pub_crate,
    reason = "the parent proc-macro entrypoints consume this private-module function"
)]
pub(super) fn expand_raw_entry(
    kind: EndpointKind,
    attr: TokenStream,
    item: TokenStream,
) -> TokenStream {
    raw_entry(kind, attr.into(), item.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

fn raw_entry(
    kind: EndpointKind,
    attr: proc_macro2::TokenStream,
    item: proc_macro2::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    let func: ItemFn = syn::parse2(item)?;
    let sig = &func.sig;
    crate::adapter::validate_raw_signature(sig, matches!(kind, EndpointKind::Update))?;
    let mut parsed = parse::parse_args(attr)?;
    if parsed.decode.is_some() || parsed.reject_access {
        return Err(syn::Error::new_spanned(
            sig,
            "raw entrypoints own their decoder and always reject access refusal",
        ));
    }
    if parsed.payload_max_bytes.is_none() && parsed.wire.is_none() {
        return Err(syn::Error::new_spanned(
            sig,
            "raw entrypoints require payload(max_bytes = N) or a Wire contract before adapter IO",
        ));
    }
    parsed.reject_access = true;
    let mut args = validate::validate(kind, parsed, sig)?;
    if let Some(wire) = &args.wire {
        let request = &wire.request;
        args.payload_max_bytes = Some(quote::quote!((#request).bytes));
    }
    expand::expand_raw(kind, args, func)
}

#[cfg(test)]
mod raw_tests {
    use super::*;
    use quote::quote;

    #[test]
    fn raw_requires_explicit_access_bounded_io_and_a_synchronous_unit_body() {
        let attributes = quote!(public, payload(max_bytes = 4096));
        let valid = quote!(
            fn native() {}
        );
        assert!(raw_entry(EndpointKind::Query, attributes.clone(), valid.clone()).is_ok());
        for attributes in [
            quote!(),
            quote!(public),
            quote!(payload(max_bytes = 4096)),
            quote!(public, decode = LIMITS),
            quote!(
                public,
                payload(max_bytes = 4096),
                on_access_denied = "reject"
            ),
        ] {
            assert!(raw_entry(EndpointKind::Update, attributes, valid.clone()).is_err());
        }
        for invalid in [
            quote!(
                async fn native() {}
            ),
            quote!(
                unsafe fn native() {}
            ),
            quote!(
                const fn native() {}
            ),
            quote!(
                fn native(value: u64) {}
            ),
            quote!(
                fn native<T>() {}
            ),
            quote!(
                fn native() -> u64 {
                    0
                }
            ),
        ] {
            assert!(raw_entry(EndpointKind::Query, attributes.clone(), invalid).is_err());
        }
    }

    #[test]
    fn raw_exports_have_the_selected_identity_context_and_shared_guards() {
        for (kind, export, executor) in [
            (
                EndpointKind::Query,
                "canister_query native_call",
                "in_tracking_query_executor_context",
            ),
            (
                EndpointKind::Update,
                "canister_update native_call",
                "in_tracking_executor_context",
            ),
        ] {
            let expanded = raw_entry(
                kind,
                quote!(
                    requires(caller::is_controller()),
                    payload(max_bytes = 4096),
                    name = "native_call"
                ),
                quote!(
                    fn implementation() {
                        reply_native();
                    }
                ),
            )
            .unwrap();
            syn::parse2::<syn::File>(expanded.clone()).unwrap();
            let text = expanded.to_string();
            for required in [
                export,
                executor,
                "preflight_endpoint",
                "eval_application_startup",
                "eval_access_selected",
                "measure_endpoint",
                "msg_arg_data_size",
                "msg_reject",
                "reply_native",
            ] {
                assert!(text.contains(required));
            }
            // Live architectural boundary: codec-neutral exports never enter
            // Candid's service registration, decoder or reply path.
            assert!(!text.contains("candid"));
        }
    }
}

#[cfg(test)]
mod async_raw_tests {
    use super::*;
    use quote::quote;
    #[test]
    fn raw_async_updates_keep_admission_before_spawn_and_measure_the_entire_future() {
        let expanded = raw_entry(
            EndpointKind::Update,
            quote!(public, payload(max_bytes = 512)),
            quote!(
                async fn blob_update() {
                    work().await;
                }
            ),
        )
        .unwrap();
        syn::parse2::<syn::File>(expanded.clone()).unwrap();
        let text = expanded.to_string();
        assert!(
            text.find("eval_default_fleet_guard").unwrap() < text.find("futures :: spawn").unwrap()
        );
        assert!(text.contains("measure_endpoint_async"));
        assert!(!text.contains("measure_endpoint ("));
        assert!(text.contains("msg_arg_data_size"));
        assert!(!text.contains("candid"));
        assert!(
            raw_entry(
                EndpointKind::Query,
                quote!(public, payload(max_bytes = 512)),
                quote!(
                    async fn blob_query() {}
                ),
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod raw_wire_tests {
    use super::*;
    use quote::quote;

    #[test]
    fn raw_wire_declares_owner_contract_without_synthesizing_codecs() {
        for kind in [EndpointKind::Query, EndpointKind::Update] {
            let expanded = raw_entry(
                kind,
                quote!(
                    public,
                    wire(id = owner::ID, request = REQUEST, response = RESPONSE)
                ),
                quote!(
                    fn owned() {
                        owner_codec();
                    }
                ),
            )
            .unwrap();
            syn::parse2::<syn::File>(expanded.clone()).unwrap();
            let text = expanded.to_string();
            assert!(text.contains("icp:private canic:wire"));
            assert!(text.contains("owner :: ID"));
            assert!(text.contains("(REQUEST) . bytes"));
            assert!(text.contains("owner_codec"));
            assert!(!text.contains("icp:private canic:raw"));
            assert!(!text.contains("decode_request"));
            assert!(!text.contains("encode_response"));
            assert!(!text.contains("candid"));
        }
        assert!(
            raw_entry(
                EndpointKind::Update,
                quote!(
                    public,
                    payload(max_bytes = 12),
                    wire(id = owner::ID, request = REQUEST, response = RESPONSE)
                ),
                quote!(
                    fn owned() {}
                )
            )
            .is_err()
        );
    }
}
