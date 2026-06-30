use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Expr, Ident, LitStr, Meta, MetaNameValue, Path, Token, parse::Parser, punctuated::Punctuated,
};

const ENDPOINT_ATTR_HELP: &str = "endpoint attributes must be expressed via requires(...), public, payload(...), internal, composite, or name = \"...\"";

//
// ============================================================================
// parse - attribute grammar (expression-only access DSL)
// ============================================================================
//
// Allowed access DSL:
//
//   #[canic_update(requires(...))]
//
// `requires(...)` is the only gated access-control surface.
// Intentionally open endpoints must opt into `public`.
//

///
/// AuthScopeArg
///

#[derive(Clone, Debug)]
pub enum AuthScopeArg {
    Literal(String),
    Expr(TokenStream2),
}

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
    Authenticated {
        required_scope: Option<AuthScopeArg>,
    },
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

///
/// QueryMode
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryMode {
    Plain,
    Composite,
}

impl QueryMode {
    pub const fn is_composite(self) -> bool {
        matches!(self, Self::Composite)
    }
}

///
/// ParsedArgs
///

#[derive(Debug)]
pub struct ParsedArgs {
    pub forwarded: Vec<TokenStream2>,
    pub export_name: Option<LitStr>,
    pub payload_max_bytes: Option<TokenStream2>,
    pub requires: Vec<AccessExprAst>,
    pub internal: bool,
    pub public: bool,
    pub query_mode: QueryMode,
}

