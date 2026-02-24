use proc_macro2::TokenStream as TokenStream2;
use syn::{Expr, Ident, Meta, Path, Token, parse::Parser, punctuated::Punctuated};

//
// ============================================================================
// parse - attribute grammar (expression-only access DSL)
// ============================================================================
//
// Allowed access DSL:
//
//   #[canic_update(requires(...))]
//
// `requires(...)` is the only access-control surface.
// All access semantics are expressed as AccessExprAst.
//

///
/// BuiltinPredicate
///

#[derive(Clone, Debug)]
pub enum BuiltinPredicate {
    AppAllowsUpdates,
    AppIsQueryable,
    SelfIsPrimeSubnet,
    SelfIsPrimeRoot,
    CallerIsController,
    CallerIsParent,
    CallerIsChild,
    CallerIsRoot,
    CallerIsSameCanister,
    CallerIsRegisteredToSubnet,
    CallerIsWhitelisted,
    Authenticated { required_scope: Option<String> },
    BuildIcOnly,
    BuildLocalOnly,
}

///
/// AccessExprAst
///

#[derive(Clone, Debug)]
pub enum AccessExprAst {
    All(Vec<Self>),
    Any(Vec<Self>),
    Not(Box<Self>),
    Pred(AccessPredicateAst),
}

///
/// AccessPredicateAst
///

#[derive(Clone, Debug)]
pub enum AccessPredicateAst {
    Builtin(BuiltinPredicate),
    Custom(TokenStream2),
}

//
// ParsedArgs
//

#[derive(Debug)]
pub struct ParsedArgs {
    pub forwarded: Vec<TokenStream2>,
    pub requires: Vec<AccessExprAst>,
    pub requires_async: bool,
    pub requires_fallible: bool,
    pub internal: bool,
}

pub fn parse_args(attr: TokenStream2) -> syn::Result<ParsedArgs> {
    if attr.is_empty() {
        return Ok(empty());
    }

    let metas = Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(attr.clone())
        .map_err(|_| syn::Error::new_spanned(&attr, "expected requires(...)"))?;

    let mut requires = Vec::new();
    let mut internal = false;

    for meta in metas {
        match meta {
            Meta::List(list) if list.path.is_ident("requires") => {
                requires.push(parse_requires(&list)?);
            }
            Meta::Path(path) if path.is_ident("internal") => {
                if internal {
                    return Err(syn::Error::new_spanned(
                        path,
                        "internal endpoint marker must appear only once",
                    ));
                }
                internal = true;
            }
            Meta::NameValue(nv) if nv.path.is_ident("internal") => {
                if internal {
                    return Err(syn::Error::new_spanned(
                        nv,
                        "internal endpoint marker must appear only once",
                    ));
                }
                let value = match &nv.value {
                    Expr::Lit(expr) => match &expr.lit {
                        syn::Lit::Bool(lit) => lit.value,
                        _ => {
                            return Err(syn::Error::new_spanned(
                                nv,
                                "internal must be set to a boolean literal",
                            ));
                        }
                    },
                    _ => {
                        return Err(syn::Error::new_spanned(
                            nv,
                            "internal must be set to a boolean literal",
                        ));
                    }
                };
                if !value {
                    return Err(syn::Error::new_spanned(
                        nv,
                        "internal must be true when specified",
                    ));
                }
                internal = true;
            }
            Meta::List(list) => {
                return Err(syn::Error::new_spanned(
                    list,
                    "unsupported access clause; use requires(...)",
                ));
            }
            Meta::Path(path) => {
                return Err(syn::Error::new_spanned(
                    path,
                    "access control must be expressed via requires(...) or internal",
                ));
            }
            Meta::NameValue(nv) => {
                return Err(syn::Error::new_spanned(
                    nv,
                    "access control must be expressed via requires(...) or internal",
                ));
            }
        }
    }

    if requires.is_empty() && !internal {
        return Err(syn::Error::new_spanned(attr, "expected requires(...)"));
    }

    let requires_async = !requires.is_empty();
    let requires_fallible = !requires.is_empty();

    Ok(ParsedArgs {
        forwarded: Vec::new(),
        requires,
        requires_async,
        requires_fallible,
        internal,
    })
}

const fn empty() -> ParsedArgs {
    ParsedArgs {
        forwarded: Vec::new(),
        requires: Vec::new(),
        requires_async: false,
        requires_fallible: false,
        internal: false,
    }
}

