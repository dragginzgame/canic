//! Application endpoint attributes, re-exported by Canic.

mod components;
#[cfg(test)]
mod tests;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;
use quote::quote;
use syn::{Expr, FnArg, ItemFn, LitStr, Path, ReturnType, parse::Parser, spanned::Spanned};

/// Name a direct shard component and record its pool requirement for fleet planning.
#[proc_macro]
pub fn shard_pool(input: TokenStream) -> TokenStream {
    components::expand("shard", input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Name a direct worker component and record its pool requirement for fleet planning.
#[proc_macro]
pub fn worker_pool(input: TokenStream) -> TokenStream {
    components::expand("worker", input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Export a synchronous query using Canic's bounded Wire contract.
#[proc_macro_attribute]
pub fn query(options: TokenStream, item: TokenStream) -> TokenStream {
    expand(Mode::Query, options.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Export a synchronous or asynchronous update using Canic's bounded Wire contract.
#[proc_macro_attribute]
pub fn update(options: TokenStream, item: TokenStream) -> TokenStream {
    expand(Mode::Update, options.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derive the passive typed caller contract from the same inline role endpoint source.
#[proc_macro_attribute]
pub fn attested_method(options: TokenStream, item: TokenStream) -> TokenStream {
    expand(Mode::AttestedMethod, options.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derive a passive Wire contract without mounting the handler or importing its guard.
#[proc_macro_attribute]
pub fn method_descriptor(options: TokenStream, item: TokenStream) -> TokenStream {
    expand(Mode::Descriptor, options.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Query,
    Update,
    Descriptor,
    AttestedMethod,
}

fn expand(mode: Mode, options: Tokens, item: Tokens) -> syn::Result<Tokens> {
    let mut contract: Option<Path> = None;
    let mut attested_method = false;
    let mut roles: Option<Expr> = None;
    let mut guard: Option<Expr> = None;
    let mut scope: Option<Expr> = None;
    let mut name: Option<LitStr> = None;
    let mut limits: Option<Expr> = None;
    let mut reply_limits: Option<Expr> = None;
    syn::meta::parser(|option| {
        if (option.path.is_ident("contract") || option.path.is_ident("attested_method"))
            && contract.is_none()
        {
            attested_method = option.path.is_ident("attested_method");
            contract = Some(option.value()?.parse()?);
        } else if option.path.is_ident("roles") && roles.is_none() {
            roles = Some(option.value()?.parse()?);
        } else if option.path.is_ident("scope") && scope.is_none() {
            scope = Some(option.value()?.parse()?);
        } else if option.path.is_ident("guard") && guard.is_none() {
            guard = Some(option.value()?.parse()?);
        } else if option.path.is_ident("name") && name.is_none() {
            name = Some(option.value()?.parse()?);
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
    let mut function: ItemFn = syn::parse2(item)?;
    let cfg = function
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"));
    let signature = &function.sig;
    let handler = &signature.ident;
    let export = name.unwrap_or_else(|| {
        LitStr::new(handler.to_string().trim_start_matches("r#"), handler.span())
    });
    let update = mode == Mode::Update;
    let kind = if update {
        quote!(update)
    } else {
        quote!(query)
    };
    let asynchronous = signature.asyncness;
    if asynchronous.is_some() && mode == Mode::Query {
        return Err(syn::Error::new(
            signature.span(),
            "async endpoints require #[canic::update]",
        ));
    }
    let wait = asynchronous.map(|_| quote!(.await));
    let mut parameters = signature.inputs.iter();
    let scoped = scope.is_some();
    if let Some(scope) = scope {
        let body = &function.block;
        function.block = if asynchronous.is_some() {
            syn::parse_quote!({
                const __CANIC_SCOPE: &str = #scope;
                let auth = ::canic::auth::session::with_authorization(__CANIC_SCOPE, |identity| identity);
                let _ = &auth;
                #body
            })
        } else {
            syn::parse_quote!({
                const __CANIC_SCOPE: &str = #scope;
                ::canic::auth::session::with_authorization(__CANIC_SCOPE, |auth| { let _ = &auth; #body })
            })
        };
    }
    let parameter = match (parameters.next(), parameters.next()) {
        (Some(FnArg::Typed(arg)), None) => {
            if scoped
                && let syn::Pat::Ident(binding) = &*arg.pat
                && binding.ident == "auth"
            {
                return Err(syn::Error::new(
                    binding.span(),
                    "auth is reserved for the admitted identity",
                ));
            }
            Some(&arg.ty)
        }
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
    if let Some(roles) = &roles
        && contract.is_some()
    {
        return Err(syn::Error::new(
            roles.span(),
            "roles cannot be combined with contract or attested_method",
        ));
    }
    let (request, reply, id, request_budget, reply_budget, check) = if let Some(contract) =
        &contract
    {
        if limits.is_some() || reply_limits.is_some() {
            return Err(syn::Error::new(
                contract.span(),
                "the contract supplies the method ID and both budgets",
            ));
        }
        let c = quote!(<#contract as ::canic::method::Contract>);
        let plain_mount = (!attested_method).then(|| {
            quote! {
                const _: () = <#contract as ::canic::method::PlainMount<_>>::CHECK;
            }
        });
        (
            quote!(#c::Input),
            quote!(#c::Output),
            quote!(#c::ID),
            quote!(#c::REQUEST),
            quote!(#c::REPLY),
            quote!(
                #plain_mount
                const _: () = assert!(::canic::method::same_name(#export, #c::NAME), "endpoint name differs from contract");
            ),
        )
    } else {
        (
            input,
            output,
            quote!(wire::method_id(#export)),
            limits.map_or_else(|| quote!(::canic::method::LIMITS), |value| quote!(#value)),
            reply_limits.map_or_else(|| quote!(__CANIC_REQUEST), |value| quote!(#value)),
            quote!(),
        )
    };
    if matches!(mode, Mode::Descriptor | Mode::AttestedMethod) {
        if let Some(contract) = contract {
            return Err(syn::Error::new(
                contract.span(),
                "use the explicit contract directly",
            ));
        }
        if mode == Mode::AttestedMethod && roles.is_none() {
            return Err(syn::Error::new(
                handler.span(),
                "attested_method requires inline roles",
            ));
        }
        if mode == Mode::Descriptor && roles.is_some() {
            return Err(syn::Error::new(
                handler.span(),
                "use attested_method for an attested endpoint",
            ));
        }
        let role_method = roles.map(|roles| {
            quote! {
                impl ::canic::topology::attestation::RoleMethod for #handler {
                    const ROLES: &'static [&'static str] = #roles;
                }
            }
        });
        let vis = &function.vis;
        let conditions: Vec<_> = cfg.collect();
        return Ok(quote! {
            #(#conditions)*
            #[allow(non_camel_case_types)]
            #[doc = concat!("Typed caller contract for `", #export, "`.")]
            #vis enum #handler {}
            #(#conditions)*
            const _: () = {
                use ::canic::method::wire;
                const __CANIC_REQUEST: wire::Limits = #request_budget;
                impl ::canic::method::Contract for #handler {
                    const NAME: &'static str = #export;
                    const ID: u64 = #id;
                    const REQUEST: wire::Limits = __CANIC_REQUEST;
                    const REPLY: wire::Limits = #reply_budget;
                    type Input = #request;
                    type Output = #reply;
                }
                #role_method
            };
        });
    }
    let roles = roles.map(|roles| quote!(#roles)).or_else(|| {
        attested_method
            .then(|| quote!(<#contract as ::canic::topology::attestation::RoleMethod>::ROLES))
    });
    let guard = guard.map(|g| {
        let check = if matches!(g, Expr::Path(_)) {
            quote!(#g())
        } else {
            quote!(#g)
        };
        quote! { let () = (#check).map_err(|error| format!("{error:?}"))?; }
    });
    // A predecessor read with just SessionView must fail to compile, even through a type alias.
    // Otherwise the old server-supplied identity would silently become client-controlled input.
    let checked_request = scoped.then(|| {
        quote! {
            trait ScopedRequestCannotBeSessionView {}
            impl ScopedRequestCannotBeSessionView for ::canic::auth::protocol::SessionView {}
            impl ScopedRequestCannotBeSessionView for #request {}
        }
    });
    let input_type = parameter.map(|_| &request);
    let admitted_type = if roles.is_some() {
        quote!((::canic::topology::attestation::Proof,#request))
    } else {
        request.clone()
    };
    let return_type = if asynchronous.is_some() {
        quote!(_)
    } else {
        reply.clone()
    };
    let unit = parameter.is_none().then(|| quote!(let () = __canic_input;));
    let admit_proof = roles.map(|roles| {
        quote! {
            let (__canic_proof,__canic_input) = __canic_input;
            const __CANIC_ROLES: &[&str] = #roles;
            ::canic::guard::admit_roles(__CANIC_ROLES, &__canic_proof)
                .unwrap_or_else(|e| ::canic::method::fail(e));
        }
    });
    let invoke = if update && asynchronous.is_none() {
        quote!(::canic::__canic_measure_update!(#export, __canic_handler(#argument)))
    } else {
        quote!(__canic_handler(#argument)#wait)
    };
    Ok(quote! {
        #[allow(clippy::needless_pass_by_value)]
        #function

        #(#cfg)*
        const _: () = {
            use ::canic::method::wire;

            let _: fn(#input_type) -> #return_type = #handler;
            #check
            #checked_request

            const __CANIC_ID: u64 = #id;
            const __CANIC_REQUEST: wire::Limits = #request_budget;
            const __CANIC_REPLY: wire::Limits = #reply_budget;
            ::canic::__canic_inspect!(wire, #export, #kind, __CANIC_ID, #admitted_type, #reply, __CANIC_REQUEST, __CANIC_REPLY);

            fn __canic_decode(bytes: &[u8]) -> Result<#admitted_type, wire::Error> {
                wire::decode(bytes, wire::Encoding::Frame { method: __CANIC_ID, direction: wire::Direction::Request }, __CANIC_REQUEST)
            }

            #[allow(clippy::needless_pass_by_value)]
            fn __canic_encode(value: #reply) -> Result<Vec<u8>, wire::Error> {
                wire::encode(&value, wire::Encoding::Frame { method: __CANIC_ID, direction: wire::Direction::Response }, __CANIC_REPLY)
            }

            {
                use ::canic::{__private::runtime as platform, method::fail as __canic_fail};
                use self::#handler as __canic_handler;

                /// Apply the declared guard before admitting the request frame.
                fn __canic_admit() -> Result<(), String> {
                    #guard
                    ::canic::method::admit_request::<#request>(__CANIC_REQUEST.bytes)
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
                #asynchronous fn __canic_invoke(__canic_input: #admitted_type) -> #reply {
                    #admit_proof
                    #unit
                    #invoke
                }
            }
        };
    })
}
