#![deny(unreachable_pub)]

mod adapter;
mod endpoint;
mod lifecycle;
mod native;

use crate::endpoint::{EndpointKind, expand_entry};
use proc_macro::TokenStream;

/// Internal entrypoint for standalone component adapters sharing Platform's executor.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn __canic_native_entry(attr: TokenStream, item: TokenStream) -> TokenStream {
    native::expand(attr.into(), item.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Internal bounded method adapter for components in a Canic-managed lifecycle.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn __canic_managed_component_entry(attr: TokenStream, item: TokenStream) -> TokenStream {
    native::expand_managed(attr.into(), item.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Internal lifecycle adapter selected by Canic's public start macros.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn __canic_lifecycle(attr: TokenStream, item: TokenStream) -> TokenStream {
    lifecycle::expand(attr.into(), item.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Define a Canic query using IC Wire request and response codecs.
///
/// Requires explicit access and `wire(id = ID, request = LIMITS, response = LIMITS)`.
/// See `canic::endpoint` for supported attributes.
#[proc_macro_attribute]
pub fn query(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_entry(EndpointKind::Query, attr, item)
}

/// Define a Canic update using IC Wire request and response codecs.
///
/// Requires explicit access and `wire(id = ID, request = LIMITS, response = LIMITS)`.
/// Reply encoding failure after mutation traps. See `canic::endpoint`.
#[proc_macro_attribute]
pub fn update(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_entry(EndpointKind::Update, attr, item)
}

/// Adapt a required external Candid query protocol with Canic guards.
///
/// Use `decode = LIMITS` for bounded Candid decoding. Owned RPC uses `query`.
#[proc_macro_attribute]
pub fn candid_query(attr: TokenStream, item: TokenStream) -> TokenStream {
    endpoint::expand_candid_entry(EndpointKind::Query, attr, item)
}

/// Adapt a required external Candid update protocol with Canic guards.
///
/// Use `decode = LIMITS` for bounded Candid decoding. Owned RPC uses `update`.
#[proc_macro_attribute]
pub fn candid_update(attr: TokenStream, item: TokenStream) -> TokenStream {
    endpoint::expand_candid_entry(EndpointKind::Update, attr, item)
}

/// Export a synchronous raw query with Canic startup, access and dispatch guards.
///
/// Requires explicit access and `payload(max_bytes = N)` or `wire(...)`; the fn() body owns
/// bounded decoding and its reply. No Candid declaration or encoding is emitted.
#[proc_macro_attribute]
pub fn raw_query(attr: TokenStream, item: TokenStream) -> TokenStream {
    endpoint::expand_raw_entry(EndpointKind::Query, attr, item)
}

/// Export a raw update with Canic startup, access and dispatch guards.
///
/// Requires explicit access and `payload(max_bytes = N)` or `wire(...)`; the fn() body owns
/// bounded decoding and its reply. Reply failure after mutation must trap.
/// An async unit handler uses the shared executor and async measurement owner;
/// admission completes before it is spawned. Effects across awaits retain IC semantics.
#[proc_macro_attribute]
pub fn raw_update(attr: TokenStream, item: TokenStream) -> TokenStream {
    endpoint::expand_raw_entry(EndpointKind::Update, attr, item)
}