//
// ---------------------------------------------------------------------------
// Access expression parsing helpers
// ---------------------------------------------------------------------------
//
fn parse_requires(list: &syn::MetaList) -> syn::Result<AccessExprAst> {
    let exprs = parse_expr_list(&list.tokens)?;
    Ok(AccessExprAst::All(exprs))
}

fn parse_expr_list(tokens: &TokenStream2) -> syn::Result<Vec<AccessExprAst>> {
    let exprs = Punctuated::<Expr, Token![,]>::parse_terminated
        .parse2(tokens.clone())
        .map_err(|_| {
            syn::Error::new_spanned(
                tokens,
                "expected a comma-separated list of access expressions",
            )
        })?;

    if exprs.is_empty() {
        return Err(syn::Error::new_spanned(
            tokens,
            "expected at least one access expression",
        ));
    }

    exprs.into_iter().map(parse_expr).collect()
}

fn parse_expr(expr: Expr) -> syn::Result<AccessExprAst> {
    match expr {
        Expr::Call(call) => parse_call_expr(call),
        other => Err(syn::Error::new_spanned(
            other,
            "expected access expression call (all/any/not/custom or built-in predicate)",
        )),
    }
}

#[expect(clippy::too_many_lines)]
fn parse_call_expr(call: syn::ExprCall) -> syn::Result<AccessExprAst> {
    let path = match *call.func {
        Expr::Path(expr) => expr.path,
        other => {
            return Err(syn::Error::new_spanned(
                other,
                "access expressions must be path-based calls",
            ));
        }
    };

    let name = path_ident(&path)?.to_string();
    let mut args = call.args.into_iter();

    match name.as_str() {
        "all" | "requires" => {
            let exprs = parse_expr_args(args)?;
            Ok(AccessExprAst::All(exprs))
        }
        "any" => {
            let exprs = parse_expr_args(args)?;
            Ok(AccessExprAst::Any(exprs))
        }
        "not" => {
            let expr = args
                .next()
                .ok_or_else(|| syn::Error::new_spanned(&path, "not(...) requires one argument"))?;
            if args.next().is_some() {
                return Err(syn::Error::new_spanned(
                    &path,
                    "not(...) accepts exactly one argument",
                ));
            }
            Ok(AccessExprAst::Not(Box::new(parse_expr(expr)?)))
        }
        "custom" => {
            let expr = args.next().ok_or_else(|| {
                syn::Error::new_spanned(&path, "custom(...) requires one argument")
            })?;
            if args.next().is_some() {
                return Err(syn::Error::new_spanned(
                    &path,
                    "custom(...) accepts exactly one argument",
                ));
            }
            Ok(AccessExprAst::Pred(AccessPredicateAst::Custom(
                quote::quote!(#expr),
            )))
        }
        _ => {
            if is_authenticated_path(&path) {
                let required_scope = match args.next() {
                    None => None,
                    Some(scope_expr) => {
                        if args.next().is_some() {
                            return Err(syn::Error::new_spanned(
                                &path,
                                "authenticated(...) accepts zero arguments or one string literal scope",
                            ));
                        }
                        let scope = match scope_expr {
                            Expr::Lit(expr_lit) => match &expr_lit.lit {
                                syn::Lit::Str(scope_lit) => scope_lit.value(),
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        expr_lit,
                                        "authenticated(...) scope must be a string literal",
                                    ));
                                }
                            },
                            other => {
                                return Err(syn::Error::new_spanned(
                                    other,
                                    "authenticated(...) scope must be a string literal",
                                ));
                            }
                        };
                        if scope.trim().is_empty() {
                            return Err(syn::Error::new_spanned(
                                &path,
                                "authenticated(...) scope must not be empty",
                            ));
                        }
                        Some(scope)
                    }
                };
                return Ok(AccessExprAst::Pred(AccessPredicateAst::Builtin(
                    BuiltinPredicate::Authenticated { required_scope },
                )));
            }

            if args.next().is_some() {
                return Err(syn::Error::new_spanned(
                    &path,
                    "built-in predicates do not accept arguments",
                ));
            }
            let builtin = builtin_from_path(&path).ok_or_else(|| {
                if builtin_from_path_tail(&path).is_some() || is_authenticated_path(&path) {
                    return syn::Error::new_spanned(
                        &path,
                        "built-in predicates must use short paths like auth::authenticated()",
                    );
                }
                syn::Error::new_spanned(
                    &path,
                    "unknown access predicate; expected built-in predicate or any/all/not/custom",
                )
            })?;
            Ok(AccessExprAst::Pred(AccessPredicateAst::Builtin(builtin)))
        }
    }
}

