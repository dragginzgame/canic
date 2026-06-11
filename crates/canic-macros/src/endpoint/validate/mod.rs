use crate::endpoint::{
    EndpointKind,
    parse::{AccessExprAst, AccessPredicateAst, BuiltinPredicate, ParsedArgs, QueryMode},
};
use proc_macro2::TokenStream as TokenStream2;
use syn::{FnArg, LitStr, Signature, Type};

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
    pub export_name: Option<LitStr>,
    pub payload_max_bytes: Option<TokenStream2>,
    pub requires: Vec<AccessExprAst>,
    pub internal: bool,
    pub query_mode: QueryMode,
}

pub fn validate(
    kind: EndpointKind,
    parsed: ParsedArgs,
    sig: &Signature,
    asyncness: bool,
) -> syn::Result<ValidatedArgs> {
    if parsed.payload_max_bytes.is_some() && matches!(kind, EndpointKind::Query) {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "payload(...) is supported only on canic_update endpoints",
        ));
    }

    if parsed.query_mode.is_composite() && matches!(kind, EndpointKind::Update) {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "composite is supported only on canic_query endpoints",
        ));
    }

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

    if !parsed.internal && contains_internal_only_caller_predicate(&parsed.requires) {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "caller topology predicates are internal-only; mark the endpoint as `internal` or use caller::is_parent()/caller::is_child()/caller::is_root()",
        ));
    }

    Ok(ValidatedArgs {
        forwarded: parsed.forwarded,
        export_name: parsed.export_name,
        payload_max_bytes: parsed.payload_max_bytes,
        requires: parsed.requires,
        internal: parsed.internal,
        query_mode: parsed.query_mode,
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

fn contains_internal_only_caller_predicate(requires: &[AccessExprAst]) -> bool {
    requires
        .iter()
        .any(access_expr_contains_internal_only_caller_predicate)
}

fn access_expr_contains_internal_only_caller_predicate(expr: &AccessExprAst) -> bool {
    match expr {
        AccessExprAst::All(exprs) | AccessExprAst::Any(exprs) => exprs
            .iter()
            .any(access_expr_contains_internal_only_caller_predicate),
        AccessExprAst::Not(expr) => access_expr_contains_internal_only_caller_predicate(expr),
        AccessExprAst::Pred(AccessPredicateAst::Builtin(builtin)) => {
            matches!(builtin, BuiltinPredicate::CallerIsRegisteredToSubnet)
        }
        AccessExprAst::Pred(AccessPredicateAst::Custom(_)) => false,
    }
}

fn validate_authenticated_args(sig: &Signature) -> syn::Result<()> {
    let Some(first) = sig.inputs.first() else {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            authenticated_arg_error(),
        ));
    };

    let first_ty = match first {
        FnArg::Typed(pat) => pat.ty.as_ref(),
        FnArg::Receiver(recv) => {
            return Err(syn::Error::new_spanned(recv, authenticated_arg_error()));
        }
    };

    let Some(ident) = type_ident(first_ty) else {
        return Err(syn::Error::new_spanned(first_ty, authenticated_arg_error()));
    };

    if ident == "DelegatedToken" {
        return Ok(());
    }

    Err(syn::Error::new_spanned(first_ty, authenticated_arg_error()))
}

const fn authenticated_arg_error() -> &'static str {
    "authenticated(...) requires a first argument of type `DelegatedToken`"
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
mod tests;
