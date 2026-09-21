#![deny(unreachable_pub)]

mod endpoint;

use crate::endpoint::{EndpointKind, expand_entry};
use proc_macro::TokenStream;

/// Define a Canic query endpoint.
///
/// See `canic::endpoint` for supported attributes.
#[proc_macro_attribute]
pub fn canic_query(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_entry(EndpointKind::Query, attr, item)
}

/// Define a Canic update endpoint.
///
/// `payload(max_bytes = N)` selects the encoded Candid argument limit and adds
/// a pre-decode guard for both ingress and inter-canister updates. Without it,
/// managed application ingress inherits the 16 KiB inspector default. The bound
/// includes encoding overhead; Candid types do not advertise it to clients.
/// An exported `name = "wire_method"` also selects the payload registration name.
///
/// See `canic::endpoint` for supported attributes.
#[proc_macro_attribute]
pub fn canic_update(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_entry(EndpointKind::Update, attr, item)
}
