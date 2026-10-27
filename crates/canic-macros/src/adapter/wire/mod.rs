//! Native Wire IO inside the shared guarded endpoint adapter.

use crate::endpoint::WireArgs;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

#[expect(
    clippy::redundant_pub_crate,
    reason = "shared by the sibling endpoint expander while unreachable_pub forbids public visibility"
)]
pub(crate) fn declaration(
    wire: &WireArgs,
    kind: &super::EntryKind,
    method: &str,
    signature: &syn::Signature,
) -> syn::Result<TokenStream> {
    if method.is_empty() || method.len() > u16::MAX as usize || method.chars().any(char::is_control)
    {
        return Err(syn::Error::new_spanned(
            signature,
            "invalid Wire method name",
        ));
    }
    let kind = match kind {
        super::EntryKind::Query => quote!(Query),
        super::EntryKind::CompositeQuery => quote!(CompositeQuery),
        super::EntryKind::Update => quote!(Update),
        _ => {
            return Err(syn::Error::new_spanned(
                signature,
                "Wire method requires a callable export",
            ));
        }
    };
    let id = &wire.id;
    let length = method.len();
    let limits = |expression: &TokenStream| {
        quote! {
            ::canic::__internal::contracts::protocol::wire_declaration::MethodLimits {
                bytes: (#expression).bytes as u64,
                allocation: (#expression).allocation as u64,
                nodes: (#expression).nodes as u64,
                depth: (#expression).depth as u64,
            }
        }
    };
    let request = limits(&wire.request);
    let response = limits(&wire.response);
    Ok(quote! {
        #[used]
        #[unsafe(link_section = "icp:private canic:wire")]
        static DECLARATION: [u8;
            ::canic::__internal::contracts::protocol::wire_declaration::HEADER_BYTES + #length
        ] = ::canic::__internal::contracts::protocol::wire_declaration::CompiledMethod {
            name: #method,
            kind: ::canic::__internal::contracts::protocol::wire_declaration::MethodKind::#kind,
            id: #id,
            request: #request,
            response: #response,
        }.encode();
    })
}

pub(super) fn decode(wire: &WireArgs, names: &[syn::Ident], types: &[&Type]) -> TokenStream {
    let id = &wire.id;
    let limits = &wire.request;
    let binding = if let (Some(name), Some(ty)) = (names.first(), types.first()) {
        quote!(let #name: #ty)
    } else {
        quote!(let (): ())
    };
    quote! {
        const __CANIC_WIRE_REQUEST: ::canic::__internal::ic_wire::Limits = #limits;
        let __canic_size = ::canic::__internal::cdk::raw::msg_arg_data_size();
        if __canic_size > __CANIC_WIRE_REQUEST.bytes {
            ::canic::__internal::core::control_plane_support::ops::ic::IcOps::trap("Wire request exceeds byte allowance");
        }
        let mut __canic_bytes = Vec::new();
        __canic_bytes.try_reserve_exact(__canic_size)
            .unwrap_or_else(|_| ::canic::__internal::core::control_plane_support::ops::ic::IcOps::trap("Wire request allocation failed"));
        __canic_bytes.resize(__canic_size, 0);
        ::canic::__internal::cdk::raw::msg_arg_data_copy(&mut __canic_bytes, 0);
        #binding = ::canic::__internal::ic_wire::native::decode_request(
            #id, &__canic_bytes, __CANIC_WIRE_REQUEST,
        ).unwrap_or_else(|error| ::canic::__internal::core::control_plane_support::ops::ic::IcOps::trap(error.to_string()));
    }
}

pub(super) fn reply(wire: &WireArgs) -> TokenStream {
    let id = &wire.id;
    let limits = &wire.response;
    quote! {
        const __CANIC_WIRE_RESPONSE: ::canic::__internal::ic_wire::Limits = #limits;
        let __canic_reply = ::canic::__internal::ic_wire::native::encode_response(
            #id, &__canic_result, __CANIC_WIRE_RESPONSE,
        ).unwrap_or_else(|error| ::canic::__internal::core::control_plane_support::ops::ic::IcOps::trap(error.to_string()));
        ::canic::__internal::cdk::api::msg_reply(__canic_reply);
    }
}
