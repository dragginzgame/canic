mod endpoint;

use crate::endpoint::{EndpointKind, expand_entry};
use canic_dsl as _;
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
/// See `canic::endpoint` for supported attributes.
#[proc_macro_attribute]
pub fn canic_update(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_entry(EndpointKind::Update, attr, item)
}
