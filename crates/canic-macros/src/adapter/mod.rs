//! Raw IC entrypoints shared by bounded endpoint and lifecycle adapters.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Signature, Type};

///
/// EntryKind
///
/// The macro adapter's IC export kind determines whether a reply is legal.
///

#[expect(
    clippy::redundant_pub_crate,
    reason = "shared by sibling macro expansion modules"
)]
pub(crate) enum EntryKind {
    Query,
    CompositeQuery,
    Update,
    Init,
    PostUpgrade,
}

impl EntryKind {
    const fn export(&self) -> &'static str {
        match self {
            Self::Query => "canister_query",
            Self::CompositeQuery => "canister_composite_query",
            Self::Update => "canister_update",
            Self::Init => "canister_init",
            Self::PostUpgrade => "canister_post_upgrade",
        }
    }

    pub(crate) fn candid_attribute(&self, method: &syn::LitStr) -> TokenStream {
        match self {
            Self::Query => quote!(#[::candid::candid_method(query, rename = #method)]),
            Self::CompositeQuery => {
                quote!(#[::candid::candid_method(composite_query, rename = #method)])
            }
            Self::Update => quote!(#[::candid::candid_method(update, rename = #method)]),
            Self::Init => quote!(#[::candid::candid_method(init)]),
            Self::PostUpgrade => quote!(),
        }
    }
}

///
/// RawAdapter
///
/// The original signature owns Candid; the wrapper may separately return an access refusal.
/// Shared by endpoint and lifecycle expansion.
///

#[expect(
    clippy::redundant_pub_crate,
    reason = "shared by sibling macro expansion modules"
)]
pub(crate) struct RawAdapter<'a> {
    pub signature: &'a Signature,
    pub kind: EntryKind,
    pub method: String,
    pub decode: Option<&'a TokenStream>,
    pub max_bytes: Option<&'a TokenStream>,
    pub reject_access: bool,
    pub wrapper_async: bool,
}

