//! Canic proc macros.
//!
//! Thin, opinionated wrappers around IC CDK endpoint attributes
//! (`#[query]`, `#[update]`), routed through `canic::cdk::*`.
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
use syn::{ItemFn, parse_macro_input};

///
/// EndpointKind
///
/// Internal classification used during expansion to determine
/// whether the generated wrapper uses `query` or `update`.
///

#[derive(Clone, Copy, Debug)]
pub enum EndpointKind {
    Query,
    Update,
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

pub fn expand_entry(kind: EndpointKind, attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let sig = func.sig.clone();
    let is_async = sig.asyncness.is_some();

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

    let validated = match validate::validate(parsed, &sig, is_async) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    // ---------------------------------------------------------------------
    // Expansion phase (code generation)
    // ---------------------------------------------------------------------

    expand::expand(kind, validated, func).into()
}