fn parse_expr_args<I>(args: I) -> syn::Result<Vec<AccessExprAst>>
where
    I: IntoIterator<Item = Expr>,
{
    let mut out = Vec::new();
    for expr in args {
        out.push(parse_expr(expr)?);
    }
    if out.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "expected at least one access expression",
        ));
    }
    Ok(out)
}

//
// ---------------------------------------------------------------------------
// Built-in predicate resolution
// ---------------------------------------------------------------------------
//

fn builtin_from_path(path: &Path) -> Option<BuiltinPredicate> {
    if path.leading_colon.is_some() {
        return None;
    }
    if path.segments.len() == 1 {
        return None;
    }
    if path.segments.len() != 2 {
        return None;
    }
    builtin_from_path_tail(path)
}

fn builtin_from_path_tail(path: &Path) -> Option<BuiltinPredicate> {
    let mut names = path.segments.iter().map(|seg| seg.ident.to_string());
    let last = names.next_back()?;
    let module = names.next_back();

    match (module.as_deref(), last.as_str()) {
        (Some("app"), "allows_updates") => Some(BuiltinPredicate::AppAllowsUpdates),
        (Some("app"), "is_queryable") => Some(BuiltinPredicate::AppIsQueryable),
        (Some("env"), "is_prime_subnet") => Some(BuiltinPredicate::SelfIsPrimeSubnet),
        (Some("env"), "is_prime_root") => Some(BuiltinPredicate::SelfIsPrimeRoot),
        (Some("caller"), "is_controller") => Some(BuiltinPredicate::CallerIsController),
        (Some("caller"), "is_parent") => Some(BuiltinPredicate::CallerIsParent),
        (Some("caller"), "is_child") => Some(BuiltinPredicate::CallerIsChild),
        (Some("caller"), "is_root") => Some(BuiltinPredicate::CallerIsRoot),
        (Some("caller"), "is_same_canister") => Some(BuiltinPredicate::CallerIsSameCanister),
        (Some("caller"), "is_registered_to_subnet") => {
            Some(BuiltinPredicate::CallerIsRegisteredToSubnet)
        }
        (Some("caller"), "is_whitelisted") => Some(BuiltinPredicate::CallerIsWhitelisted),
        (Some("env"), "build_ic_only") => Some(BuiltinPredicate::BuildIcOnly),
        (Some("env"), "build_local_only") => Some(BuiltinPredicate::BuildLocalOnly),
        _ => None,
    }
}

fn is_authenticated_path(path: &Path) -> bool {
    if path.leading_colon.is_some() {
        return false;
    }

    if path.segments.len() == 1 {
        return path
            .segments
            .last()
            .is_some_and(|seg| seg.ident == "authenticated");
    }

    if path.segments.len() != 2 {
        return false;
    }

    let mut names = path.segments.iter().map(|seg| seg.ident.to_string());
    let last = names.next_back();
    let module = names.next_back();
    matches!(
        (module.as_deref(), last.as_deref()),
        (Some("auth"), Some("authenticated"))
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn authenticated_allows_no_scope_argument() {
        let parsed = parse_args(quote!(requires(auth::authenticated()))).expect("parse args");
        let AccessExprAst::All(exprs) = &parsed.requires[0] else {
            panic!("expected requires(all)");
        };
        let AccessExprAst::Pred(AccessPredicateAst::Builtin(BuiltinPredicate::Authenticated {
            required_scope,
        })) = &exprs[0]
        else {
            panic!("expected authenticated predicate");
        };
        assert!(required_scope.is_none());
    }

    #[test]
    fn authenticated_allows_string_scope_argument() {
        let parsed =
            parse_args(quote!(requires(auth::authenticated("scope:test")))).expect("parse args");
        let AccessExprAst::All(exprs) = &parsed.requires[0] else {
            panic!("expected requires(all)");
        };
        let AccessExprAst::Pred(AccessPredicateAst::Builtin(BuiltinPredicate::Authenticated {
            required_scope,
        })) = &exprs[0]
        else {
            panic!("expected authenticated predicate");
        };
        assert_eq!(required_scope.as_deref(), Some("scope:test"));
    }

    #[test]
    fn authenticated_rejects_multiple_arguments() {
        let err = parse_args(quote!(requires(auth::authenticated("a", "b"))))
            .expect_err("authenticated with two args must fail");
        assert!(
            err.to_string()
                .contains("authenticated(...) accepts zero arguments or one string literal scope")
        );
    }
}