impl RawAdapter<'_> {
    pub(crate) fn expand(&self) -> syn::Result<TokenStream> {
        let name = &self.signature.ident;
        let adapter_name = format_ident!("__canic_raw_{}", name);
        let lifecycle = matches!(self.kind, EntryKind::Init | EntryKind::PostUpgrade);
        let mut names = Vec::new();
        let mut types = Vec::new();
        for input in &self.signature.inputs {
            let syn::FnArg::Typed(input) = input else {
                return Err(syn::Error::new_spanned(
                    input,
                    "`self` is unsupported on Canic entrypoints",
                ));
            };
            let syn::Pat::Ident(ident) = &*input.pat else {
                return Err(syn::Error::new_spanned(
                    &input.pat,
                    "destructuring parameters not supported",
                ));
            };
            names.push(ident.ident.clone());
            types.push(input.ty.as_ref());
        }
        let decode = self.decode(&names, &types);
        let invoke = if self.wrapper_async {
            quote!(#name(#(#names),*).await)
        } else {
            quote!(#name(#(#names),*))
        };
        let invoke = if self.reject_access {
            quote! {
                let __canic_result = match #invoke {
                    Ok(value) => value,
                    Err(error) => {
                        ::canic::__internal::cdk::api::msg_reject(error.to_string());
                        return;
                    }
                };
            }
        } else {
            quote!(let __canic_result = #invoke;)
        };
        let reply = if lifecycle { quote!() } else { self.reply() };
        let execute = quote! { #decode #invoke #reply };
        let execute = if self.wrapper_async {
            quote!(::canic::__internal::cdk::futures::spawn(async { #execute });)
        } else {
            execute
        };
        Ok(entrypoint(
            &self.kind,
            &self.method,
            &adapter_name,
            quote!(::canic::__internal::cdk),
            execute,
        ))
    }

    fn decode(&self, names: &[syn::Ident], types: &[&Type]) -> TokenStream {
        if let Some(limits) = self.decode {
            return quote! {
                const __CANIC_ARGUMENT_LIMITS: ::canic::endpoint::ArgumentLimits = #limits;
                let (#(#names,)*): (#(#types,)*) = __CANIC_ARGUMENT_LIMITS.read()
                    .unwrap_or_else(|error| ::canic::__internal::cdk::trap(error.to_string()));
            };
        }
        let size_guard = self.max_bytes.map(|limit| {
            quote! {
                const __CANIC_MAX_BYTES: usize = #limit;
                if __canic_payload_len > __CANIC_MAX_BYTES {
                    ::canic::__internal::cdk::trap(format!(
                        "argument envelope is {__canic_payload_len} bytes; maximum is {}",
                        __CANIC_MAX_BYTES,
                    ));
                }
            }
        });
        let size_guard = quote! {
            let __canic_payload_len = ::canic::__internal::cdk::raw::msg_arg_data_size();
            #size_guard
        };
        // Byte-only declarations retain their existing zero-argument behavior.
        // Selecting a decoder above always validates the complete envelope.
        if names.is_empty() {
            return size_guard;
        }
        quote! {
            #size_guard
            let mut __canic_arg_bytes = vec![0_u8; __canic_payload_len];
            ::canic::__internal::cdk::raw::msg_arg_data_copy(&mut __canic_arg_bytes, 0);
            let mut __canic_decoder_config = ::candid::DecoderConfig::new();
            __canic_decoder_config.set_skipping_quota(10_000);
            let (#(#names,)*): (#(#types,)*) = ::candid::utils::decode_args_with_config(
                &__canic_arg_bytes, &__canic_decoder_config,
            ).unwrap_or_else(|error| ::canic::__internal::cdk::trap(error.to_string()));
        }
    }

    fn reply(&self) -> TokenStream {
        let encode = match &self.signature.output {
            syn::ReturnType::Type(_, ty) if matches!(&**ty, Type::Tuple(tuple) if tuple.elems.len() > 1) =>
            {
                quote!(::candid::utils::encode_args(__canic_result))
            }
            _ => quote!(::candid::utils::encode_one(__canic_result)),
        };
        quote! {
            let __canic_reply = #encode.unwrap_or_else(|error| {
                ::canic::__internal::cdk::trap(format!("failed to encode response: {error}"))
            });
            ::canic::__internal::cdk::api::msg_reply(__canic_reply);
        }
    }
}

/// Emit only the IC export and executor context, independent of payload protocol.
#[expect(
    clippy::redundant_pub_crate,
    reason = "shared by sibling endpoint expansion modules"
)]
pub(crate) fn entrypoint(
    kind: &EntryKind,
    method: &str,
    name: &syn::Ident,
    sdk: TokenStream,
    execute: TokenStream,
) -> TokenStream {
    let export = if matches!(kind, EntryKind::Init | EntryKind::PostUpgrade) {
        kind.export().to_string()
    } else {
        format!("{} {}", kind.export(), method)
    };
    let host_export = export.replace(' ', ".").replace(['-', '<', '>'], "_");
    let executor = if matches!(kind, EntryKind::Query | EntryKind::CompositeQuery) {
        format_ident!("in_query_executor_context")
    } else {
        format_ident!("in_executor_context")
    };
    quote! {
        #[cfg_attr(target_family = "wasm", unsafe(export_name = #export))]
        #[cfg_attr(not(target_family = "wasm"), unsafe(export_name = #host_export))]
        fn #name() {
            #sdk::futures::internals::#executor(|| { #execute });
        }
    }
}

/// Bind one protocol-neutral method declaration to the shared export owner.
#[expect(
    clippy::redundant_pub_crate,
    reason = "shared by native and framework adapters"
)]
pub(crate) fn raw_declaration(
    kind: &EntryKind,
    method: &str,
    signature: &Signature,
) -> syn::Result<TokenStream> {
    let mode = match kind {
        EntryKind::Query => 1,
        EntryKind::Update => 2,
        EntryKind::CompositeQuery => 3,
        EntryKind::Init | EntryKind::PostUpgrade => return Ok(TokenStream::new()),
    };
    let length = u16::try_from(method.len())
        .map_err(|_| syn::Error::new_spanned(signature, "raw endpoint name is too long"))?;
    if method.is_empty() || method.chars().any(char::is_control) {
        return Err(syn::Error::new_spanned(
            signature,
            "raw endpoint name must be nonempty without controls",
        ));
    }
    let mut declaration = vec![mode];
    declaration.extend_from_slice(&length.to_le_bytes());
    declaration.extend_from_slice(method.as_bytes());
    let declaration_length = declaration.len();
    Ok(quote! {
        #[used]
        #[unsafe(link_section = "canic:raw")]
        static DECLARATION: [u8; #declaration_length] = [#(#declaration),*];
    })
}

/// Validate the synchronous zero-argument body owned by a raw adapter.
#[expect(
    clippy::redundant_pub_crate,
    reason = "shared by native and framework adapters"
)]
pub(crate) fn validate_raw_signature(sig: &Signature) -> syn::Result<()> {
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
    Ok(())
}
