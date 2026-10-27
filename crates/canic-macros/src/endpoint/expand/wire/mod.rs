//! Register the actual typed Wire declaration for native artifact generation.

use crate::endpoint::{EndpointKind, parse::QueryMode, validate::ValidatedArgs};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, ReturnType, Signature};

pub(super) fn registration(
    kind: EndpointKind,
    args: &ValidatedArgs,
    signature: &Signature,
) -> TokenStream {
    let Some(wire) = &args.wire else {
        return quote!();
    };
    let method = super::exported_method(args, &signature.ident);
    let kind = match (kind, args.query_mode) {
        (EndpointKind::Query, QueryMode::Composite) => quote!(CompositeQuery),
        (EndpointKind::Query, _) => quote!(Query),
        (EndpointKind::Update, _) => quote!(Update),
    };
    let request = if let Some(FnArg::Typed(input)) = signature.inputs.first() {
        let ty = &input.ty;
        quote!(#ty)
    } else {
        quote!(())
    };
    let response = match &signature.output {
        ReturnType::Default => quote!(()),
        ReturnType::Type(_, ty) => quote!(#ty),
    };
    let id = &wire.id;
    let request_limits = &wire.request;
    let response_limits = &wire.response;
    quote! {
        ::canic::__canic_register_wire_schema!(
            #method, #kind, #id, #request, #response, #request_limits, #response_limits
        );
    }
}
