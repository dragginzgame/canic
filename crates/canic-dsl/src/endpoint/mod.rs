//! Canic proc macros.
//!
//! Thin, opinionated wrappers around IC CDK endpoint attributes
//! (`#[query]`, `#[update]`), routed through `canic::cdk::*`.
//!
//! Pipeline enforced by generated wrappers:
//!   guard → auth → env → rule → dispatch

mod expand;
mod parse;
mod validate;

use proc_macro::TokenStream;
use syn::{ItemFn, parse_macro_input};

///
/// EndpointKind
/// Internal endpoint classification used by macro expansion.
///

#[derive(Clone, Copy)]
pub enum EndpointKind {
    Query,
    Update,
}

/// Internal dispatcher for canic endpoint macros.
///
/// This performs parse → validate → expand and returns a compiled TokenStream.
pub fn expand_entry(kind: EndpointKind, attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let sig = func.sig.clone();
    let asyncness = sig.asyncness.is_some();

    let parsed = match parse::parse_args(attr.into()) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    let validated = match validate::validate(parsed, &sig, asyncness) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    expand::expand(kind, validated, func).into()
}