#[expect(clippy::too_many_lines)]
pub fn parse_args(attr: TokenStream2) -> syn::Result<ParsedArgs> {
    if attr.is_empty() {
        return Ok(empty());
    }

    let metas = Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(attr.clone())
        .map_err(|_| syn::Error::new_spanned(&attr, "expected requires(...)"))?;

    let mut forwarded = Vec::new();
    let mut requires = Vec::new();
    let mut internal = false;
    let mut public = false;
    let mut saw_name = false;
    let mut query_mode = QueryMode::Plain;
    let mut export_name = None;
    let mut payload_max_bytes = None;

    for meta in metas {
        match meta {
            Meta::List(list) if list.path.is_ident("requires") => {
                requires.push(parse_requires(&list)?);
            }
            Meta::List(list) if list.path.is_ident("payload") => {
                if payload_max_bytes.is_some() {
                    return Err(syn::Error::new_spanned(
                        list,
                        "payload(...) must appear only once",
                    ));
                }
                payload_max_bytes = Some(parse_payload_max_bytes(&list)?);
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
            Meta::Path(path) if path.is_ident("public") => {
                if public {
                    return Err(syn::Error::new_spanned(
                        path,
                        "public endpoint marker must appear only once",
                    ));
                }
                public = true;
            }
            Meta::Path(path) if path.is_ident("composite") => {
                if query_mode.is_composite() {
                    return Err(syn::Error::new_spanned(
                        path,
                        "composite query marker must appear only once",
                    ));
                }
                forwarded.push(quote!(composite = true));
                query_mode = QueryMode::Composite;
            }
            Meta::NameValue(nv) if nv.path.is_ident("name") => {
                if saw_name {
                    return Err(syn::Error::new_spanned(
                        nv,
                        "endpoint export name must appear only once",
                    ));
                }

                let value = parse_string_literal(&nv, "endpoint export name")?;

                forwarded.push(quote!(name = #value));
                export_name = Some(value.clone());
                saw_name = true;
            }
            Meta::NameValue(nv) if nv.path.is_ident("internal") => {
                if internal {
                    return Err(syn::Error::new_spanned(
                        nv,
                        "internal endpoint marker must appear only once",
                    ));
                }
                parse_true_marker(&nv, "internal")?;
                internal = true;
            }
            Meta::NameValue(nv) if nv.path.is_ident("public") => {
                if public {
                    return Err(syn::Error::new_spanned(
                        nv,
                        "public endpoint marker must appear only once",
                    ));
                }
                parse_true_marker(&nv, "public")?;
                public = true;
            }
            Meta::NameValue(nv) if nv.path.is_ident("composite") => {
                if query_mode.is_composite() {
                    return Err(syn::Error::new_spanned(
                        nv,
                        "composite query marker must appear only once",
                    ));
                }
                parse_true_marker(&nv, "composite")?;
                forwarded.push(quote!(composite = true));
                query_mode = QueryMode::Composite;
            }
            Meta::List(list) => {
                return Err(syn::Error::new_spanned(
                    list,
                    "unsupported endpoint clause; use requires(...) or payload(...)",
                ));
            }
            Meta::Path(path) => {
                return Err(syn::Error::new_spanned(path, ENDPOINT_ATTR_HELP));
            }
            Meta::NameValue(nv) => {
                return Err(syn::Error::new_spanned(nv, ENDPOINT_ATTR_HELP));
            }
        }
    }

    if requires.is_empty()
        && !internal
        && !public
        && forwarded.is_empty()
        && payload_max_bytes.is_none()
    {
        return Err(syn::Error::new_spanned(
            attr,
            "expected requires(...), public, internal, composite, name = \"...\", or payload(...)",
        ));
    }

    Ok(ParsedArgs {
        forwarded,
        export_name,
        payload_max_bytes,
        requires,
        internal,
        public,
        query_mode,
    })
}

fn parse_string_literal<'a>(nv: &'a MetaNameValue, label: &'static str) -> syn::Result<&'a LitStr> {
    if let Expr::Lit(expr) = &nv.value
        && let syn::Lit::Str(lit) = &expr.lit
    {
        return Ok(lit);
    }

    Err(syn::Error::new_spanned(
        nv,
        format!("{label} must be a string literal"),
    ))
}

fn parse_true_marker(nv: &MetaNameValue, marker: &'static str) -> syn::Result<()> {
    let value = match &nv.value {
        Expr::Lit(expr) => match &expr.lit {
            syn::Lit::Bool(lit) => lit.value,
            _ => {
                return Err(syn::Error::new_spanned(
                    nv,
                    format!("{marker} must be set to a boolean literal"),
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                nv,
                format!("{marker} must be set to a boolean literal"),
            ));
        }
    };

    if value {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(
            nv,
            format!("{marker} must be true when specified"),
        ))
    }
}

const fn empty() -> ParsedArgs {
    ParsedArgs {
        forwarded: Vec::new(),
        export_name: None,
        payload_max_bytes: None,
        requires: Vec::new(),
        internal: false,
        public: false,
        query_mode: QueryMode::Plain,
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

fn parse_payload_max_bytes(list: &syn::MetaList) -> syn::Result<TokenStream2> {
    let metas = Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .map_err(|_| {
            syn::Error::new_spanned(list, "expected payload(max_bytes = <usize expression>)")
        })?;

    let mut max_bytes = None;

    for meta in metas {
        match meta {
            Meta::NameValue(nv) if nv.path.is_ident("max_bytes") => {
                if max_bytes.is_some() {
                    return Err(syn::Error::new_spanned(
                        nv,
                        "payload max_bytes must appear only once",
                    ));
                }
                let value = nv.value;
                max_bytes = Some(quote!(#value));
            }
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "expected payload(max_bytes = <usize expression>)",
                ));
            }
        }
    }

    max_bytes.ok_or_else(|| {
        syn::Error::new_spanned(list, "payload(...) requires max_bytes = <usize expression>")
    })
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
        Expr::Group(group) => parse_expr(*group.expr),
        Expr::Paren(paren) => parse_expr(*paren.expr),
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
                                "authenticated(...) accepts zero arguments or one string literal/path scope",
                            ));
                        }
                        let scope = match scope_expr {
                            Expr::Lit(expr_lit) => match &expr_lit.lit {
                                syn::Lit::Str(scope_lit) => {
                                    let value = scope_lit.value();
                                    if value.trim().is_empty() {
                                        return Err(syn::Error::new_spanned(
                                            &path,
                                            "authenticated(...) scope must not be empty",
                                        ));
                                    }
                                    AuthScopeArg::Literal(value)
                                }
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        expr_lit,
                                        "authenticated(...) scope must be a string literal or path constant",
                                    ));
                                }
                            },
                            Expr::Path(expr_path) => AuthScopeArg::Expr(quote::quote!(#expr_path)),
                            other => {
                                return Err(syn::Error::new_spanned(
                                    other,
                                    "authenticated(...) scope must be a string literal or path constant",
                                ));
                            }
                        };
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
                if builtin_from_path_tail(&path).is_some()
                    || is_authenticated_path(&path)
                    || is_bare_authenticated_path(&path)
                {
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
    short_path_tail(path)?;
    builtin_from_path_tail(path)
}

fn builtin_from_path_tail(path: &Path) -> Option<BuiltinPredicate> {
    let (module, last) = path_tail(path)?;
    let module = module.to_string();
    let last = last.to_string();

    match (module.as_str(), last.as_str()) {
        ("app", "allows_updates") => Some(BuiltinPredicate::AppAllowsUpdates),
        ("app", "is_queryable") => Some(BuiltinPredicate::AppIsQueryable),
        ("env", "is_prime_subnet") => Some(BuiltinPredicate::SelfIsPrimeSubnet),
        ("env", "is_prime_root") => Some(BuiltinPredicate::SelfIsPrimeRoot),
        ("caller", "is_controller") => Some(BuiltinPredicate::CallerIsController),
        ("caller", "is_parent") => Some(BuiltinPredicate::CallerIsParent),
        ("caller", "is_child") => Some(BuiltinPredicate::CallerIsChild),
        ("caller", "is_root") => Some(BuiltinPredicate::CallerIsRoot),
        ("caller", "is_same_canister") => Some(BuiltinPredicate::CallerIsSameCanister),
        ("caller", "is_registered_to_subnet") => Some(BuiltinPredicate::CallerIsRegisteredToSubnet),
        ("caller", "is_whitelisted") => Some(BuiltinPredicate::CallerIsWhitelisted),
        ("env", "build_ic_only") => Some(BuiltinPredicate::BuildIcOnly),
        ("env", "build_local_only") => Some(BuiltinPredicate::BuildLocalOnly),
        _ => None,
    }
}

fn is_authenticated_path(path: &Path) -> bool {
    short_path_is(path, "auth", "authenticated")
}

fn is_bare_authenticated_path(path: &Path) -> bool {
    if path.leading_colon.is_some() {
        return false;
    }

    path.segments.len() == 1
        && path
            .segments
            .last()
            .is_some_and(|seg| seg.ident == "authenticated")
}

fn path_tail(path: &Path) -> Option<(&Ident, &Ident)> {
    if path.leading_colon.is_some() {
        return None;
    }

    let mut segments = path.segments.iter().rev();
    let last = &segments.next()?.ident;
    let module = &segments.next()?.ident;
    Some((module, last))
}

fn short_path_tail(path: &Path) -> Option<(&Ident, &Ident)> {
    if path.segments.len() == 2 {
        path_tail(path)
    } else {
        None
    }
}

fn short_path_is(path: &Path, module: &str, last: &str) -> bool {
    short_path_tail(path)
        .is_some_and(|(found_module, found_last)| found_module == module && found_last == last)
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
mod tests;
