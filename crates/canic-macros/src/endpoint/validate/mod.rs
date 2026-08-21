use crate::endpoint::{
    EndpointKind,
    parse::{AccessExprAst, AccessPredicateAst, BuiltinPredicate, ParsedArgs, QueryMode},
    returns_fallible,
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
/// - explicit public-vs-gated access shape
///
/// It does NOT interpret access semantics beyond structural checks.
///

#[derive(Debug)]
pub(super) struct ValidatedArgs {
    pub forwarded: Vec<TokenStream2>,
    pub export_name: Option<LitStr>,
    pub payload_max_bytes: Option<TokenStream2>,
    pub requires: Vec<AccessExprAst>,
    pub internal: bool,
    pub query_mode: QueryMode,
}

pub(super) fn validate(
    kind: EndpointKind,
    parsed: ParsedArgs,
    sig: &Signature,
    asyncness: bool,
) -> syn::Result<ValidatedArgs> {
    let requires_access = !parsed.requires.is_empty();

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

    if requires_access && !asyncness {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "this endpoint requires `async fn` due to access predicates",
        ));
    }

    if requires_access && !returns_fallible(sig) {
        return Err(syn::Error::new_spanned(
            &sig.output,
            "this endpoint must return `Result<_, E>` where `E: From<canic::Error>`",
        ));
    }

    if parsed.public && !parsed.requires.is_empty() {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "public endpoints must not also declare requires(...)",
        ));
    }

    if !parsed.public && parsed.requires.is_empty() {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "endpoint access must be explicit: add requires(...) or public",
        ));
    }

    if contains_negated_auth_or_caller_predicate(&parsed.requires) {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "not(...) must not wrap caller::*, auth::* or deployment::is_service_authority(...) predicates",
        ));
    }

    let requires_authenticated = requires_authenticated(&parsed.requires);
    let requires_attested_local_subnet = requires_attested_local_subnet(&parsed.requires);
    if requires_authenticated && requires_attested_local_subnet {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "authenticated(...) and attested_local_subnet() cannot share one first proof argument",
        ));
    }
    if requires_authenticated {
        validate_authenticated_args(sig)?;
    }
    if requires_attested_local_subnet {
        validate_attested_local_subnet_args(sig)?;
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

fn requires_attested_local_subnet(requires: &[AccessExprAst]) -> bool {
    requires
        .iter()
        .any(access_expr_contains_attested_local_subnet)
}

fn access_expr_contains_attested_local_subnet(expr: &AccessExprAst) -> bool {
    match expr {
        AccessExprAst::All(exprs) | AccessExprAst::Any(exprs) => {
            exprs.iter().any(access_expr_contains_attested_local_subnet)
        }
        AccessExprAst::Not(expr) => access_expr_contains_attested_local_subnet(expr),
        AccessExprAst::Pred(AccessPredicateAst::Builtin(BuiltinPredicate::AttestedLocalSubnet)) => {
            true
        }
        AccessExprAst::Pred(AccessPredicateAst::Builtin(_) | AccessPredicateAst::Custom(_)) => {
            false
        }
    }
}

fn contains_negated_auth_or_caller_predicate(requires: &[AccessExprAst]) -> bool {
    requires.iter().any(access_expr_contains_negated_identity)
}

fn access_expr_contains_negated_identity(expr: &AccessExprAst) -> bool {
    match expr {
        AccessExprAst::All(exprs) | AccessExprAst::Any(exprs) => {
            exprs.iter().any(access_expr_contains_negated_identity)
        }
        AccessExprAst::Not(expr) => access_expr_contains_identity_predicate(expr),
        AccessExprAst::Pred(_) => false,
    }
}

fn access_expr_contains_identity_predicate(expr: &AccessExprAst) -> bool {
    match expr {
        AccessExprAst::All(exprs) | AccessExprAst::Any(exprs) => {
            exprs.iter().any(access_expr_contains_identity_predicate)
        }
        AccessExprAst::Not(expr) => access_expr_contains_identity_predicate(expr),
        AccessExprAst::Pred(AccessPredicateAst::Builtin(builtin)) => {
            matches!(
                builtin,
                BuiltinPredicate::CallerIsController
                    | BuiltinPredicate::CallerIsParent
                    | BuiltinPredicate::CallerIsChild
                    | BuiltinPredicate::CallerIsRoot
                    | BuiltinPredicate::CallerIsSameCanister
                    | BuiltinPredicate::CallerIsWhitelisted
                    | BuiltinPredicate::Authenticated { .. }
                    | BuiltinPredicate::AttestedLocalSubnet
                    | BuiltinPredicate::ServiceAuthority { .. }
            )
        }
        AccessExprAst::Pred(AccessPredicateAst::Custom(_)) => false,
    }
}

fn validate_authenticated_args(sig: &Signature) -> syn::Result<()> {
    validate_first_arg_type(sig, "DelegatedToken", authenticated_arg_error())
}

fn validate_attested_local_subnet_args(sig: &Signature) -> syn::Result<()> {
    validate_first_arg_type(
        sig,
        "SignedRoleAttestation",
        "attested_local_subnet() requires a first argument of type `SignedRoleAttestation`",
    )
}

fn validate_first_arg_type(
    sig: &Signature,
    expected_type: &str,
    error: &'static str,
) -> syn::Result<()> {
    let Some(first) = sig.inputs.first() else {
        return Err(syn::Error::new_spanned(&sig.ident, error));
    };

    let first_ty = match first {
        FnArg::Typed(pat) => pat.ty.as_ref(),
        FnArg::Receiver(recv) => {
            return Err(syn::Error::new_spanned(recv, error));
        }
    };

    let Some(ident) = type_ident(first_ty) else {
        return Err(syn::Error::new_spanned(first_ty, error));
    };

    if ident == expected_type {
        return Ok(());
    }

    Err(syn::Error::new_spanned(first_ty, error))
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
