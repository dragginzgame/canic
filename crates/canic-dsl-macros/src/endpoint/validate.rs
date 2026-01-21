use crate::endpoint::parse::{AccessExprAst, ParsedArgs};
use proc_macro2::TokenStream as TokenStream2;
use syn::Signature;

///
/// ValidatedArgs
///
/// Arguments validated for macro expansion.
///
/// This phase enforces only *structural* invariants:
/// - async requirements
/// - fallible return requirements
///
/// It does NOT interpret symbols semantically.
///
pub struct ValidatedArgs {
    pub forwarded: Vec<TokenStream2>,
    pub requires: Vec<AccessExprAst>,
    pub internal: bool,
}

pub fn validate(
    parsed: ParsedArgs,
    sig: &Signature,
    asyncness: bool,
) -> syn::Result<ValidatedArgs> {
    if parsed.requires_async && !asyncness {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "this endpoint requires `async fn` due to access predicates",
        ));
    }

    if parsed.requires_fallible && !returns_fallible(sig) {
        return Err(syn::Error::new_spanned(
            &sig.output,
            "this endpoint must return `Result<_, E>` where `E: From<canic::Error>`",
        ));
    }

    Ok(ValidatedArgs {
        forwarded: parsed.forwarded,
        requires: parsed.requires,
        internal: parsed.internal,
    })
}

fn returns_fallible(sig: &Signature) -> bool {
    let syn::ReturnType::Type(_, ty) = &sig.output else {
        return false;
    };

    let syn::Type::Path(ty) = &**ty else {
        return false;
    };

    ty.path
        .segments
        .last()
        .is_some_and(|seg| seg.ident == "Result")
}
