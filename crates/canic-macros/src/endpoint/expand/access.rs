//
// ============================================================================
// ACCESS PIPELINE & METRICS INVARIANTS
// ============================================================================
//
// This module generates the access-control wrapper fragments for canister
// endpoints. The code below is SECURITY-SENSITIVE.
//
// The following invariants are intentional and MUST be preserved:
//
// 1. Access pipeline semantics
//    --------------------------
//    Access checks are evaluated via `access::expr::eval_access`.
//    `requires(...)` always lowers to a single AccessExpr::All list.
//
//    Evaluation short-circuits on the FIRST failure.
//
// 2. Access metrics (denial-only)
//    -----------------------------
//    Access metrics are emitted ONLY on access denial paths.
//    Each denied request MUST emit EXACTLY ONE access metric via the
//    expression evaluator, tagged with the predicate kind that denied access.
//
//    Successful requests MUST emit NO access metrics.
//
//    These invariants are relied upon by access metrics aggregation logic.
//
// 3. Error handling
//    --------------
//    Access failures for gated endpoints must return a Result error; trapping
//    is forbidden outside lifecycle adapters. Infallible endpoints that can
//    deny access are rejected at compile time.
//
// 4. Macro constraints
//    ------------------
//    - requires(...) accepts only expression calls (all/any/not/custom + built-ins).
//    - `self` receivers are forbidden.
//    - Fallibility detection assumes a direct `Result<_, _>` return type.
//
// Any change to this file should be reviewed against ALL of the above
// invariants. Violating them will silently corrupt access metrics or
// authorization behavior.
//

use crate::endpoint::{
    EndpointKind,
    parse::{AccessExprAst, AccessPredicateAst, AuthScopeArg, BuiltinPredicate},
    validate::ValidatedArgs,
};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{GenericArgument, PathArguments, Signature, Type, visit::Visit};

pub(super) fn access_stage(plan: &AccessPlan, call: &syn::Ident) -> TokenStream2 {
    let caller = format_ident!("__canic_caller");
    let authenticated_identity = format_ident!("__canic_authenticated_identity");
    let ctx = format_ident!("__canic_access_ctx");

    let deny = quote!(return Err(err.into()););

    match plan {
        AccessPlan::None => quote!(),
        AccessPlan::DefaultFleet(guard) => {
            let guard_expr = guard_tokens(*guard);
            quote! {
                let #caller = ::canic::__internal::cdk::api::msg_caller();
                let #ctx = ::canic::__internal::core::access::expr::AccessContext {
                    caller: #caller,
                    authenticated_caller: #caller,
                    identity_source: ::canic::__internal::core::access::auth::AuthenticatedIdentitySource::RawCaller,
                    call: #call,
                };
                if let Err(err) = ::canic::__internal::core::access::expr::eval_default_fleet_guard(
                    #guard_expr,
                    &#ctx,
                ) {
                    #deny
                }
            }
        }
        AccessPlan::Expr(expr) => {
            let expr_ident = format_ident!("__canic_access_expr");
            quote! {
                let #caller = ::canic::__internal::cdk::api::msg_caller();
                let #authenticated_identity =
                    ::canic::__internal::core::access::auth::resolve_authenticated_identity(#caller);
                let #ctx = ::canic::__internal::core::access::expr::AccessContext {
                    caller: #authenticated_identity.transport_caller,
                    authenticated_caller: #authenticated_identity.authenticated_subject,
                    identity_source: #authenticated_identity.identity_source,
                    call: #call,
                };
                let #expr_ident = #expr;
                if let Err(err) = ::canic::__internal::core::access::expr::eval_access(&#expr_ident, &#ctx).await {
                    #deny
                }
            }
        }
    }
}

///
/// DefaultFleetGuard
///

#[derive(Clone, Copy, Debug)]
pub(super) enum DefaultFleetGuard {
    AllowsUpdates,
    IsQueryable,
}

///
/// AccessPlan
///

#[derive(Debug)]
pub(super) enum AccessPlan {
    None,
    DefaultFleet(DefaultFleetGuard),
    Expr(TokenStream2),
}

impl AccessPlan {
    pub(super) const fn requires_async(&self) -> bool {
        matches!(self, Self::Expr(_))
    }
}

fn guard_tokens(guard: DefaultFleetGuard) -> TokenStream2 {
    match guard {
        DefaultFleetGuard::AllowsUpdates => {
            quote!(::canic::__internal::core::access::expr::DefaultFleetGuard::AllowsUpdates)
        }
        DefaultFleetGuard::IsQueryable => {
            quote!(::canic::__internal::core::access::expr::DefaultFleetGuard::IsQueryable)
        }
    }
}

