use crate::endpoint::parse::{AuthSpec, ParsedArgs};
use proc_macro2::TokenStream as TokenStream2;
use syn::Expr;

///
/// ValidatedArgs
///
/// Arguments validated for macro expansion.
///
/// These invariants are enforced:
/// - fallible return when required
/// - async where required
/// - no conflicting guard/auth usage
///

pub struct ValidatedArgs {
    pub forwarded: Vec<TokenStream2>,
    pub app_guard: bool,
    pub auth: Option<AuthSpec>,
    pub env: Vec<Expr>,
    pub rules: Vec<Expr>,
}

pub fn validate(
    parsed: ParsedArgs,
    sig: &syn::Signature,
    asyncness: bool,
) -> syn::Result<ValidatedArgs> {
    if parsed.app_guard && parsed.user_guard {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "`app` cannot be combined with `guard = ...`",
        ));
    }

    if parsed.auth.is_some() && parsed.user_guard {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "authorization cannot be combined with `guard = ...`",
        ));
    }

    if parsed.auth.is_some() {
        if !asyncness {
            return Err(syn::Error::new_spanned(
                &sig.ident,
                "authorization requires `async fn`",
            ));
        }

        if !returns_fallible(sig) {
            return Err(syn::Error::new_spanned(
                &sig.output,
                "authorized endpoints must return `Result<_, E>` where `E: From<canic::Error>`",
            ));
        }
    }

    if parsed.app_guard && !returns_fallible(sig) {
        return Err(syn::Error::new_spanned(
            &sig.output,
            "`app` guard requires `Result<_, E>` where `E: From<canic::Error>`",
        ));
    }

    if !parsed.rules.is_empty() && !returns_fallible(sig) {
        return Err(syn::Error::new_spanned(
            &sig.output,
            "`rule(...)` requires `Result<_, E>` where `E: From<canic::Error>`",
        ));
    }

    if !parsed.env.is_empty() && !returns_fallible(sig) {
        return Err(syn::Error::new_spanned(
            &sig.output,
            "`env(...)` requires `Result<_, E>` where `E: From<canic::Error>`",
        ));
    }

    if !parsed.rules.is_empty() && !asyncness {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "`rule(...)` requires `async fn`",
        ));
    }

    if !parsed.env.is_empty() && !asyncness {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "`env(...)` requires `async fn`",
        ));
    }

    Ok(ValidatedArgs {
        forwarded: parsed.forwarded,
        app_guard: parsed.app_guard,
        auth: parsed.auth,
        rules: parsed.rules,
        env: parsed.env,
    })
}

fn returns_fallible(sig: &syn::Signature) -> bool {
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
