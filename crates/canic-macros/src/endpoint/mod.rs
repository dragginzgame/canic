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
    let syn::Type::Path(ty) = &**ty else {
        return false;
    };

    ty.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "Result")
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

/// Protocol-neutral synchronous entrypoint; the consumer owns bounded IO/replies.
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
    if sig.asyncness.is_some()
        || sig.constness.is_some()
        || sig.abi.is_some()
        || sig.variadic.is_some()
        || sig.generics.where_clause.is_some()
        || !matches!(sig.safety, syn::Safety::Default)
        || !sig.inputs.is_empty()
        || !sig.generics.params.is_empty()
        || !matches!(sig.output, syn::ReturnType::Default)
    {
        return Err(syn::Error::new_spanned(
            sig,
            "raw entrypoints require a synchronous safe fn() with an implicit unit result",
        ));
    }
    let mut parsed = parse::parse_args(attr)?;
    if parsed.decode.is_some() || parsed.reject_access {
        return Err(syn::Error::new_spanned(
            sig,
            "raw entrypoints own their decoder and always reject access refusal",
        ));
    }
    if parsed.payload_max_bytes.is_none() {
        return Err(syn::Error::new_spanned(
            sig,
            "raw entrypoints require payload(max_bytes = N) before adapter IO",
        ));
    }
    parsed.reject_access = true;
    let args = validate::validate(kind, parsed, sig)?;
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
                "in_query_executor_context",
            ),
            (
                EndpointKind::Update,
                "canister_update native_call",
                "in_executor_context",
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
