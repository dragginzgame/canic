#![deny(unreachable_pub)]

mod adapter;
mod endpoint;
mod lifecycle;

use crate::endpoint::{EndpointKind, expand_entry};
use proc_macro::TokenStream;

/// Internal lifecycle adapter selected by Canic's public start macros.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn __canic_lifecycle(attr: TokenStream, item: TokenStream) -> TokenStream {
    lifecycle::expand(attr.into(), item.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Define a Canic query endpoint.
///
/// `decode = LIMITS` selects `canic::endpoint::ArgumentLimits`; `payload(...)`
/// selects only a byte bound. `on_access_denied = "reject"` keeps a plain reply.
///
/// See `canic::endpoint` for supported attributes.
#[proc_macro_attribute]
pub fn canic_query(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_entry(EndpointKind::Query, attr, item)
}

/// Define a Canic update endpoint.
///
/// `payload(max_bytes = N)` selects the encoded Candid argument limit and adds
/// a pre-decode guard for both ingress and inter-canister updates. Alternatively,
/// `decode = LIMITS` selects all `canic::endpoint::ArgumentLimits` dimensions.
/// `on_access_denied = "reject"` preserves plain successful Candid replies.
/// Without an explicit byte limit,
/// managed application ingress inherits the 16 KiB inspector default. The bound
/// includes encoding overhead; Candid types do not advertise it to clients.
/// An exported `name = "wire_method"` also selects the payload registration name.
///
/// See `canic::endpoint` for supported attributes.
#[proc_macro_attribute]
pub fn canic_update(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_entry(EndpointKind::Update, attr, item)
}
