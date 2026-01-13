use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Expr, Ident, Meta, Path, Token, parse::Parser, punctuated::Punctuated};

//
// ============================================================================
// parse — attribute grammar only
// ============================================================================
//

///
/// AuthSpec
///

#[derive(Clone, Debug)]
pub enum AuthSpec {
    Any(Vec<AuthSymbol>),
    All(Vec<AuthSymbol>),
}

#[derive(Clone, Debug)]
pub enum GuardSymbol {
    AppIsLive,
}

#[derive(Clone, Debug)]
pub enum AuthSymbol {
    CallerIsController,
    CallerIsParent,
    CallerIsChild,
    CallerIsRoot,
    CallerIsSameCanister,
    CallerIsRegisteredToSubnet,
    CallerIsWhitelisted,
}

#[derive(Clone, Debug)]
pub enum EnvSymbol {
    SelfIsPrimeSubnet,
    SelfIsPrimeRoot,
}

///
/// ParsedArgs
///

#[derive(Debug)]
pub struct ParsedArgs {
    pub forwarded: Vec<TokenStream2>,
    pub app_guard: bool,
    pub user_guard: bool,
    pub auth: Option<AuthSpec>,
    pub env: Vec<EnvSymbol>,
    pub rules: Vec<Expr>,
}

pub fn parse_args(attr: TokenStream2) -> syn::Result<ParsedArgs> {
    let Ok(metas) = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(attr.clone()) else {
        // If the attr doesn't parse as Meta list, fall back to forwarding raw tokens to the CDK.
        // This preserves compatibility with CDK syntax we don't model.
        if attr.is_empty() {
            return Ok(empty());
        }

        return Ok(ParsedArgs {
            forwarded: vec![attr],
            ..empty()
        });
    };

    let mut forwarded = Vec::new();
    let mut app_guard = false;
    let mut user_guard = false;
    let mut auth = None::<AuthSpec>;
    let mut env = Vec::<EnvSymbol>::new();
    let mut rules = Vec::<Expr>::new();

    for meta in metas {
        match meta {
            // guard(...)
            //
            // Canic-specific guard stage. Top-level `app` is no longer accepted.
            Meta::List(list) if list.path.is_ident("guard") => {
                let inner = Punctuated::<Meta, Token![,]>::parse_terminated
                    .parse2(list.tokens.clone())?
                    .into_iter()
                    .collect::<Vec<_>>();

                if inner.is_empty() {
                    return Err(syn::Error::new_spanned(
                        list,
                        "`guard(...)` expects at least one symbol (e.g., `guard(app_is_live)`)",
                    ));
                }

                for item in inner {
                    match item {
                        Meta::Path(p) => match parse_guard_symbol(p)? {
                            GuardSymbol::AppIsLive => app_guard = true,
                        },
                        other => {
                            return Err(syn::Error::new_spanned(
                                other,
                                "only `guard(app_is_live)` is supported",
                            ));
                        }
                    }
                }
            }

            // auth_any(...)
            Meta::List(list) if list.path.is_ident("auth_any") => {
                if auth.is_some() {
                    return Err(conflicting_auth(&list));
                }
                let rules = parse_auth_symbols(&list)?;
                auth = Some(AuthSpec::Any(rules));
            }

            // auth_all(...)
            Meta::List(list) if list.path.is_ident("auth_all") => {
                if auth.is_some() {
                    return Err(conflicting_auth(&list));
                }
                let rules = parse_auth_symbols(&list)?;
                auth = Some(AuthSpec::All(rules));
            }

            // rule(...)
            //
            // Parse as Expr so you can do rule(local_only()), rule(max_rounds(rounds, 10_000)), etc.
            Meta::List(list) if list.path.is_ident("rule") => {
                let parsed = Punctuated::<Expr, Token![,]>::parse_terminated
                    .parse2(list.tokens.clone())?
                    .into_iter()
                    .collect::<Vec<_>>();

                if parsed.is_empty() {
                    return Err(syn::Error::new_spanned(
                        list,
                        "`rule(...)` expects at least one rule expression",
                    ));
                }

                rules.extend(parsed);
            }

            // env(...)
            //
            // Parse as DSL symbols like env(self_is_prime_subnet).
            Meta::List(list) if list.path.is_ident("env") => {
                let parsed = Punctuated::<Path, Token![,]>::parse_terminated
                    .parse2(list.tokens.clone())?
                    .into_iter()
                    .map(parse_env_symbol)
                    .collect::<syn::Result<Vec<_>>>()?;

                if parsed.is_empty() {
                    return Err(syn::Error::new_spanned(
                        list,
                        "`env(...)` expects at least one symbol",
                    ));
                }

                env.extend(parsed);
            }

            // explicit CDK guard = ...
            //
            // We still forward it, but track that it exists so validation can ban combinations.
            Meta::NameValue(nv) if nv.path.is_ident("guard") => {
                user_guard = true;
                forwarded.push(quote!(#nv));
            }

            // Everything else is forwarded to the CDK attribute unchanged.
            _ => forwarded.push(quote!(#meta)),
        }
    }

    Ok(ParsedArgs {
        forwarded,
        app_guard,
        user_guard,
        auth,
        env,
        rules,
    })
}
const fn empty() -> ParsedArgs {
    ParsedArgs {
        forwarded: Vec::new(),
        app_guard: false,
        user_guard: false,
        auth: None,
        env: Vec::new(),
        rules: Vec::new(),
    }
}

fn parse_auth_symbols(list: &syn::MetaList) -> syn::Result<Vec<AuthSymbol>> {
    let rules = Punctuated::<Path, Token![,]>::parse_terminated
        .parse2(list.tokens.clone())?
        .into_iter()
        .map(parse_auth_symbol)
        .collect::<syn::Result<Vec<_>>>()?;

    if rules.is_empty() {
        return Err(syn::Error::new_spanned(
            list,
            "authorization requires at least one symbol",
        ));
    }

    Ok(rules)
}

fn parse_guard_symbol(path: Path) -> syn::Result<GuardSymbol> {
    let ident = path_ident(&path)?;
    match ident.to_string().as_str() {
        "app_is_live" => Ok(GuardSymbol::AppIsLive),
        _ => Err(syn::Error::new_spanned(path, "unknown guard DSL symbol")),
    }
}

fn parse_auth_symbol(path: Path) -> syn::Result<AuthSymbol> {
    let ident = path_ident(&path)?;
    match ident.to_string().as_str() {
        "caller_is_controller" => Ok(AuthSymbol::CallerIsController),
        "caller_is_parent" => Ok(AuthSymbol::CallerIsParent),
        "caller_is_child" => Ok(AuthSymbol::CallerIsChild),
        "caller_is_root" => Ok(AuthSymbol::CallerIsRoot),
        "caller_is_same_canister" => Ok(AuthSymbol::CallerIsSameCanister),
        "caller_is_registered_to_subnet" => Ok(AuthSymbol::CallerIsRegisteredToSubnet),
        "caller_is_whitelisted" => Ok(AuthSymbol::CallerIsWhitelisted),
        _ => Err(syn::Error::new_spanned(path, "unknown auth DSL symbol")),
    }
}

fn parse_env_symbol(path: Path) -> syn::Result<EnvSymbol> {
    let ident = path_ident(&path)?;
    match ident.to_string().as_str() {
        "self_is_prime_subnet" => Ok(EnvSymbol::SelfIsPrimeSubnet),
        "self_is_prime_root" => Ok(EnvSymbol::SelfIsPrimeRoot),
        _ => Err(syn::Error::new_spanned(path, "unknown env DSL symbol")),
    }
}

fn path_ident(path: &Path) -> syn::Result<&Ident> {
    for segment in &path.segments {
        if !segment.arguments.is_empty() {
            return Err(syn::Error::new_spanned(
                path,
                "DSL symbols do not accept arguments",
            ));
        }
    }

    path.segments
        .last()
        .map(|segment| &segment.ident)
        .ok_or_else(|| syn::Error::new_spanned(path, "expected a DSL symbol"))
}

fn conflicting_auth(list: &syn::MetaList) -> syn::Error {
    syn::Error::new_spanned(list, "conflicting authorization composition")
}
