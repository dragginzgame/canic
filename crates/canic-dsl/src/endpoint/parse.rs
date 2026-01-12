use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Expr, Meta, Token, parse::Parser, punctuated::Punctuated};

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
    Any(Vec<Expr>),
    All(Vec<Expr>),
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
    pub env: Vec<Expr>,
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
    let mut env = Vec::<Expr>::new();
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
                        "`guard(...)` expects at least one argument (e.g., `guard(app)`)",
                    ));
                }

                // Only guard(app) is supported.
                for item in inner {
                    match item {
                        Meta::Path(p) if p.is_ident("app") => {
                            app_guard = true;
                        }
                        other => {
                            return Err(syn::Error::new_spanned(
                                other,
                                "only `guard(app)` is supported",
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
                let rules = parse_rules(&list)?;
                auth = Some(AuthSpec::Any(rules));
            }

            // auth_all(...)
            Meta::List(list) if list.path.is_ident("auth_all") => {
                if auth.is_some() {
                    return Err(conflicting_auth(&list));
                }
                let rules = parse_rules(&list)?;
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
            // Parse as Expr so you can do env(is_prime_subnet), env(is_root), etc.
            Meta::List(list) if list.path.is_ident("env") => {
                let parsed = Punctuated::<Expr, Token![,]>::parse_terminated
                    .parse2(list.tokens.clone())?
                    .into_iter()
                    .collect::<Vec<_>>();

                if parsed.is_empty() {
                    return Err(syn::Error::new_spanned(
                        list,
                        "`env(...)` expects at least one expression",
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

fn parse_rules(list: &syn::MetaList) -> syn::Result<Vec<Expr>> {
    let rules = Punctuated::<Expr, Token![,]>::parse_terminated
        .parse2(list.tokens.clone())?
        .into_iter()
        .collect::<Vec<_>>();

    if rules.is_empty() {
        return Err(syn::Error::new_spanned(
            list,
            "authorization requires at least one rule",
        ));
    }

    Ok(rules)
}

fn conflicting_auth(list: &syn::MetaList) -> syn::Error {
    syn::Error::new_spanned(list, "conflicting authorization composition")
}
