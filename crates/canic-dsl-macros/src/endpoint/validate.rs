use crate::endpoint::parse::{AccessExprAst, AccessPredicateAst, BuiltinPredicate, ParsedArgs};
use proc_macro2::TokenStream as TokenStream2;
use syn::{FnArg, Signature, Type};

///
/// ValidatedArgs
///
/// Arguments validated for macro expansion.
///
/// This phase enforces only *structural* invariants:
/// - async requirements
/// - fallible return requirements
/// - authenticated predicate argument shape
/// - internal-only predicate usage
///
/// It does NOT interpret access semantics beyond structural checks.
///

#[derive(Debug)]
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

    if requires_authenticated(&parsed.requires) {
        validate_authenticated_args(sig)?;
    }

    if !parsed.internal && contains_registered_to_subnet(&parsed.requires) {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "caller::is_registered_to_subnet() is internal-only; mark the endpoint as `internal` or use caller::is_parent()/caller::is_child()/caller::is_root()",
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

fn requires_authenticated(requires: &[AccessExprAst]) -> bool {
    requires.iter().any(access_expr_contains_authenticated)
}

fn access_expr_contains_authenticated(expr: &AccessExprAst) -> bool {
    match expr {
        AccessExprAst::All(exprs) | AccessExprAst::Any(exprs) => {
            exprs.iter().any(access_expr_contains_authenticated)
        }
        AccessExprAst::Not(expr) => access_expr_contains_authenticated(expr),
        AccessExprAst::Pred(AccessPredicateAst::Builtin(BuiltinPredicate::Authenticated {
            ..
        })) => true,
        AccessExprAst::Pred(AccessPredicateAst::Builtin(_) | AccessPredicateAst::Custom(_)) => {
            false
        }
    }
}

fn contains_registered_to_subnet(requires: &[AccessExprAst]) -> bool {
    requires
        .iter()
        .any(access_expr_contains_registered_to_subnet)
}

fn access_expr_contains_registered_to_subnet(expr: &AccessExprAst) -> bool {
    match expr {
        AccessExprAst::All(exprs) | AccessExprAst::Any(exprs) => {
            exprs.iter().any(access_expr_contains_registered_to_subnet)
        }
        AccessExprAst::Not(expr) => access_expr_contains_registered_to_subnet(expr),
        AccessExprAst::Pred(AccessPredicateAst::Builtin(
            BuiltinPredicate::CallerIsRegisteredToSubnet,
        )) => true,
        AccessExprAst::Pred(AccessPredicateAst::Builtin(_) | AccessPredicateAst::Custom(_)) => {
            false
        }
    }
}

fn validate_authenticated_args(sig: &Signature) -> syn::Result<()> {
    let Some(first) = sig.inputs.first() else {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "is_authenticated(...) requires a first argument of type `DelegatedToken`",
        ));
    };

    let first_ty = match first {
        FnArg::Typed(pat) => pat.ty.as_ref(),
        FnArg::Receiver(recv) => {
            return Err(syn::Error::new_spanned(
                recv,
                "is_authenticated(...) requires a first argument of type `DelegatedToken`",
            ));
        }
    };

    let Some(ident) = type_ident(first_ty) else {
        return Err(syn::Error::new_spanned(
            first_ty,
            "is_authenticated(...) requires a first argument of type `DelegatedToken`",
        ));
    };

    if ident == "DelegatedToken" {
        return Ok(());
    }

    Err(syn::Error::new_spanned(
        first_ty,
        "is_authenticated(...) requires a first argument of type `DelegatedToken`",
    ))
}

fn type_ident(ty: &Type) -> Option<&syn::Ident> {
    match ty {
        Type::Path(ty) => ty.path.segments.last().map(|seg| &seg.ident),
        Type::Reference(ty) => type_ident(&ty.elem),
        Type::Paren(ty) => type_ident(&ty.elem),
        Type::Group(ty) => type_ident(&ty.elem),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoint::parse::{AccessExprAst, AccessPredicateAst, BuiltinPredicate, ParsedArgs};

    fn parsed_authenticated() -> ParsedArgs {
        ParsedArgs {
            forwarded: Vec::new(),
            requires: vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
                BuiltinPredicate::Authenticated {
                    required_scope: None,
                },
            ))],
            requires_async: true,
            requires_fallible: true,
            internal: false,
        }
    }

    fn parsed_registered_to_subnet(internal: bool) -> ParsedArgs {
        ParsedArgs {
            forwarded: Vec::new(),
            requires: vec![AccessExprAst::Any(vec![
                AccessExprAst::Pred(AccessPredicateAst::Builtin(
                    BuiltinPredicate::CallerIsController,
                )),
                AccessExprAst::Not(Box::new(AccessExprAst::Pred(AccessPredicateAst::Builtin(
                    BuiltinPredicate::CallerIsRegisteredToSubnet,
                )))),
            ])],
            requires_async: true,
            requires_fallible: true,
            internal,
        }
    }

    #[test]
    fn authenticated_requires_first_argument() {
        let sig: Signature = syn::parse_quote!(async fn hello() -> Result<(), ::canic::Error>);
        let err = validate(parsed_authenticated(), &sig, true).unwrap_err();
        assert!(
            err.to_string()
                .contains("is_authenticated(...) requires a first argument")
        );
    }

    #[test]
    fn authenticated_accepts_delegated_token_first_arg() {
        let sig: Signature = syn::parse_quote!(
            async fn hello(token: ::canic::dto::auth::DelegatedToken) -> Result<(), ::canic::Error>
        );
        validate(parsed_authenticated(), &sig, true).expect("authenticated arg ok");
    }

    #[test]
    fn authenticated_rejects_wrong_first_arg_type() {
        let sig: Signature = syn::parse_quote!(
            async fn hello(user: ::canic::cdk::candid::Principal) -> Result<(), ::canic::Error>
        );
        let err = validate(parsed_authenticated(), &sig, true).unwrap_err();
        assert!(
            err.to_string()
                .contains("is_authenticated(...) requires a first argument")
        );
    }

    #[test]
    fn registered_to_subnet_requires_internal_endpoint() {
        let sig: Signature = syn::parse_quote!(async fn hello() -> Result<(), ::canic::Error>);
        let err = validate(parsed_registered_to_subnet(false), &sig, true).unwrap_err();
        assert!(
            err.to_string()
                .contains("caller::is_registered_to_subnet() is internal-only")
        );
    }

    #[test]
    fn registered_to_subnet_is_allowed_for_internal_endpoint() {
        let sig: Signature = syn::parse_quote!(async fn hello() -> Result<(), ::canic::Error>);
        validate(parsed_registered_to_subnet(true), &sig, true).expect("internal predicate ok");
    }
}
