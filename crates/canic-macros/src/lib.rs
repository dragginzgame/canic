//! Application endpoint attributes, re-exported by Canic.

#[cfg(test)]
mod tests;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;
use quote::quote;
use syn::{Expr, FnArg, ItemFn, LitStr, Path, ReturnType, parse::Parser, spanned::Spanned};

/// Export a synchronous query using Canic's bounded Wire contract.
#[proc_macro_attribute]
pub fn query(options: TokenStream, item: TokenStream) -> TokenStream {
    expand(false, options.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Export a synchronous or asynchronous update using Canic's bounded Wire contract.
#[proc_macro_attribute]
pub fn update(options: TokenStream, item: TokenStream) -> TokenStream {
    expand(true, options.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(update: bool, options: Tokens, item: Tokens) -> syn::Result<Tokens> {
    let mut contract: Option<Path> = None;
    let mut guard: Option<Path> = None;
    let mut scope: Option<LitStr> = None;
    let mut name: Option<LitStr> = None;
    let mut method: Option<Expr> = None;
    let mut limits: Option<Expr> = None;
    let mut reply_limits: Option<Expr> = None;
    syn::meta::parser(|option| {
        if option.path.is_ident("contract") && contract.is_none() {
            contract = Some(option.value()?.parse()?);
        } else if option.path.is_ident("scope") && scope.is_none() {
            scope = Some(option.value()?.parse()?);
        } else if option.path.is_ident("guard") && guard.is_none() {
            guard = Some(option.value()?.parse()?);
        } else if option.path.is_ident("name") && name.is_none() {
            name = Some(option.value()?.parse()?);
        } else if option.path.is_ident("method") && method.is_none() {
            method = Some(option.value()?.parse()?);
        } else if option.path.is_ident("limits") && limits.is_none() {
            limits = Some(option.value()?.parse()?);
        } else if option.path.is_ident("reply_limits") && reply_limits.is_none() {
            reply_limits = Some(option.value()?.parse()?);
        } else {
            return Err(option.error("unknown or repeated endpoint option"));
        }
        Ok(())
    })
    .parse2(options)?;
    let function: ItemFn = syn::parse2(item)?;
    let cfg = function
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"));
    let signature = &function.sig;
    let handler = &signature.ident;
    let export = name.unwrap_or_else(|| {
        LitStr::new(handler.to_string().trim_start_matches("r#"), handler.span())
    });
    let kind = if update {
        quote!(update)
    } else {
        quote!(query)
    };
    let asynchronous = signature.asyncness;
    if asynchronous.is_some() && !update {
        return Err(syn::Error::new(
            signature.span(),
            "async endpoints require #[canic::update]",
        ));
    }
    let wait = asynchronous.map(|_| quote!(.await));
    let mut parameters = signature.inputs.iter();
    if scope.is_some() && !matches!(parameters.next(), Some(FnArg::Typed(_))) {
        return Err(syn::Error::new(
            signature.span(),
            "a scoped endpoint takes the verified session first",
        ));
    }
    let session = scope.map(|scope| {
        quote!(
            ::canic::auth::session::authorize(#scope).unwrap_or_else(|error|
                ::canic::auth::endpoint::api::trap(&format!("user authorization: {error:?}"))),
        )
    });
    let parameter = match (parameters.next(), parameters.next()) {
        (Some(FnArg::Typed(arg)), None) => Some(&arg.ty),
        (None, None) => None,
        _ => {
            return Err(syn::Error::new(
                signature.span(),
                "expected at most one Wire request parameter; group request fields in a struct",
            ));
        }
    };
    let input = parameter.map_or_else(|| quote!(()), |ty| quote!(#ty));
    let output = match &signature.output {
        ReturnType::Default => quote!(()),
        ReturnType::Type(_, ty) => quote!(#ty),
    };
    let argument = parameter.map(|_| quote!(__canic_input));
    let (request, reply, id, request_budget, reply_budget, check) = if let Some(contract) = contract
    {
        if method.is_some() || limits.is_some() || reply_limits.is_some() {
            return Err(syn::Error::new(
                contract.span(),
                "the contract supplies the method ID and both budgets",
            ));
        }
        let c = quote!(<#contract as ::canic::method::Contract>);
        (
            quote!(#c::Input),
            quote!(#c::Output),
            quote!(#c::ID),
            quote!(#c::REQUEST),
            quote!(#c::REPLY),
            quote!(
                const _: () = assert!(::canic::method::same_name(#export, #c::NAME), "endpoint name differs from contract");
            ),
        )
    } else {
        (
            input,
            output,
            method.map_or_else(|| quote!(wire::method_id(#export)), |id| quote!(#id)),
            limits.map_or_else(|| quote!(::canic::method::LIMITS), |value| quote!(#value)),
            reply_limits.map_or_else(|| quote!(__CANIC_REQUEST), |value| quote!(#value)),
            quote!(),
        )
    };
    let guard = guard.map(|g| quote!(#g().map_err(|error| format!("{error:?}"))?;));
    let unit = parameter.is_none().then(|| quote!(let () = __canic_input;));
    Ok(quote! {
        #[allow(clippy::needless_pass_by_value)]
        #function
        #(#cfg)*
        const _: () = {
            use ::canic::method::{runtime as platform, wire, fail as __canic_fail};
            use self::#handler as __canic_handler;
            #check
            const __CANIC_ID: u64 = #id;
            const __CANIC_REQUEST: wire::Limits = #request_budget;
            const __CANIC_REPLY: wire::Limits = #reply_budget;

            /// Apply the declared guard before admitting the request frame.
            fn __canic_admit() -> Result<(), String> {
                #guard
                ::canic::method::admit_request::<#request>(__CANIC_REQUEST.bytes)
            }

            fn __canic_decode(bytes: &[u8]) -> Result<#request, wire::Error> {
                wire::decode(bytes, wire::Encoding::Frame { method: __CANIC_ID, direction: wire::Direction::Request }, __CANIC_REQUEST)
            }

            #[allow(clippy::needless_pass_by_value)]
            fn __canic_encode(value: #reply) -> Result<Vec<u8>, wire::Error> {
                wire::encode(&value, wire::Encoding::Frame { method: __CANIC_ID, direction: wire::Direction::Response }, __CANIC_REPLY)
            }

            #[platform::#kind(
                runtime = platform,
                guard = __canic_admit,
                decode = __canic_decode,
                max_bytes = 4294967295,
                on_error = __canic_fail,
                name = #export,
                encode = __canic_encode,
            )]
            #asynchronous fn __canic_invoke(__canic_input: #request) -> #reply {
                #unit
                __canic_handler(#session #argument)#wait
            }
        };
    })
}