pub(super) fn build_access_plan(
    kind: EndpointKind,
    args: &ValidatedArgs,
    sig: &Signature,
) -> syn::Result<AccessPlan> {
    let is_fleet_command = is_fleet_command_endpoint(sig);
    let is_internal = args.internal || is_fleet_command;
    let has_fleet_state = exprs_have_fleet_state_predicate(&args.requires);

    if is_internal && has_fleet_state {
        let message = if is_fleet_command {
            "FleetCommand endpoints must never be gated on Fleet state."
        } else {
            "Internal protocol endpoints must never be gated on Fleet state."
        };
        return Err(syn::Error::new_spanned(&sig.ident, message));
    }

    let mut exprs = args.requires.clone();

    if !is_internal && !has_fleet_state {
        if exprs.is_empty() {
            let default_guard = match kind {
                EndpointKind::Update => DefaultFleetGuard::AllowsUpdates,
                EndpointKind::Query => DefaultFleetGuard::IsQueryable,
            };
            return Ok(AccessPlan::DefaultFleet(default_guard));
        }

        let injected = match kind {
            EndpointKind::Update => BuiltinPredicate::FleetAllowsUpdates,
            EndpointKind::Query => BuiltinPredicate::FleetIsQueryable,
        };
        exprs.push(AccessExprAst::Pred(AccessPredicateAst::Builtin(injected)));
    }

    if exprs.is_empty() {
        return Ok(AccessPlan::None);
    }

    let exprs: Vec<_> = exprs.iter().map(expr_from_ast).collect();

    Ok(AccessPlan::Expr(quote! {
        ::canic::__internal::core::access::expr::AccessExpr::All(vec![#(#exprs),*])
    }))
}

fn expr_from_ast(expr: &AccessExprAst) -> TokenStream2 {
    match expr {
        AccessExprAst::All(exprs) => {
            let items = exprs.iter().map(expr_from_ast);
            quote!(::canic::__internal::core::access::expr::AccessExpr::All(
                vec![#(#items),*]
            ))
        }
        AccessExprAst::Any(exprs) => {
            let items = exprs.iter().map(expr_from_ast);
            quote!(::canic::__internal::core::access::expr::AccessExpr::Any(
                vec![#(#items),*]
            ))
        }
        AccessExprAst::Not(expr) => {
            let inner = expr_from_ast(expr);
            quote!(::canic::__internal::core::access::expr::AccessExpr::Not(Box::new(#inner)))
        }
        AccessExprAst::Pred(pred) => match pred {
            AccessPredicateAst::Builtin(builtin) => expr_from_builtin(builtin),
            AccessPredicateAst::Custom(expr) => {
                quote!(::canic::__internal::core::access::expr::custom(#expr))
            }
        },
    }
}

fn expr_from_builtin(pred: &BuiltinPredicate) -> TokenStream2 {
    match pred {
        BuiltinPredicate::FleetAllowsUpdates => {
            quote!(::canic::__internal::core::access::expr::fleet::allows_updates())
        }
        BuiltinPredicate::FleetIsQueryable => {
            quote!(::canic::__internal::core::access::expr::fleet::is_queryable())
        }
        BuiltinPredicate::SelfIsPrimeSubnet => {
            quote!(::canic::__internal::core::access::expr::env::is_prime_subnet())
        }
        BuiltinPredicate::SelfIsPrimeRoot => {
            quote!(::canic::__internal::core::access::expr::env::is_prime_root())
        }
        BuiltinPredicate::CallerIsController => {
            quote!(::canic::__internal::core::access::expr::caller::is_controller())
        }
        BuiltinPredicate::CallerIsParent => {
            quote!(::canic::__internal::core::access::expr::caller::is_parent())
        }
        BuiltinPredicate::CallerIsChild => {
            quote!(::canic::__internal::core::access::expr::caller::is_child())
        }
        BuiltinPredicate::CallerIsRoot => {
            quote!(::canic::__internal::core::access::expr::caller::is_root())
        }
        BuiltinPredicate::CallerIsSameCanister => {
            quote!(::canic::__internal::core::access::expr::caller::is_same_canister())
        }
        BuiltinPredicate::CallerIsRegisteredToSubnet => {
            quote!(::canic::__internal::core::access::expr::caller::is_registered_to_subnet())
        }
        BuiltinPredicate::CallerIsWhitelisted => {
            quote!(::canic::__internal::core::access::expr::caller::is_whitelisted())
        }
        BuiltinPredicate::Authenticated { required_scope } => match required_scope {
            Some(AuthScopeArg::Literal(required_scope)) => quote!(
                ::canic::__internal::core::access::expr::auth::authenticated_with_scope(
                    #required_scope
                )
            ),
            Some(AuthScopeArg::Expr(required_scope)) => quote!(
                ::canic::__internal::core::access::expr::auth::authenticated_with_scope(
                    #required_scope
                )
            ),
            None => quote!(
                ::canic::__internal::core::access::expr::auth::authenticated(
                    ::core::option::Option::None
                )
            ),
        },
        BuiltinPredicate::BuildIcOnly => {
            quote!(::canic::__internal::core::access::expr::env::build_ic_only())
        }
        BuiltinPredicate::BuildLocalOnly => {
            quote!(::canic::__internal::core::access::expr::env::build_local_only())
        }
    }
}

fn exprs_have_fleet_state_predicate(exprs: &[AccessExprAst]) -> bool {
    exprs.iter().any(expr_has_fleet_state_predicate)
}

pub(super) fn requires_authenticated(exprs: &[AccessExprAst]) -> bool {
    exprs.iter().any(expr_has_authenticated_predicate)
}

fn expr_has_authenticated_predicate(expr: &AccessExprAst) -> bool {
    match expr {
        AccessExprAst::All(exprs) | AccessExprAst::Any(exprs) => {
            exprs.iter().any(expr_has_authenticated_predicate)
        }
        AccessExprAst::Not(expr) => expr_has_authenticated_predicate(expr),
        AccessExprAst::Pred(pred) => match pred {
            AccessPredicateAst::Builtin(builtin) => {
                matches!(builtin, BuiltinPredicate::Authenticated { .. })
            }
            AccessPredicateAst::Custom(_) => false,
        },
    }
}

fn expr_has_fleet_state_predicate(expr: &AccessExprAst) -> bool {
    match expr {
        AccessExprAst::All(exprs) | AccessExprAst::Any(exprs) => {
            exprs.iter().any(expr_has_fleet_state_predicate)
        }
        AccessExprAst::Not(expr) => expr_has_fleet_state_predicate(expr),
        AccessExprAst::Pred(pred) => match pred {
            AccessPredicateAst::Builtin(builtin) => builtin_is_fleet_state(builtin),
            AccessPredicateAst::Custom(tokens) => custom_has_fleet_state_is(tokens),
        },
    }
}

const fn builtin_is_fleet_state(pred: &BuiltinPredicate) -> bool {
    matches!(
        pred,
        BuiltinPredicate::FleetAllowsUpdates | BuiltinPredicate::FleetIsQueryable
    )
}

fn custom_has_fleet_state_is(tokens: &TokenStream2) -> bool {
    let Ok(expr) = syn::parse2::<syn::Expr>(tokens.clone()) else {
        return false;
    };
    let mut visitor = FleetStateVisitor { found: false };
    visitor.visit_expr(&expr);
    visitor.found
}

///
/// FleetStateVisitor
///

struct FleetStateVisitor {
    found: bool,
}

impl Visit<'_> for FleetStateVisitor {
    fn visit_path(&mut self, path: &syn::Path) {
        if path.segments.iter().any(|seg| seg.ident == "FleetStateIs") {
            self.found = true;
            return;
        }
        syn::visit::visit_path(self, path);
    }
}

fn is_fleet_command_endpoint(sig: &Signature) -> bool {
    sig.inputs.iter().any(|input| match input {
        syn::FnArg::Typed(pat) => type_has_fleet_command(&pat.ty),
        syn::FnArg::Receiver(_) => true,
    })
}

fn type_has_fleet_command(ty: &Type) -> bool {
    match ty {
        Type::Path(path) => path_has_fleet_command(&path.path),
        Type::Reference(reference) => type_has_fleet_command(&reference.elem),
        Type::Group(group) => type_has_fleet_command(&group.elem),
        Type::Paren(paren) => type_has_fleet_command(&paren.elem),
        Type::Tuple(tuple) => tuple.elems.iter().any(type_has_fleet_command),
        _ => false,
    }
}

fn path_has_fleet_command(path: &syn::Path) -> bool {
    path.segments.iter().any(|seg| {
        if seg.ident == "FleetCommand" {
            return true;
        }

        match &seg.arguments {
            PathArguments::AngleBracketed(args) => args.args.iter().any(|arg| match arg {
                GenericArgument::Type(ty) => type_has_fleet_command(ty),
                _ => false,
            }),
            _ => false,
        }
    })
}
