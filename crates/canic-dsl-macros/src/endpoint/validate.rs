use crate::endpoint::parse::{AuthSymbol, EnvSymbol, GuardSymbol, ParsedArgs, RuleSymbol};
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
    pub guard: Vec<GuardSymbol>,
    pub auth: Vec<AuthSymbol>,
    pub env: Vec<EnvSymbol>,
    pub rules: Vec<RuleSymbol>,
}

pub fn validate(
    parsed: ParsedArgs,
    sig: &Signature,
    asyncness: bool,
) -> syn::Result<ValidatedArgs> {
    let has_guard = !parsed.guard.is_empty();
    let has_auth = !parsed.auth.is_empty();
    let has_env = !parsed.env.is_empty();
    let has_rules = !parsed.rules.is_empty();

    // Any access DSL beyond pure forwarding requires async
    let requires_async = has_auth || has_env || has_rules;

    // Any access DSL at all requires Result<_, E>
    let requires_fallible = has_guard || has_auth || has_env || has_rules;

    if requires_async && !asyncness {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "this endpoint requires `async fn` due to access rules",
        ));
    }

    if requires_fallible && !returns_fallible(sig) {
        return Err(syn::Error::new_spanned(
            &sig.output,
            "this endpoint must return `Result<_, E>` where `E: From<canic::Error>`",
        ));
    }

    Ok(ValidatedArgs {
        forwarded: parsed.forwarded,
        guard: parsed.guard,
        auth: parsed.auth,
        env: parsed.env,
        rules: parsed.rules,
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
