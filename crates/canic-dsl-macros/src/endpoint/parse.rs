use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Ident, Meta, Path, Token, parse::Parser, punctuated::Punctuated};

//
// ============================================================================
// parse — attribute grammar only (symbolic DSL)
// ============================================================================
//

//
// GuardSymbol
//

#[derive(Clone, Debug)]
pub enum GuardSymbol {
    AppIsLive,
}

//
// AuthSymbol
//

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

//
// EnvSymbol
//

#[derive(Clone, Debug)]
pub enum EnvSymbol {
    SelfIsPrimeSubnet,
    SelfIsPrimeRoot,
}

//
// RuleSymbol
//

#[derive(Clone, Debug)]
pub enum RuleSymbol {
    BuildIcOnly,
    BuildLocalOnly,
}

//
// ParsedArgs
//

#[derive(Debug)]
pub struct ParsedArgs {
    pub forwarded: Vec<TokenStream2>,
    pub guard: Vec<GuardSymbol>,
    pub auth: Vec<AuthSymbol>,
    pub env: Vec<EnvSymbol>,
    pub rules: Vec<RuleSymbol>,
}

pub fn parse_args(attr: TokenStream2) -> syn::Result<ParsedArgs> {
    let Ok(metas) = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(attr.clone()) else {
        if attr.is_empty() {
            return Ok(empty());
        }

        return Ok(ParsedArgs {
            forwarded: vec![attr],
            ..empty()
        });
    };

    let mut forwarded = Vec::new();
    let mut guard = Vec::new();
    let mut auth = Vec::new();
    let mut env = Vec::new();
    let mut rules = Vec::new();

    for meta in metas {
        match meta {
            Meta::List(list) if list.path.is_ident("app") => {
                return Err(syn::Error::new_spanned(
                    list,
                    "app DSL is not supported; use guard instead",
                ));
            }

            // guard(...)
            Meta::List(list) if list.path.is_ident("guard") => {
                let symbols = parse_paths(&list)?;
                let parsed: Vec<GuardSymbol> = symbols
                    .into_iter()
                    .map(parse_guard_symbol)
                    .collect::<syn::Result<_>>()?;

                guard.extend(parsed);
            }

            // auth(...)
            Meta::List(list) if list.path.is_ident("auth") => {
                let symbols = parse_paths(&list)?;
                let parsed: Vec<AuthSymbol> = symbols
                    .into_iter()
                    .map(parse_auth_symbol)
                    .collect::<syn::Result<_>>()?;

                auth.extend(parsed);
            }

            // env(...)
            Meta::List(list) if list.path.is_ident("env") => {
                let symbols = parse_paths(&list)?;
                let parsed: Vec<EnvSymbol> = symbols
                    .into_iter()
                    .map(parse_env_symbol)
                    .collect::<syn::Result<_>>()?;

                env.extend(parsed);
            }

            // rule(...)
            Meta::List(list) if list.path.is_ident("rule") => {
                let symbols = parse_paths(&list)?;
                let parsed: Vec<RuleSymbol> = symbols
                    .into_iter()
                    .map(parse_rule_symbol)
                    .collect::<syn::Result<_>>()?;

                rules.extend(parsed);
            }

            // Everything else is forwarded untouched
            other => forwarded.push(quote!(#other)),
        }
    }

    Ok(ParsedArgs {
        forwarded,
        guard,
        auth,
        env,
        rules,
    })
}

const fn empty() -> ParsedArgs {
    ParsedArgs {
        forwarded: Vec::new(),
        guard: Vec::new(),
        auth: Vec::new(),
        env: Vec::new(),
        rules: Vec::new(),
    }
}

//
// ---------------------------------------------------------------------------
// Symbol parsing helpers
// ---------------------------------------------------------------------------
//

fn parse_paths(list: &syn::MetaList) -> syn::Result<Vec<Path>> {
    let paths = Punctuated::<Path, Token![,]>::parse_terminated
        .parse2(list.tokens.clone())?
        .into_iter()
        .collect::<Vec<_>>();

    if paths.is_empty() {
        return Err(syn::Error::new_spanned(
            list,
            "expected at least one DSL symbol",
        ));
    }

    Ok(paths)
}

fn parse_guard_symbol(path: Path) -> syn::Result<GuardSymbol> {
    match path_ident(&path)?.to_string().as_str() {
        "app_is_live" => Ok(GuardSymbol::AppIsLive),
        _ => Err(unknown_symbol(path, "guard")),
    }
}

fn parse_auth_symbol(path: Path) -> syn::Result<AuthSymbol> {
    match path_ident(&path)?.to_string().as_str() {
        "caller_is_controller" => Ok(AuthSymbol::CallerIsController),
        "caller_is_parent" => Ok(AuthSymbol::CallerIsParent),
        "caller_is_child" => Ok(AuthSymbol::CallerIsChild),
        "caller_is_root" => Ok(AuthSymbol::CallerIsRoot),
        "caller_is_same_canister" => Ok(AuthSymbol::CallerIsSameCanister),
        "caller_is_registered_to_subnet" => Ok(AuthSymbol::CallerIsRegisteredToSubnet),
        "caller_is_whitelisted" => Ok(AuthSymbol::CallerIsWhitelisted),
        _ => Err(unknown_symbol(path, "auth")),
    }
}

fn parse_env_symbol(path: Path) -> syn::Result<EnvSymbol> {
    match path_ident(&path)?.to_string().as_str() {
        "self_is_prime_subnet" => Ok(EnvSymbol::SelfIsPrimeSubnet),
        "self_is_prime_root" => Ok(EnvSymbol::SelfIsPrimeRoot),
        _ => Err(unknown_symbol(path, "env")),
    }
}

fn parse_rule_symbol(path: Path) -> syn::Result<RuleSymbol> {
    match path_ident(&path)?.to_string().as_str() {
        "build_ic_only" => Ok(RuleSymbol::BuildIcOnly),
        "build_local_only" => Ok(RuleSymbol::BuildLocalOnly),
        _ => Err(unknown_symbol(path, "rule")),
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

fn unknown_symbol(path: Path, category: &str) -> syn::Error {
    syn::Error::new_spanned(path, format!("unknown {category} DSL symbol"))
}
