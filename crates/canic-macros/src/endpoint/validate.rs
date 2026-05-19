use crate::endpoint::{
    EndpointKind,
    parse::{AccessExprAst, AccessPredicateAst, BuiltinPredicate, ParsedArgs},
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

    if contains_attested_caller_role_predicate(&parsed.requires)
        && !matches!(kind, EndpointKind::Update)
    {
        return Err(syn::Error::new_spanned(
            &sig.ident,
            "caller::has_role(...) protected internal endpoints are update-only in 0.40 V1",
        ));
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

fn contains_attested_caller_role_predicate(requires: &[AccessExprAst]) -> bool {
    requires
        .iter()
        .any(access_expr_contains_attested_caller_role_predicate)
}

fn access_expr_contains_attested_caller_role_predicate(expr: &AccessExprAst) -> bool {
    match expr {
        AccessExprAst::All(exprs) | AccessExprAst::Any(exprs) => exprs
            .iter()
            .any(access_expr_contains_attested_caller_role_predicate),
        AccessExprAst::Not(expr) => access_expr_contains_attested_caller_role_predicate(expr),
        AccessExprAst::Pred(AccessPredicateAst::Builtin(
            BuiltinPredicate::CallerHasRole { .. } | BuiltinPredicate::CallerHasAnyRole { .. },
        )) => true,
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
        AccessExprAst::Pred(AccessPredicateAst::Builtin(builtin)) => matches!(
            builtin,
            BuiltinPredicate::CallerHasAppRole { .. }
                | BuiltinPredicate::CallerHasRole { .. }
                | BuiltinPredicate::CallerHasAnyRole { .. }
                | BuiltinPredicate::CallerIsRegisteredToSubnet
        ),
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
mod tests {
    use super::*;
    use crate::endpoint::parse::{AccessExprAst, AccessPredicateAst, BuiltinPredicate, ParsedArgs};

    fn parsed_authenticated() -> ParsedArgs {
        ParsedArgs {
            forwarded: Vec::new(),
            export_name: None,
            payload_max_bytes: None,
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
            export_name: None,
            payload_max_bytes: None,
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

    fn parsed_app_role(internal: bool) -> ParsedArgs {
        ParsedArgs {
            forwarded: Vec::new(),
            export_name: None,
            payload_max_bytes: None,
            requires: vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
                BuiltinPredicate::CallerHasAppRole {
                    role: crate::endpoint::parse::CanisterRoleArg::Literal(
                        "project_hub".to_string(),
                    ),
                },
            ))],
            requires_async: true,
            requires_fallible: true,
            internal,
        }
    }

    fn parsed_attested_role(internal: bool) -> ParsedArgs {
        ParsedArgs {
            forwarded: Vec::new(),
            export_name: None,
            payload_max_bytes: None,
            requires: vec![AccessExprAst::Pred(AccessPredicateAst::Builtin(
                BuiltinPredicate::CallerHasRole {
                    role: crate::endpoint::parse::CanisterRoleArg::Literal(
                        "project_hub".to_string(),
                    ),
                },
            ))],
            requires_async: true,
            requires_fallible: true,
            internal,
        }
    }

    #[test]
    fn authenticated_requires_first_argument() {
        let sig: Signature = syn::parse_quote!(async fn hello() -> Result<(), ::canic::Error>);
        let err = validate(EndpointKind::Update, parsed_authenticated(), &sig, true).unwrap_err();
        assert!(
            err.to_string()
                .contains("authenticated(...) requires a first argument")
        );
    }

    #[test]
    fn authenticated_accepts_delegated_token_first_arg() {
        let sig: Signature = syn::parse_quote!(
            async fn hello(token: ::canic::dto::auth::DelegatedToken) -> Result<(), ::canic::Error>
        );
        validate(EndpointKind::Update, parsed_authenticated(), &sig, true)
            .expect("authenticated arg ok");
    }

    #[test]
    fn authenticated_rejects_wrong_first_arg_type() {
        let sig: Signature = syn::parse_quote!(
            async fn hello(user: ::canic::cdk::candid::Principal) -> Result<(), ::canic::Error>
        );
        let err = validate(EndpointKind::Update, parsed_authenticated(), &sig, true).unwrap_err();
        assert!(
            err.to_string()
                .contains("authenticated(...) requires a first argument")
        );
    }

    #[test]
    fn registered_to_subnet_requires_internal_endpoint() {
        let sig: Signature = syn::parse_quote!(async fn hello() -> Result<(), ::canic::Error>);
        let err = validate(
            EndpointKind::Update,
            parsed_registered_to_subnet(false),
            &sig,
            true,
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("caller topology predicates are internal-only")
        );
    }

    #[test]
    fn registered_to_subnet_is_allowed_for_internal_endpoint() {
        let sig: Signature = syn::parse_quote!(async fn hello() -> Result<(), ::canic::Error>);
        validate(
            EndpointKind::Update,
            parsed_registered_to_subnet(true),
            &sig,
            true,
        )
        .expect("internal predicate ok");
    }

    #[test]
    fn app_role_requires_internal_endpoint() {
        let sig: Signature = syn::parse_quote!(async fn hello() -> Result<(), ::canic::Error>);
        let err = validate(EndpointKind::Update, parsed_app_role(false), &sig, true).unwrap_err();
        assert!(
            err.to_string()
                .contains("caller topology predicates are internal-only")
        );
    }

    #[test]
    fn app_role_is_allowed_for_internal_endpoint() {
        let sig: Signature = syn::parse_quote!(async fn hello() -> Result<(), ::canic::Error>);
        validate(EndpointKind::Update, parsed_app_role(true), &sig, true)
            .expect("internal app canister predicate ok");
    }

    #[test]
    fn attested_role_requires_internal_update_endpoint() {
        let sig: Signature = syn::parse_quote!(async fn hello() -> Result<(), ::canic::Error>);
        let err = validate(
            EndpointKind::Update,
            parsed_attested_role(false),
            &sig,
            true,
        )
        .expect_err("attested role must be internal");
        assert!(
            err.to_string()
                .contains("caller topology predicates are internal-only")
        );

        let err = validate(EndpointKind::Query, parsed_attested_role(true), &sig, true)
            .expect_err("attested query must fail");
        assert!(
            err.to_string()
                .contains("protected internal endpoints are update-only")
        );
    }

    #[test]
    fn attested_role_is_allowed_for_internal_update_endpoint() {
        let sig: Signature = syn::parse_quote!(async fn hello() -> Result<(), ::canic::Error>);
        validate(EndpointKind::Update, parsed_attested_role(true), &sig, true)
            .expect("internal update attested role predicate ok");
    }

    #[test]
    fn payload_limit_is_update_only() {
        let sig: Signature = syn::parse_quote!(fn hello() -> bool);
        let parsed = ParsedArgs {
            forwarded: Vec::new(),
            export_name: None,
            payload_max_bytes: Some(quote::quote!(1024)),
            requires: Vec::new(),
            requires_async: false,
            requires_fallible: false,
            internal: false,
        };

        let err = validate(EndpointKind::Query, parsed, &sig, false).unwrap_err();
        assert!(
            err.to_string()
                .contains("payload(...) is supported only on canic_update")
        );
    }
}
